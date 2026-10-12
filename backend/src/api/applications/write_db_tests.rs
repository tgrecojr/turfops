//! Creating, updating and deleting applications against PostgreSQL.

use super::db_support::{count, entry, lawn, plant, reply, request, state, store};
use super::{create_application, delete_application, update_application};
use crate::db::queries;
use crate::models::{ApplicationType, FracClass};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sqlx::PgPool;

// @spec APP-API-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_created_entry_comes_back_201_with_its_id(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    let mut req = request("Fertilizer", "2026-05-01");
    req.product_name = Some("Milorganite".into());
    let (status, Json(created)) = create_application(State(state), Json(req)).await.unwrap();
    assert_eq!(status, StatusCode::CREATED);
    let id = created.id.expect("the stored id");
    let stored = queries::get_application_by_id(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.product_name.as_deref(), Some("Milorganite"));
    assert_eq!(stored.application_date, created.application_date);
}

// @spec APP-API-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_invalid_create_is_a_400_and_stores_nothing(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let shrub = plant(&pool, lawn_id, "Viburnum").await;
    let state = state(&pool).await;

    let mut cases = vec![
        request("Spraying", "2026-05-01"),
        request("Fertilizer", "05/01/2026"),
        request("Fertilizer", "2026-05-01T10:00:00"),
        request("PlantFertilizer", "2026-05-01"),
    ];
    let mut bad_follow_up = request("Fertilizer", "2026-05-01");
    bad_follow_up.follow_up_date = Some("next week".into());
    let mut early_follow_up = request("Fertilizer", "2026-05-01");
    early_follow_up.follow_up_date = Some("2026-04-30".into());
    let mut mowing_a_shrub = request("Mowing", "2026-05-01");
    mowing_a_shrub.plant_id = Some(shrub);
    cases.extend([bad_follow_up, early_follow_up, mowing_a_shrub]);

    for req in cases {
        let label = format!("{} {}", req.application_type, req.application_date);
        let err = create_application(State(state.clone()), Json(req))
            .await
            .unwrap_err();
        assert_eq!(reply(err).await.0, StatusCode::BAD_REQUEST, "{label}");
    }
    assert_eq!(count(&pool).await, 0);
}

// @spec APP-API-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_invalid_update_is_a_400_and_changes_nothing(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let id = store(
        &pool,
        &entry(lawn_id, ApplicationType::Fertilizer, "2026-05-01", "Old"),
    )
    .await;
    let state = state(&pool).await;

    let mut req = request("Fertilizer", "2026-05-02");
    req.product_name = Some("New".into());
    req.follow_up_date = Some("2026-04-01".into());
    let err = update_application(State(state), Path(id), Json(req))
        .await
        .unwrap_err();
    assert_eq!(reply(err).await.0, StatusCode::BAD_REQUEST);
    let stored = queries::get_application_by_id(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.product_name.as_deref(), Some("Old"));
    assert_eq!(stored.application_date.to_string(), "2026-05-01");
}

// @spec APP-API-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn updating_an_unknown_entry_is_not_found(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    let err = update_application(
        State(state),
        Path(999),
        Json(request("Mowing", "2026-05-01")),
    )
    .await
    .unwrap_err();
    assert_eq!(reply(err).await.0, StatusCode::NOT_FOUND);
    assert_eq!(count(&pool).await, 0);
}

// @spec APP-API-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_delete_removes_the_entry_and_answers_204(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let id = store(
        &pool,
        &entry(lawn_id, ApplicationType::Mowing, "2026-05-01", ""),
    )
    .await;
    let kept = store(
        &pool,
        &entry(lawn_id, ApplicationType::Mowing, "2026-05-02", ""),
    )
    .await;
    let state = state(&pool).await;

    let status = delete_application(State(state), Path(id)).await.unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(queries::get_application_by_id(&pool, id)
        .await
        .unwrap()
        .is_none());
    assert!(queries::get_application_by_id(&pool, kept)
        .await
        .unwrap()
        .is_some());
}

/// Create `req`, then read the stored entry's FRAC classes back.
async fn stored_classes(
    pool: PgPool,
    state: crate::state::AppState,
    req: super::CreateApplicationRequest,
) -> Option<Vec<FracClass>> {
    let (_, Json(app)) = create_application(State(state), Json(req)).await.unwrap();
    let stored = queries::get_application_by_id(&pool, app.id.unwrap())
        .await
        .unwrap()
        .unwrap();
    stored.frac_classes
}

// @spec APP-FRAC-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn frac_classes_are_stored_for_a_fungicide_only(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    let save = |kind: &str, day: &str, classes: Option<Vec<FracClass>>| {
        let mut req = request(kind, day);
        req.frac_classes = classes;
        stored_classes(pool.clone(), state.clone(), req)
    };

    let premix = vec![FracClass::Frac3, FracClass::Frac11];
    assert_eq!(
        save("Fungicide", "2026-05-01", Some(premix.clone())).await,
        Some(premix.clone())
    );
    assert_eq!(save("Fungicide", "2026-05-02", Some(vec![])).await, None);
    assert_eq!(save("Fungicide", "2026-05-03", None).await, None);
    assert_eq!(save("Fertilizer", "2026-05-04", Some(premix)).await, None);
}
