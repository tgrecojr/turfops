use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{GrassType, IrrigationType, LawnProfile, SoilType};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use std::str::FromStr;

#[cfg(all(test, feature = "db-tests"))]
mod db_tests;
#[cfg(test)]
mod route_tests;

// @spec APP-PROF-008
pub async fn get_profile(State(state): State<AppState>) -> Result<Json<LawnProfile>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    Ok(Json(profile))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub grass_type: Option<String>,
    pub usda_zone: Option<String>,
    pub soil_type: Option<String>,
    pub lawn_size_sqft: Option<f64>,
    pub irrigation_type: Option<String>,
}

/// Update the lawn profile, or create it when none exists yet (filling omitted fields
/// the way startup seeding does), so saving Settings always works.
// @spec APP-PROF-009, APP-PROF-010, APP-PROF-011, APP-PROF-012
pub async fn update_profile(
    State(state): State<AppState>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<LawnProfile>, TurfOpsError> {
    let existing = queries::get_default_lawn_profile(&state.pool).await?;
    let creating = existing.is_none();
    let mut profile = existing.unwrap_or_else(|| {
        LawnProfile::new("Main Lawn".into(), GrassType::TallFescue, "7a".into())
    });

    if let Some(name) = req.name {
        profile.name = name;
    }
    if let Some(gt) = req.grass_type {
        profile.grass_type = GrassType::from_str(&gt)
            .map_err(|_| TurfOpsError::InvalidData(format!("Unknown grass type: {}", gt)))?;
    }
    if let Some(zone) = req.usda_zone {
        profile.usda_zone = zone;
    }
    // Optional fields: absent = leave alone, empty (or a non-positive size) = clear.
    match req.soil_type.as_deref() {
        None => {}
        Some("") => profile.soil_type = None,
        Some(st) => {
            profile.soil_type =
                Some(SoilType::from_str(st).map_err(|_| {
                    TurfOpsError::InvalidData(format!("Unknown soil type: {}", st))
                })?);
        }
    }
    if let Some(sqft) = req.lawn_size_sqft {
        profile.lawn_size_sqft = Some(sqft).filter(|s| *s > 0.0);
    }
    match req.irrigation_type.as_deref() {
        None => {}
        Some("") => profile.irrigation_type = None,
        Some(it) => {
            profile.irrigation_type = Some(IrrigationType::from_str(it).map_err(|_| {
                TurfOpsError::InvalidData(format!("Unknown irrigation type: {}", it))
            })?);
        }
    }

    if creating {
        queries::create_lawn_profile(&state.pool, &profile).await?;
    } else {
        queries::update_lawn_profile(&state.pool, &profile).await?;
    }

    // Re-fetch to get the stored timestamps (and id, when created)
    let updated = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("Profile disappeared after update".into()))?;

    Ok(Json(updated))
}
