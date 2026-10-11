mod product_link;

use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{Application, ApplicationScope, ApplicationType, FracClass, WeatherSnapshot};
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
        .ok_or_else(|| TurfOpsError::Internal("lawn profile row without an id".into()))?;

    let limit = params
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let offset = params.offset.unwrap_or(0).max(0);

    // Optional filter by application type
    let app_type = params
        .app_type
        .map(|type_filter| {
            ApplicationType::from_str(&type_filter).map_err(|_| {
                TurfOpsError::InvalidData(format!(
                    "Unknown application type filter: {}",
                    type_filter
                ))
            })
        })
        .transpose()?;

    let apps =
        queries::get_applications_page(&state.pool, profile_id, app_type, limit, offset).await?;

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
    /// FRAC classes read off the label (fungicides only).
    pub frac_classes: Option<Vec<FracClass>>,
    /// Shelf product this application used; its name is copied when product_name is empty.
    pub product_id: Option<i64>,
}

/// The checked type and dates of a create or update request.
struct Validated {
    application_type: ApplicationType,
    application_date: NaiveDate,
    follow_up_date: Option<NaiveDate>,
}

/// Check a create or update request's type, dates and plant link.
// @spec APP-API-009
fn validate(req: &CreateApplicationRequest) -> Result<Validated, TurfOpsError> {
    let application_type = ApplicationType::from_str(&req.application_type).map_err(|_| {
        TurfOpsError::InvalidData(format!(
            "Unknown application type: {}",
            req.application_type
        ))
    })?;

    let application_date =
        NaiveDate::parse_from_str(&req.application_date, "%Y-%m-%d").map_err(|_| {
            TurfOpsError::InvalidData(format!(
                "The date \"{}\" isn't a valid date. Pick it from the calendar.",
                req.application_date
            ))
        })?;

    match application_type.scope() {
        ApplicationScope::PlantRequired if req.plant_id.is_none() => {
            return Err(TurfOpsError::InvalidData(format!(
                "{} is logged against a landscape plant. Pick the plant.",
                application_type
            )));
        }
        ApplicationScope::TurfOnly if req.plant_id.is_some() => {
            return Err(TurfOpsError::InvalidData(format!(
                "{} is a lawn application and can't be linked to a plant.",
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
                    "The follow-up date \"{}\" isn't a valid date. Pick it from the calendar.",
                    s
                ))
            })
        })
        .transpose()?;

    if let Some(fu) = follow_up_date {
        if fu < application_date {
            return Err(TurfOpsError::InvalidData(
                "The follow-up date must be on or after the application date.".into(),
            ));
        }
    }

    Ok(Validated {
        application_type,
        application_date,
        follow_up_date,
    })
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
    } = validate(&req)?;

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
        weather_snapshot: req.weather_snapshot,
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
    } = validate(&req)?;

    let (product_id, product_name) = product_link::resolve(
        &state,
        existing.lawn_profile_id,
        req.product_id,
        application_type,
        req.product_name,
    )
    .await?;

    let updated = Application {
        id: Some(id),
        lawn_profile_id: existing.lawn_profile_id,
        application_type,
        product_name,
        application_date,
        rate_per_1000sqft: req.rate_per_1000sqft,
        coverage_sqft: req.coverage_sqft,
        notes: req.notes,
        weather_snapshot: req.weather_snapshot.or(existing.weather_snapshot),
        nitrogen_pct: req.nitrogen_pct,
        phosphorus_pct: req.phosphorus_pct,
        potassium_pct: req.potassium_pct,
        plant_id: req.plant_id,
        follow_up_date,
        frac_classes: fungicide_classes(application_type, req.frac_classes),
        product_id,
        created_at: existing.created_at,
    };

    let affected = queries::update_application(&state.pool, &updated).await?;
    if affected == 0 {
        return Err(TurfOpsError::NotFound(format!(
            "Application {} not found",
            id
        )));
    }

    Ok(Json(updated))
}

pub async fn delete_application(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    queries::delete_application(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod validate_tests {
    use super::*;

    fn request(kind: &str, date: &str) -> CreateApplicationRequest {
        CreateApplicationRequest {
            application_type: kind.into(),
            product_name: None,
            application_date: date.into(),
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: None,
            frac_classes: None,
            product_id: None,
        }
    }

    fn message(req: CreateApplicationRequest) -> String {
        match validate(&req) {
            Err(TurfOpsError::InvalidData(msg)) => msg,
            other => panic!("expected a validation error, got {:?}", other.is_ok()),
        }
    }

    /// Request field names the owner never sees on the form.
    fn names_no_request_field(msg: &str) {
        for field in [
            "plant_id",
            "follow_up_date",
            "application_date",
            "YYYY-MM-DD",
        ] {
            assert!(!msg.contains(field), "{field} in {msg:?}");
        }
    }

    // @spec APP-API-009
    #[test]
    fn validation_messages_use_the_forms_words() {
        names_no_request_field(&message(request("PlantFertilizer", "2026-05-01")));

        let mut bad_follow_up = request("Fertilizer", "2026-05-01");
        bad_follow_up.follow_up_date = Some("05/20/2026".into());
        names_no_request_field(&message(bad_follow_up));

        let mut early_follow_up = request("Fertilizer", "2026-05-01");
        early_follow_up.follow_up_date = Some("2026-04-20".into());
        names_no_request_field(&message(early_follow_up));

        names_no_request_field(&message(request("Fertilizer", "May 1")));
    }
}
