//! The soil-test endpoints against PostgreSQL (`--features db-tests`).

use super::*;
use crate::logic::data_sync::test_env::offline_state;
use crate::models::{GrassType, LawnProfile};
use axum::response::IntoResponse;
use serde_json::Value;
use sqlx::PgPool;

/// A lawn profile and the app state over `pool`.
pub(super) async fn lawn(pool: PgPool) -> (AppState, i64) {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    let id = queries::create_lawn_profile(&pool, &profile).await.unwrap();
    (offline_state(pool).await, id)
}

pub(super) fn req(date: &str, ph: f64) -> CreateSoilTestRequest {
    CreateSoilTestRequest {
        test_date: date.into(),
        lab_name: None,
        ph,
        buffer_ph: None,
        phosphorus_ppm: None,
        potassium_ppm: None,
        calcium_ppm: None,
        magnesium_ppm: None,
        sulfur_ppm: None,
        iron_ppm: None,
        manganese_ppm: None,
        zinc_ppm: None,
        boron_ppm: None,
        copper_ppm: None,
        organic_matter_pct: None,
        cec: None,
        notes: None,
    }
}

/// Every measurement reported.
fn full(date: &str) -> CreateSoilTestRequest {
    CreateSoilTestRequest {
        lab_name: Some("Waypoint".into()),
        buffer_ph: Some(6.9),
        phosphorus_ppm: Some(13.0),
        potassium_ppm: Some(50.0),
        calcium_ppm: Some(196.0),
        magnesium_ppm: Some(44.0),
        sulfur_ppm: Some(5.0),
        iron_ppm: Some(2.4),
        manganese_ppm: Some(10.9),
        zinc_ppm: Some(0.1),
        boron_ppm: Some(0.2),
        copper_ppm: Some(0.08),
        organic_matter_pct: Some(3.0),
        cec: Some(8.0),
        notes: Some("spring sample".into()),
        ..req(date, 5.9)
    }
}

pub(super) async fn add(state: &AppState, body: CreateSoilTestRequest) -> SoilTest {
    let (status, Json(test)) = create_soil_test(State(state.clone()), Json(body))
        .await
        .unwrap();
    assert_eq!(status, StatusCode::CREATED);
    test
}

async fn list(state: &AppState, limit: Option<i64>, offset: Option<i64>) -> Vec<SoilTest> {
    let query = ListSoilTestsQuery { limit, offset };
    list_soil_tests(State(state.clone()), Query(query))
        .await
        .unwrap()
        .0
}

pub(super) async fn advice(state: &AppState) -> Result<SoilTestSummary, TurfOpsError> {
    get_soil_test_recommendations(State(state.clone()))
        .await
        .map(|Json(s)| s)
}

/// The test's fields without `created_at` (set by the database).
fn fields(test: &SoilTest) -> Value {
    let mut value = serde_json::to_value(test).unwrap();
    value.as_object_mut().unwrap().remove("created_at");
    value
}

fn not_found<T: std::fmt::Debug>(result: Result<T, TurfOpsError>, message: &str) {
    match result {
        Err(TurfOpsError::NotFound(msg)) => {
            assert_eq!(msg, message);
            let status = TurfOpsError::NotFound(msg).into_response().status();
            assert_eq!(status, StatusCode::NOT_FOUND);
        }
        other => panic!("expected 404 {message:?}, got {other:?}"),
    }
}

fn dates(tests: &[SoilTest]) -> Vec<String> {
    tests.iter().map(|t| t.test_date.to_string()).collect()
}

// @spec SOIL-API-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_list_is_newest_test_date_first_and_paged(pool: PgPool) {
    let (state, profile_id) = lawn(pool.clone()).await;
    sqlx::query(
        "INSERT INTO soil_tests (lawn_profile_id, test_date, ph) \
         SELECT $1, DATE '2000-01-01' + g, 6.5 FROM generate_series(1, 205) AS g",
    )
    .bind(profile_id)
    .execute(&pool)
    .await
    .unwrap();

    let page = list(&state, None, None).await;
    assert_eq!(page.len(), 50, "default limit");
    assert_eq!(page[0].test_date.to_string(), "2000-07-24");
    assert!(page.windows(2).all(|w| w[0].test_date > w[1].test_date));
    assert_eq!(
        list(&state, Some(500), None).await.len(),
        200,
        "clamped to 200"
    );
    assert_eq!(list(&state, Some(0), None).await.len(), 1, "clamped to 1");
    let second = list(&state, Some(1), Some(1)).await;
    assert_eq!(dates(&second), ["2000-07-23"]);
    let negative = list(&state, Some(1), Some(-5)).await;
    assert_eq!(dates(&negative), ["2000-07-24"], "offset never below 0");
}

