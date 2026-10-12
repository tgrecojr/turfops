//! `GET /api/v1/nitrogen-budget` against PostgreSQL (`--features db-tests`).

use super::*;
use crate::clock::Clock;
use crate::logic::data_sync::DataSyncService;
use crate::models::{Application, ApplicationType, GrassType, LawnProfile};
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use sqlx::PgPool;

fn state(pool: &PgPool) -> AppState {
    let sync = DataSyncService::for_tests(pool.clone(), None, Clock::System);
    AppState::new(pool.clone(), sync, None, false, Clock::System)
}

async fn lawn(pool: &PgPool) -> i64 {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(pool, &profile).await.unwrap()
}

/// A log entry: `n_pct`% N at `rate` lb product / 1000 sq ft.
async fn log(
    pool: &PgPool,
    profile_id: i64,
    kind: ApplicationType,
    on: NaiveDate,
    n_pct: Option<f64>,
    rate: Option<f64>,
    plant_name: Option<&str>,
) {
    let app = Application {
        id: None,
        lawn_profile_id: profile_id,
        application_type: kind,
        product_name: Some(format!("Product {on}")),
        application_date: on,
        rate_per_1000sqft: rate,
        coverage_sqft: None,
        notes: None,
        plant_name: plant_name.map(str::to_string),
        nitrogen_pct: n_pct,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    };
    queries::create_application(pool, &app).await.unwrap();
}

async fn budget(state: &AppState, year: Option<i32>) -> Result<NitrogenBudget, TurfOpsError> {
    get_nitrogen_budget(State(state.clone()), Query(NitrogenBudgetQuery { year }))
        .await
        .map(|Json(b)| b)
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

// @spec RULE-BUDGET-003, RULE-BUDGET-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_budget_counts_the_years_lawn_nitrogen_against_the_target(pool: PgPool) {
    let id = lawn(&pool).await;
    let today = Clock::System.today();
    let last = today.year() - 1;
    let fert = ApplicationType::Fertilizer;
    // Last year, logged out of order: 1.0 + 0.5 lb N.
    log(&pool, id, fert, d(last, 9, 1), Some(20.0), Some(5.0), None).await;
    log(&pool, id, fert, d(last, 4, 15), Some(10.0), Some(5.0), None).await;
    // Not lawn nitrogen: no rate, no analysis, a plant's feeding.
    log(&pool, id, fert, d(last, 5, 1), Some(20.0), None, None).await;
    log(&pool, id, fert, d(last, 5, 2), None, Some(5.0), None).await;
    let plant = ApplicationType::PlantFertilizer;
    log(
        &pool,
        id,
        plant,
        d(last, 5, 3),
        Some(20.0),
        Some(5.0),
        Some("Hydrangea"),
    )
    .await;
    // This year: today counts, tomorrow does not yet.
    log(&pool, id, fert, today, Some(10.0), Some(10.0), None).await;
    log(
        &pool,
        id,
        fert,
        today + Duration::days(1),
        Some(40.0),
        Some(5.0),
        None,
    )
    .await;
    let state = state(&pool);

    let past = budget(&state, Some(last)).await.unwrap();
    assert_eq!(past.year, last);
    assert_eq!(past.target_lbs_per_1000sqft, 3.5);
    assert!((past.applied_lbs_per_1000sqft - 1.5).abs() < 1e-9);
    assert!((past.remaining_lbs_per_1000sqft - 2.0).abs() < 1e-9);
    assert!((past.percent_of_target - 1.5 / 3.5 * 100.0).abs() < 1e-9);
    let dates: Vec<NaiveDate> = past.applications.iter().map(|a| a.date).collect();
    assert_eq!(dates, vec![d(last, 4, 15), d(last, 9, 1)]);
    assert_eq!(past.applications[0].n_lbs_per_1000sqft, 0.5);
    let range = &past.grass_type_target;
    assert_eq!(
        (range.min_lbs_per_1000sqft, range.max_lbs_per_1000sqft),
        (2.0, 4.0)
    );

    let now = budget(&state, None).await.unwrap();
    assert_eq!(now.year, today.year());
    assert!((now.applied_lbs_per_1000sqft - 1.0).abs() < 1e-9);
    assert_eq!(now.applications.len(), 1);
}

// @spec RULE-BUDGET-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_overspent_budget_has_nothing_left_and_caps_the_percentage(pool: PgPool) {
    let id = lawn(&pool).await;
    let last = Clock::System.today().year() - 1;
    let fert = ApplicationType::Fertilizer;
    log(
        &pool,
        id,
        fert,
        d(last, 6, 1),
        Some(46.0),
        Some(100.0),
        None,
    )
    .await; // 46 lb N
    let over = budget(&state(&pool), Some(last)).await.unwrap();
    assert_eq!(over.remaining_lbs_per_1000sqft, 0.0);
    assert_eq!(over.percent_of_target, 999.0);
}

// @spec RULE-BUDGET-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn no_lawn_is_404_and_an_unrepresentable_year_is_400(pool: PgPool) {
    let state = state(&pool);
    let status = |e: TurfOpsError| e.into_response().status().as_u16();
    assert_eq!(status(budget(&state, None).await.unwrap_err()), 404);
    lawn(&pool).await;
    assert_eq!(
        status(budget(&state, Some(300_000)).await.unwrap_err()),
        400
    );
    assert_eq!(
        status(budget(&state, Some(-300_000)).await.unwrap_err()),
        400
    );
}
