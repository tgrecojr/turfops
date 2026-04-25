use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{Application, ApplicationScope, ApplicationType, WeatherSnapshot};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::str::FromStr;

const DEFAULT_PAGE_LIMIT: i64 = 50;
const MAX_PAGE_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
pub struct ListApplicationsQuery {
    #[serde(rename = "type")]
    pub app_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_applications(
    State(state): State<AppState>,
    Query(params): Query<ListApplicationsQuery>,
) -> Result<Json<Vec<Application>>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let limit = params
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let offset = params.offset.unwrap_or(0).max(0);

    let mut apps =
        queries::get_applications_for_profile(&state.pool, profile_id, limit, offset).await?;

    // Optional filter by application type
    if let Some(type_filter) = params.app_type {
        let app_type = ApplicationType::from_str(&type_filter).map_err(|_| {
            TurfOpsError::InvalidData(format!("Unknown application type filter: {}", type_filter))
        })?;
        apps.retain(|a| a.application_type == app_type);
    }

    Ok(Json(apps))
}

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub application_type: String,
    pub product_name: Option<String>,
    pub application_date: String,
    pub rate_per_1000sqft: Option<f64>,
    pub coverage_sqft: Option<f64>,
    pub notes: Option<String>,
    pub weather_snapshot: Option<WeatherSnapshot>,
    pub nitrogen_pct: Option<f64>,
    pub phosphorus_pct: Option<f64>,
    pub potassium_pct: Option<f64>,
    pub plant_id: Option<i64>,
    pub follow_up_date: Option<String>,
}

pub async fn create_application(
    State(state): State<AppState>,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<Application>), TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let application_type = ApplicationType::from_str(&req.application_type).map_err(|_| {
        TurfOpsError::InvalidData(format!(
            "Unknown application type: {}",
            req.application_type
        ))
    })?;

    let application_date =
        NaiveDate::parse_from_str(&req.application_date, "%Y-%m-%d").map_err(|_| {
            TurfOpsError::InvalidData(format!(
                "Invalid date format: {}. Expected YYYY-MM-DD",
                req.application_date
            ))
        })?;

    match application_type.scope() {
        ApplicationScope::PlantRequired if req.plant_id.is_none() => {
            return Err(TurfOpsError::InvalidData(format!(
                "Application type {} requires plant_id",
                application_type
            )));
        }
        ApplicationScope::TurfOnly if req.plant_id.is_some() => {
            return Err(TurfOpsError::InvalidData(format!(
                "Application type {} cannot be linked to a plant",
                application_type
            )));
        }
        _ => {}
    }

    let follow_up_date = req
        .follow_up_date
        .as_deref()
        .map(|s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                TurfOpsError::InvalidData(format!(
                    "Invalid follow_up_date format: {}. Expected YYYY-MM-DD",
                    s
                ))
            })
        })
        .transpose()?;

    if let Some(fu) = follow_up_date {
        if fu < application_date {
            return Err(TurfOpsError::InvalidData(
                "follow_up_date must be on or after application_date".into(),
            ));
        }
    }

    let app = Application {
        id: None,
        lawn_profile_id: profile
            .id
            .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?,
        application_type,
        product_name: req.product_name,
        application_date,
        rate_per_1000sqft: req.rate_per_1000sqft,
        coverage_sqft: req.coverage_sqft,
        notes: req.notes,
        weather_snapshot: req.weather_snapshot,
        nitrogen_pct: req.nitrogen_pct,
        phosphorus_pct: req.phosphorus_pct,
        potassium_pct: req.potassium_pct,
        plant_id: req.plant_id,
        follow_up_date,
        created_at: Utc::now(),
    };

    let id = queries::create_application(&state.pool, &app).await?;

    let created = Application {
        id: Some(id),
        ..app
    };

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn delete_application(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    queries::delete_application(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