// @spec SOIL-API-002, SOIL-DATA-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_created_test_is_stored_with_every_field_and_returned_with_its_id(pool: PgPool) {
    let (state, profile_id) = lawn(pool.clone()).await;
    let created = add(&state, full("2026-03-23")).await;
    assert!(created.id.is_some());
    assert_eq!(created.lawn_profile_id, profile_id);

    let stored = list(&state, None, None).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(fields(&stored[0]), fields(&created));

    let minimal = add(&state, req("2026-04-01", 6.4)).await;
    let stored = list(&state, None, None).await;
    assert_eq!(
        fields(&stored[0]),
        fields(&minimal),
        "only date and pH required"
    );

    let no_ph = sqlx::query(
        "INSERT INTO soil_tests (lawn_profile_id, test_date) VALUES ($1, DATE '2026-05-01')",
    )
    .bind(profile_id)
    .execute(&pool)
    .await;
    assert!(no_ph.is_err(), "pH is required");
}

// @spec SOIL-API-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_update_replaces_every_user_field_and_returns_the_stored_test(pool: PgPool) {
    let (state, _) = lawn(pool).await;
    let id = add(&state, full("2026-03-23")).await.id.unwrap();

    let Json(updated) =
        update_soil_test(State(state.clone()), Path(id), Json(req("2026-03-24", 6.8)))
            .await
            .unwrap();
    assert_eq!(updated.id, Some(id));
    assert_eq!(updated.test_date.to_string(), "2026-03-24");
    assert_eq!(updated.ph, 6.8);
    assert_eq!(
        updated.lab_name, None,
        "absent fields are cleared, not kept"
    );
    assert_eq!(updated.phosphorus_ppm, None);
    assert_eq!(updated.notes, None);
    let stored = list(&state, None, None).await;
    assert_eq!(
        serde_json::to_value(&stored[0]).unwrap(),
        serde_json::to_value(&updated).unwrap()
    );
}

// @spec SOIL-API-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn updating_a_missing_test_is_not_found(pool: PgPool) {
    let (state, _) = lawn(pool).await;
    let result = update_soil_test(State(state), Path(999), Json(req("2026-03-24", 6.8))).await;
    not_found(result, "Soil test 999 not found");
}

// @spec SOIL-API-005, SOIL-API-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn deleting_a_test_is_204_and_a_missing_one_404(pool: PgPool) {
    let (state, _) = lawn(pool).await;
    let id = add(&state, req("2026-03-23", 6.5)).await.id.unwrap();
    let status = delete_soil_test(State(state.clone()), Path(id))
        .await
        .unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(list(&state, None, None).await.is_empty());
    let again = delete_soil_test(State(state), Path(id)).await;
    not_found(again, &format!("Soil test {id} not found"));
}

// @spec SOIL-API-008, SOIL-ADV-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn advice_comes_from_the_latest_test_alone_and_carries_it(pool: PgPool) {
    let (state, _) = lawn(pool).await;
    add(&state, full("2024-05-01")).await; // acid, low P, K and every micronutrient
    let mut fine = req("2026-03-23", 6.5);
    fine.phosphorus_ppm = Some(30.0);
    fine.potassium_ppm = Some(140.0);
    let latest = add(&state, fine).await;

    let summary = advice(&state).await.unwrap();
    assert_eq!(fields(&summary.soil_test), fields(&latest));
    assert!(
        summary.ph_recommendation.is_none(),
        "the old acid test is not blended in"
    );
    assert!(summary.micronutrient_recommendations.is_empty());
    let npk = summary.npk_recommendation.unwrap();
    assert_eq!(npk.phosphorus_level, crate::models::NutrientLevel::Adequate);
    assert_eq!(npk.potassium_level, crate::models::NutrientLevel::Adequate);
}

// @spec SOIL-API-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn advice_with_no_test_stored_is_not_found(pool: PgPool) {
    let (state, _) = lawn(pool).await;
    not_found(advice(&state).await, "No soil tests found");
}

// @spec SOIL-API-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_a_lawn_profile_the_endpoints_are_not_found(pool: PgPool) {
    let state = offline_state(pool).await;
    let message = "No lawn profile found";
    let query = ListSoilTestsQuery {
        limit: None,
        offset: None,
    };
    not_found(
        list_soil_tests(State(state.clone()), Query(query)).await,
        message,
    );
    not_found(
        create_soil_test(State(state.clone()), Json(req("2026-03-23", 6.5))).await,
        message,
    );
    not_found(
        update_soil_test(State(state.clone()), Path(1), Json(req("2026-03-23", 6.5))).await,
        message,
    );
    not_found(advice(&state).await, message);
}
