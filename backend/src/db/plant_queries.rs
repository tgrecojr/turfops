use crate::error::{Result, TurfOpsError};
use crate::models::plant::{Plant, PlantMaintenancePlan, PlantType};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::warn;

pub async fn list_plants_for_profile(pool: &PgPool, profile_id: i64) -> Result<Vec<Plant>> {
    let rows = sqlx::query_as::<_, PlantRow>(
        r#"SELECT id, lawn_profile_id, common_name, scientific_name, plant_type, location,
           planting_date, notes, maintenance_plan, plan_generated_at, plan_model,
           created_at, updated_at
           FROM plants WHERE lawn_profile_id = $1 ORDER BY created_at DESC"#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(PlantRow::into_plant).collect()
}

pub async fn get_plant(pool: &PgPool, id: i64) -> Result<Option<Plant>> {
    let row = sqlx::query_as::<_, PlantRow>(
        r#"SELECT id, lawn_profile_id, common_name, scientific_name, plant_type, location,
           planting_date, notes, maintenance_plan, plan_generated_at, plan_model,
           created_at, updated_at
           FROM plants WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(PlantRow::into_plant).transpose()
}

pub async fn create_plant(pool: &PgPool, plant: &Plant) -> Result<i64> {
    let plan_json = serde_json::to_value(&plant.maintenance_plan)
        .map_err(|e| TurfOpsError::InvalidData(format!("Plan serialization: {}", e)))?;

    let id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO plants
            (lawn_profile_id, common_name, scientific_name, plant_type, location,
             planting_date, notes, maintenance_plan, plan_generated_at, plan_model)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
    )
    .bind(plant.lawn_profile_id)
    .bind(&plant.common_name)
    .bind(&plant.scientific_name)
    .bind(plant.plant_type.as_str())
    .bind(&plant.location)
    .bind(plant.planting_date)
    .bind(&plant.notes)
    .bind(plan_json)
    .bind(plant.plan_generated_at)
    .bind(&plant.plan_model)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

pub struct PlantMetadataUpdate {
    pub common_name: Option<String>,
    pub scientific_name: Option<Option<String>>,
    pub plant_type: Option<PlantType>,
    pub location: Option<Option<String>>,
    pub planting_date: Option<Option<NaiveDate>>,
    pub notes: Option<Option<String>>,
}

pub async fn update_plant_metadata(
    pool: &PgPool,
    id: i64,
    update: &PlantMetadataUpdate,
) -> Result<()> {
    // Fetch existing row so we can do a partial update without rewriting the plan.
    let existing = get_plant(pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Plant {} not found", id)))?;

    let common_name = update
        .common_name
        .clone()
        .unwrap_or(existing.common_name.clone());
    let scientific_name = update
        .scientific_name
        .clone()
        .unwrap_or(existing.scientific_name.clone());
    let plant_type = update.plant_type.unwrap_or(existing.plant_type);
    let location = update.location.clone().unwrap_or(existing.location.clone());
    let planting_date = update.planting_date.unwrap_or(existing.planting_date);
    let notes = update.notes.clone().unwrap_or(existing.notes.clone());

    sqlx::query(
        r#"
        UPDATE plants SET
            common_name = $1, scientific_name = $2, plant_type = $3, location = $4,
            planting_date = $5, notes = $6, updated_at = NOW()
        WHERE id = $7
        "#,
    )
    .bind(common_name)
    .bind(scientific_name)
    .bind(plant_type.as_str())
    .bind(location)
    .bind(planting_date)
    .bind(notes)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_plant_plan(
    pool: &PgPool,
    id: i64,
    plan: &PlantMaintenancePlan,
    model: &str,
) -> Result<()> {
    let plan_json = serde_json::to_value(plan)
        .map_err(|e| TurfOpsError::InvalidData(format!("Plan serialization: {}", e)))?;

    sqlx::query(
        r#"
        UPDATE plants SET
            maintenance_plan = $1, plan_generated_at = NOW(), plan_model = $2, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(plan_json)
    .bind(model)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_plant(pool: &PgPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM plants WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct PlantRow {
    id: i64,
    lawn_profile_id: i64,
    common_name: String,
    scientific_name: Option<String>,
    plant_type: String,
    location: Option<String>,
    planting_date: Option<NaiveDate>,
    notes: Option<String>,
    maintenance_plan: serde_json::Value,
    plan_generated_at: DateTime<Utc>,
    plan_model: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PlantRow {
    fn into_plant(self) -> Result<Plant> {
        let plant_type = PlantType::from_str(&self.plant_type).unwrap_or_else(|_| {
            warn!(
                plant_type = %self.plant_type,
                "Unknown plant_type in database, defaulting to Other"
            );
            PlantType::Other
        });

        let maintenance_plan: PlantMaintenancePlan = serde_json::from_value(self.maintenance_plan)
            .map_err(|e| {
                TurfOpsError::InvalidData(format!(
                    "Plant {} has invalid maintenance_plan JSON: {}",
                    self.id, e
                ))
            })?;

        Ok(Plant {
            id: Some(self.id),
            lawn_profile_id: self.lawn_profile_id,
            common_name: self.common_name,
            scientific_name: self.scientific_name,
            plant_type,
            location: self.location,
            planting_date: self.planting_date,
            notes: self.notes,
            maintenance_plan,
            plan_generated_at: self.plan_generated_at,
            plan_model: self.plan_model,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
