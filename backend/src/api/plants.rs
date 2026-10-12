use crate::datasources::openrouter::PlantPlanRequest;
use crate::db::{plant_queries, queries};
use crate::error::{Source, TurfOpsError};
use crate::models::plant::{Plant, PlantType};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::str::FromStr;

// @spec PLANT-API-001
pub async fn list_plants(State(state): State<AppState>) -> Result<Json<Vec<Plant>>, TurfOpsError> {
    let profile_id = active_profile_id(&state).await?;
    let plants = plant_queries::list_plants_for_profile(&state.pool, profile_id).await?;
    Ok(Json(plants))
}

// @spec PLANT-API-008
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

// @spec PLANT-API-002, PLANT-API-005, PLANT-API-006, PLANT-API-007, PLANT-AI-006
pub async fn create_plant(
    State(state): State<AppState>,
    Json(req): Json<CreatePlantRequest>,
) -> Result<(StatusCode, Json<Plant>), TurfOpsError> {
    // @spec PLANT-API-004: a bad request is a 400 whether or not the assistant is set up.
    let NewPlant {
        plant_type,
        input,
        planting_date,
    } = validate_new_plant(&req)?;
    let openrouter = state
        .openrouter
        .as_ref()
        .ok_or_else(|| TurfOpsError::not_configured(Source::PlanAssistant))?
        .clone();

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

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

/// A partial edit. For the nullable fields, absent (`None`) keeps the value and JSON `null`
/// (`Some(None)`) clears it.
#[derive(Debug, Deserialize)]
pub struct UpdatePlantRequest {
    pub common_name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub scientific_name: Option<Option<String>>,
    pub plant_type: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub location: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub planting_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    pub notes: Option<Option<String>>,
}

/// A field that is in the body, whatever its value: `null` becomes `Some(None)`. Serde
/// calls this only for keys that are present; `#[serde(default)]` covers absent ones.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

// @spec PLANT-API-010, PLANT-API-009
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

// @spec PLANT-API-011
pub async fn delete_plant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    // @spec PLANT-API-013
    deleted(plant_queries::delete_plant(&state.pool, id).await?, id)?;
    Ok(StatusCode::NO_CONTENT)
}

// @spec PLANT-API-007, PLANT-API-012, PLANT-AI-006
pub async fn refresh_plant_plan(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Plant>, TurfOpsError> {
    let openrouter = state
        .openrouter
        .as_ref()
        .ok_or_else(|| TurfOpsError::not_configured(Source::PlanAssistant))?
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

/// What a new plant's request must carry, checked before anything else.
pub(crate) struct NewPlant {
    pub plant_type: PlantType,
    pub input: String,
    pub planting_date: Option<NaiveDate>,
}

// @spec PLANT-API-003
pub(crate) fn validate_new_plant(req: &CreatePlantRequest) -> Result<NewPlant, TurfOpsError> {
    let plant_type = PlantType::from_str(&req.plant_type).map_err(|_| {
        TurfOpsError::InvalidData(format!(
            "\"{}\" isn't a plant type. Pick one from the list.",
            req.plant_type
        ))
    })?;
    let input = match (filled(&req.common_name), filled(&req.scientific_name)) {
        (Some(c), Some(s)) => format!("{c} ({s})"),
        (Some(c), None) => c.to_string(),
        (None, Some(s)) => s.to_string(),
        (None, None) => {
            return Err(TurfOpsError::InvalidData(
                "Enter the plant's common name or its scientific name.".into(),
            ))
        }
    };
    let planting_date = match filled(&req.planting_date) {
        Some(s) => Some(NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
            TurfOpsError::InvalidData(format!(
                "The planting date \"{s}\" isn't a valid date. Pick it from the calendar."
            ))
        })?),
        None => None,
    };
    Ok(NewPlant {
        plant_type,
        input,
        planting_date,
    })
}

/// A trimmed, non-empty value.
fn filled(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// 404 when a delete touched no row.
pub(crate) fn deleted(rows_affected: u64, id: i64) -> Result<(), TurfOpsError> {
    if rows_affected == 0 {
        return Err(TurfOpsError::NotFound(format!("Plant {id} not found")));
    }
    Ok(())
}

async fn active_profile_id(state: &AppState) -> Result<i64, TurfOpsError> {
    queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))
}

#[cfg(all(test, feature = "db-tests"))]
mod db_crud_tests;
#[cfg(all(test, feature = "db-tests"))]
mod db_support;
#[cfg(all(test, feature = "db-tests"))]
mod db_tests;
#[cfg(test)]
mod tests;
