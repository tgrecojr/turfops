use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{Application, ApplicationType, WeatherSnapshot};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListApplicationsQuery {
    #[serde(rename = "type")]
    pub app_type: Option<String>,
}

pub async fn list_applications(
    State(state): State<AppState>,
    Query(params): Query<ListApplicationsQuery>,
) -> Result<Json<Vec<Application>>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile.id.unwrap();
    let mut apps = queries::get_applications_for_profile(&state.pool, profile_id).await?;

    // Optional filter by application type
    if let Some(type_filter) = params.app_type {
        let app_type = ApplicationType::from_str(&type_filter).ok_or_else(|| {
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
}

pub async fn create_application(
    State(state): State<AppState>,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<Application>), TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let application_type = ApplicationType::from_str(&req.application_type).ok_or_else(|| {
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

    let app = Application {
        id: None,
        lawn_profile_id: profile.id.unwrap(),
        application_type,
        product_name: req.product_name,
        application_date,
        rate_per_1000sqft: req.rate_per_1000sqft,
        coverage_sqft: req.coverage_sqft,
        notes: req.notes,
        weather_snapshot: req.weather_snapshot,
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
