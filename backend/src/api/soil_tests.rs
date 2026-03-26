use crate::db::{queries, soil_test_queries};
use crate::error::TurfOpsError;
use crate::logic::soil_test_recommendations::generate_soil_test_recommendations;
use crate::models::{SoilTest, SoilTestSummary};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{Datelike, Local, NaiveDate, Utc};
use serde::Deserialize;

const DEFAULT_PAGE_LIMIT: i64 = 50;
const MAX_PAGE_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
pub struct ListSoilTestsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/v1/soil-tests
pub async fn list_soil_tests(
    State(state): State<AppState>,
    Query(params): Query<ListSoilTestsQuery>,
) -> Result<Json<Vec<SoilTest>>, TurfOpsError> {
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

    let tests =
        soil_test_queries::get_soil_tests_for_profile(&state.pool, profile_id, limit, offset)
            .await?;

    Ok(Json(tests))
}

#[derive(Debug, Deserialize)]
pub struct CreateSoilTestRequest {
    pub test_date: String,
    pub lab_name: Option<String>,
    pub ph: f64,
    pub buffer_ph: Option<f64>,
    pub phosphorus_ppm: Option<f64>,
    pub potassium_ppm: Option<f64>,
    pub calcium_ppm: Option<f64>,
    pub magnesium_ppm: Option<f64>,
    pub sulfur_ppm: Option<f64>,
    pub iron_ppm: Option<f64>,
    pub manganese_ppm: Option<f64>,
    pub zinc_ppm: Option<f64>,
    pub boron_ppm: Option<f64>,
    pub copper_ppm: Option<f64>,
    pub organic_matter_pct: Option<f64>,
    pub cec: Option<f64>,
    pub notes: Option<String>,
}

/// POST /api/v1/soil-tests
pub async fn create_soil_test(
    State(state): State<AppState>,
    Json(req): Json<CreateSoilTestRequest>,
) -> Result<(StatusCode, Json<SoilTest>), TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let test_date = NaiveDate::parse_from_str(&req.test_date, "%Y-%m-%d").map_err(|_| {
        TurfOpsError::InvalidData(format!(
            "Invalid date format: {}. Expected YYYY-MM-DD",
            req.test_date
        ))
    })?;

    if req.ph < 0.0 || req.ph > 14.0 {
        return Err(TurfOpsError::InvalidData(
            "pH must be between 0 and 14".into(),
        ));
    }

    let test = SoilTest {
        id: None,
        lawn_profile_id: profile_id,
        test_date,
        lab_name: req.lab_name,
        ph: req.ph,
        buffer_ph: req.buffer_ph,
        phosphorus_ppm: req.phosphorus_ppm,
        potassium_ppm: req.potassium_ppm,
        calcium_ppm: req.calcium_ppm,
        magnesium_ppm: req.magnesium_ppm,
        sulfur_ppm: req.sulfur_ppm,
        iron_ppm: req.iron_ppm,
        manganese_ppm: req.manganese_ppm,
        zinc_ppm: req.zinc_ppm,
        boron_ppm: req.boron_ppm,
        copper_ppm: req.copper_ppm,
        organic_matter_pct: req.organic_matter_pct,
        cec: req.cec,
        notes: req.notes,
        created_at: Utc::now(),
    };

    let id = soil_test_queries::create_soil_test(&state.pool, &test).await?;
    let created = SoilTest {
        id: Some(id),
        ..test
    };

    Ok((StatusCode::CREATED, Json(created)))
}

/// PUT /api/v1/soil-tests/:id
pub async fn update_soil_test(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<CreateSoilTestRequest>,
) -> Result<Json<SoilTest>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let test_date = NaiveDate::parse_from_str(&req.test_date, "%Y-%m-%d").map_err(|_| {
        TurfOpsError::InvalidData(format!(
            "Invalid date format: {}. Expected YYYY-MM-DD",
            req.test_date
        ))
    })?;

    if req.ph < 0.0 || req.ph > 14.0 {
        return Err(TurfOpsError::InvalidData(
            "pH must be between 0 and 14".into(),
        ));
    }

    let test = SoilTest {
        id: Some(id),
        lawn_profile_id: profile_id,
        test_date,
        lab_name: req.lab_name,
        ph: req.ph,
        buffer_ph: req.buffer_ph,
        phosphorus_ppm: req.phosphorus_ppm,
        potassium_ppm: req.potassium_ppm,
        calcium_ppm: req.calcium_ppm,
        magnesium_ppm: req.magnesium_ppm,
        sulfur_ppm: req.sulfur_ppm,
        iron_ppm: req.iron_ppm,
        manganese_ppm: req.manganese_ppm,
        zinc_ppm: req.zinc_ppm,
        boron_ppm: req.boron_ppm,
        copper_ppm: req.copper_ppm,
        organic_matter_pct: req.organic_matter_pct,
        cec: req.cec,
        notes: req.notes,
        created_at: Utc::now(),
    };

    let updated = soil_test_queries::update_soil_test(&state.pool, id, &test).await?;
    Ok(Json(updated))
}

/// DELETE /api/v1/soil-tests/:id
pub async fn delete_soil_test(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    soil_test_queries::delete_soil_test(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/soil-tests/recommendations
pub async fn get_soil_test_recommendations(
    State(state): State<AppState>,
) -> Result<Json<SoilTestSummary>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let test = soil_test_queries::get_latest_soil_test(&state.pool, profile_id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No soil tests found".into()))?;

    // Get current year applications for N budget calculation
    let year = Local::now().year();
    let start_date = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let end_date = NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap();
    let apps = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        start_date,
        end_date,
    )
    .await?;

    let summary = generate_soil_test_recommendations(&test, &profile, &apps);

    Ok(Json(summary))
}
