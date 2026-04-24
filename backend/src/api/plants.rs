use crate::datasources::openrouter::PlantPlanRequest;
use crate::db::{plant_queries, queries};
use crate::error::TurfOpsError;
use crate::models::plant::{Plant, PlantType};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::str::FromStr;

pub async fn list_plants(State(state): State<AppState>) -> Result<Json<Vec<Plant>>, TurfOpsError> {
    let profile_id = active_profile_id(&state).await?;
    let plants = plant_queries::list_plants_for_profile(&state.pool, profile_id).await?;
    Ok(Json(plants))
}

pub async fn get_plant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Plant>, TurfOpsError> {
    let plant = plant_queries::get_plant(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Plant {} not found", id)))?;
    Ok(Json(plant))
}

#[derive(Debug, Deserialize)]
pub struct CreatePlantRequest {
    pub common_name: Option<String>,
    pub scientific_name: Option<String>,
    pub plant_type: String,
    pub location: Option<String>,
    pub planting_date: Option<String>,
    pub notes: Option<String>,
}

pub async fn create_plant(
    State(state): State<AppState>,
    Json(req): Json<CreatePlantRequest>,
) -> Result<(StatusCode, Json<Plant>), TurfOpsError> {
    let openrouter = state
        .openrouter
        .as_ref()
        .ok_or_else(|| {
            TurfOpsError::DataSourceUnavailable(
                "OpenRouter not configured — set OPENROUTER_API_KEY to add plants".into(),
            )
        })?
        .clone();

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let plant_type = PlantType::from_str(&req.plant_type).map_err(|_| {
        TurfOpsError::InvalidData(format!("Unknown plant type: {}", req.plant_type))
    })?;

    let input = match (req.common_name.as_deref(), req.scientific_name.as_deref()) {
        (Some(c), Some(s)) if !c.is_empty() && !s.is_empty() => format!("{} ({})", c, s),
        (Some(c), _) if !c.is_empty() => c.to_string(),
        (_, Some(s)) if !s.is_empty() => s.to_string(),
        _ => {
            return Err(TurfOpsError::InvalidData(
                "Must provide common_name or scientific_name".into(),
            ))
        }
    };

    let planting_date = match req.planting_date.as_deref() {
        Some(s) if !s.is_empty() => Some(
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| TurfOpsError::InvalidData(format!("Invalid planting_date: {}", s)))?,
        ),
        _ => None,
    };

    let plan = openrouter
        .generate_plant_plan(PlantPlanRequest {
            input: &input,
            usda_zone: &profile.usda_zone,
            plant_type,
            location: req.location.as_deref(),
        })
        .await?;

    let now = Utc::now();
    let plant = Plant {
        id: None,
        lawn_profile_id: profile_id,
        common_name: req
            .common_name
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| plan.identified_name.clone()),
        scientific_name: req
            .scientific_name
            .filter(|s| !s.is_empty())
            .or_else(|| plan.scientific_name.clone()),
        plant_type,
        location: req.location,
        planting_date,
        notes: req.notes,
        maintenance_plan: plan,
        plan_generated_at: now,
        plan_model: openrouter.model().to_string(),
        created_at: now,
        updated_at: now,
    };

    let id = plant_queries::create_plant(&state.pool, &plant).await?;
    let created = plant_queries::get_plant(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::InvalidData("Plant not found after insert".into()))?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlantRequest {
    pub common_name: Option<String>,
    pub scientific_name: Option<Option<String>>,
    pub plant_type: Option<String>,
    pub location: Option<Option<String>>,
    pub planting_date: Option<Option<String>>,
    pub notes: Option<Option<String>>,
}

pub async fn update_plant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePlantRequest>,
) -> Result<Json<Plant>, TurfOpsError> {
    let plant_type = match req.plant_type.as_deref() {
        Some(s) => Some(
            PlantType::from_str(s)
                .map_err(|_| TurfOpsError::InvalidData(format!("Unknown plant type: {}", s)))?,
        ),
        None => None,
    };

    let planting_date = match req.planting_date {
        Some(Some(s)) => Some(Some(NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(
            |_| TurfOpsError::InvalidData(format!("Invalid planting_date: {}", s)),
        )?)),
        Some(None) => Some(None),
        None => None,
    };

    let update = plant_queries::PlantMetadataUpdate {
        common_name: req.common_name,
        scientific_name: req.scientific_name,
        plant_type,
        location: req.location,
        planting_date,
        notes: req.notes,
    };

    plant_queries::update_plant_metadata(&state.pool, id, &update).await?;

    let updated = plant_queries::get_plant(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Plant {} not found", id)))?;
    Ok(Json(updated))
}

pub async fn delete_plant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    plant_queries::delete_plant(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn refresh_plant_plan(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Plant>, TurfOpsError> {
    let openrouter = state
        .openrouter
        .as_ref()
        .ok_or_else(|| {
            TurfOpsError::DataSourceUnavailable(
                "OpenRouter not configured — cannot refresh plant plan".into(),
            )
        })?
        .clone();

    let plant = plant_queries::get_plant(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Plant {} not found", id)))?;

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let input = match plant.scientific_name.as_deref() {
        Some(s) if !s.is_empty() => format!("{} ({})", plant.common_name, s),
        _ => plant.common_name.clone(),
    };

    let plan = openrouter
        .generate_plant_plan(PlantPlanRequest {
            input: &input,
            usda_zone: &profile.usda_zone,
            plant_type: plant.plant_type,
            location: plant.location.as_deref(),
        })
        .await?;

    plant_queries::update_plant_plan(&state.pool, id, &plan, openrouter.model()).await?;

    let updated = plant_queries::get_plant(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Plant {} not found", id)))?;
    Ok(Json(updated))
}

async fn active_profile_id(state: &AppState) -> Result<i64, TurfOpsError> {
    queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))
}
