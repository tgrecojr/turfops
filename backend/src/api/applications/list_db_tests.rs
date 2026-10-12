//! `GET /api/v1/applications` against PostgreSQL: order, paging and the type filter.

use super::db_support::{entry, lawn, reply, request, state, store};
use super::{create_application, list_applications, ListApplicationsQuery};
use crate::models::{Application, ApplicationType};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use sqlx::PgPool;

fn query(kind: Option<&str>, limit: Option<i64>, offset: Option<i64>) -> ListApplicationsQuery {
    ListApplicationsQuery {
        app_type: kind.map(String::from),
        limit,
        offset,
        scope: None,
    }
}

async fn list(state: &AppState, q: ListApplicationsQuery) -> Vec<Application> {
    list_applications(State(state.clone()), Query(q))
        .await
        .unwrap()
        .0
}

/// `n` mowing entries on distinct dates, oldest first, after the given ones.
async fn mowings(pool: &PgPool, lawn_id: i64, n: u32) {
    let start = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    for i in 0..n {
        let date = (start + chrono::Duration::days(i.into())).to_string();
        store(pool, &entry(lawn_id, ApplicationType::Mowing, &date, "")).await;
    }
}

// @spec APP-API-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_log_is_newest_first_by_date_then_id(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let a = store(
        &pool,
        &entry(lawn_id, ApplicationType::Fertilizer, "2026-05-01", "A"),
    )
    .await;
    let b = store(
        &pool,
        &entry(lawn_id, ApplicationType::Fertilizer, "2026-05-03", "B"),
    )
    .await;
    let c = store(
        &pool,
        &entry(lawn_id, ApplicationType::Fertilizer, "2026-05-03", "C"),
    )
    .await;
    let state = state(&pool).await;

    let ids: Vec<_> = list(&state, query(None, None, None))
        .await
        .iter()
        .map(|app| app.id.unwrap())
        .collect();
    assert_eq!(ids, vec![c, b, a]);

    let negative = list(&state, query(None, Some(1), Some(-3))).await;
    assert_eq!(negative[0].id, Some(c), "a negative offset is 0");
}

// @spec APP-API-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_page_is_50_by_default_and_clamped_to_1_through_200(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    store(
        &pool,
        &entry(
            lawn_id,
            ApplicationType::Fungicide,
            "2024-06-01",
            "Heritage",
        ),
    )
    .await;
    mowings(&pool, lawn_id, 205).await;
    let state = state(&pool).await;

    assert_eq!(list(&state, query(None, None, None)).await.len(), 50);
    assert_eq!(list(&state, query(None, Some(500), None)).await.len(), 200);
    assert_eq!(list(&state, query(None, Some(0), None)).await.len(), 1);
    assert_eq!(list(&state, query(None, Some(-5), None)).await.len(), 1);

    // The oldest entry is a fungicide behind 205 mowings: the filter runs before paging.
    let fungicides = list(&state, query(Some("Fungicide"), Some(1), None)).await;
    assert_eq!(fungicides.len(), 1);
    assert_eq!(fungicides[0].product_name.as_deref(), Some("Heritage"));
}

// @spec APP-API-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_unknown_type_filter_is_a_400(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    let err = list_applications(
        State(state.clone()),
        Query(query(Some("spray"), None, None)),
    )
    .await
    .unwrap_err();
    assert_eq!(reply(err).await.0, StatusCode::BAD_REQUEST);
    for lenient in ["pre-emergent", "GRUB CONTROL", "Plant-Fertilizer"] {
        assert!(
            list_applications(
                State(state.clone()),
                Query(query(Some(lenient), None, None))
            )
            .await
            .is_ok(),
            "{lenient}"
        );
    }
}

// @spec APP-API-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_a_lawn_profile_the_log_is_not_found(pool: PgPool) {
    let state = state(&pool).await;
    let err = list_applications(State(state.clone()), Query(query(None, None, None)))
        .await
        .unwrap_err();
    assert_eq!(reply(err).await.0, StatusCode::NOT_FOUND);

    let err = create_application(State(state), Json(request("Mowing", "2026-05-01")))
        .await
        .unwrap_err();
    assert_eq!(reply(err).await.0, StatusCode::NOT_FOUND);
}
