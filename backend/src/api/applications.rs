mod product_link;
mod validate;

use validate::{
    deleted, follow_up_answer_to_clear, parse_scope, replace_user_fields, validate, Validated,
};

use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{Application, ApplicationType, FracClass};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
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
    /// `turf` or `landscape`; absent = both.
    pub scope: Option<String>,
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
        .ok_or_else(|| TurfOpsError::Internal("lawn profile row without an id".into()))?;

    let limit = params
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let offset = params.offset.unwrap_or(0).max(0);

    // Optional filter by application type
    let app_type = params
        .app_type
        .as_deref()
        .map(parse_type_filter)
        .transpose()?;

    // @spec APP-API-012
    let scope = parse_scope(params.scope.as_deref())?;
    let apps =
        queries::get_applications_page(&state.pool, profile_id, app_type, scope, limit, offset)
            .await?;

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
    pub nitrogen_pct: Option<f64>,
    pub phosphorus_pct: Option<f64>,
    pub potassium_pct: Option<f64>,
    pub plant_id: Option<i64>,
    pub follow_up_date: Option<String>,
    /// FRAC classes read off the label (fungicides only).
    pub frac_classes: Option<Vec<FracClass>>,
    /// Shelf product this application used; its name is copied when product_name is empty.
    pub product_id: Option<i64>,
}

/// The list's `type` filter, leniently parsed (`pre-emergent`, `grub control`, `mow`).
pub(crate) fn parse_type_filter(raw: &str) -> Result<ApplicationType, TurfOpsError> {
    ApplicationType::from_str(raw)
        .map_err(|_| TurfOpsError::InvalidData(format!("Unknown application type filter: {raw}")))
}

/// Only a fungicide carries FRAC classes; an empty pick means "not recorded".
fn fungicide_classes(
    application_type: ApplicationType,
    classes: Option<Vec<FracClass>>,
) -> Option<Vec<FracClass>> {
    classes.filter(|c| application_type == ApplicationType::Fungicide && !c.is_empty())
}

pub async fn create_application(
    State(state): State<AppState>,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<Application>), TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let Validated {
        application_type,
        application_date,
        follow_up_date,
    } = validate(&req, None)?;

    let lawn_profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::Internal("lawn profile row without an id".into()))?;
    let (product_id, product_name) = product_link::resolve(
        &state,
        lawn_profile_id,
        req.product_id,
        application_type,
        req.product_name,
    )
    .await?;

    let app = Application {
        id: None,
        lawn_profile_id,
        application_type,
        product_name,
        application_date,
        rate_per_1000sqft: req.rate_per_1000sqft,
        coverage_sqft: req.coverage_sqft,
        notes: req.notes,
        plant_name: None,
        nitrogen_pct: req.nitrogen_pct,
        phosphorus_pct: req.phosphorus_pct,
        potassium_pct: req.potassium_pct,
        plant_id: req.plant_id,
        follow_up_date,
        frac_classes: fungicide_classes(application_type, req.frac_classes),
        product_id,
        created_at: Utc::now(),
    };

    let id = queries::create_application(&state.pool, &app).await?;

    let created = Application {
        id: Some(id),
        ..app
    };

    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn update_application(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<Json<Application>, TurfOpsError> {
    let existing = queries::get_application_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Application {} not found", id)))?;

    let Validated {
        application_type,
        application_date,
        follow_up_date,
    } = validate(&req, Some(&existing))?;

    let (product_id, product_name) = product_link::resolve(
        &state,
        existing.lawn_profile_id,
        req.product_id,
        application_type,
        req.product_name,
    )
    .await?;

    // @spec APP-API-005
    let updated = replace_user_fields(
        &existing,
        Application {
            id: Some(id),
            lawn_profile_id: existing.lawn_profile_id,
            application_type,
            product_name,
            application_date,
            rate_per_1000sqft: req.rate_per_1000sqft,
            coverage_sqft: req.coverage_sqft,
            notes: req.notes,
            plant_name: None,
            nitrogen_pct: req.nitrogen_pct,
            phosphorus_pct: req.phosphorus_pct,
            potassium_pct: req.potassium_pct,
            plant_id: req.plant_id,
            follow_up_date,
            frac_classes: fungicide_classes(application_type, req.frac_classes),
            product_id,
            created_at: existing.created_at,
        },
    );

    let affected = queries::update_application(&state.pool, &updated).await?;
    deleted(affected, id)?;
    // @spec APP-FOLLOW-008
    if let Some(answer) = follow_up_answer_to_clear(id, existing.follow_up_date, follow_up_date) {
        queries::delete_recommendation_state(&state.pool, &answer).await?;
    }

    Ok(Json(updated))
}

pub async fn delete_application(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    // @spec APP-API-011
    deleted(queries::delete_application(&state.pool, id).await?, id)?;
    Ok(StatusCode::NO_CONTENT)
}
