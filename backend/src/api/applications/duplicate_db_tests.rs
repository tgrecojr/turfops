//! The application duplicate key against PostgreSQL's unique indexes.

use super::db_support::{count, entry, lawn, plant, reply, request, state};
use super::{create_application, update_application};
use crate::db::{plant_queries, queries};
use crate::error::TurfOpsError;
use crate::models::{Application, ApplicationType};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sqlx::PgPool;

/// The unique index a store violated, or `None` when it was stored.
async fn violated(pool: &PgPool, app: &Application) -> Option<String> {
    match queries::create_application(pool, app).await {
        Ok(_) => None,
        Err(TurfOpsError::Database(sqlx::Error::Database(db))) if db.is_unique_violation() => {
            db.constraint().map(String::from)
        }
        Err(other) => panic!("unexpected error: {other}"),
    }
}

fn named(lawn_id: i64, name: Option<&str>, plant_id: Option<i64>) -> Application {
    Application {
        product_name: name.map(String::from),
        plant_id,
        ..entry(lawn_id, ApplicationType::Fungicide, "2026-06-01", "")
    }
}

// @spec APP-DUP-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn turf_duplicates_compare_names_loosely(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let turf = Some("applications_unique_turf".to_string());

    assert_eq!(
        violated(&pool, &named(lawn_id, Some("Heritage G"), None)).await,
        None
    );
    assert_eq!(
        violated(&pool, &named(lawn_id, Some("  heritage   g "), None)).await,
        turf,
        "case and whitespace do not distinguish names"
    );
    assert_eq!(
        violated(&pool, &named(lawn_id, Some("Banner Maxx"), None)).await,
        None,
        "two products of one type on one day"
    );
    let mut next_day = named(lawn_id, Some("Heritage G"), None);
    next_day.application_date = "2026-06-02".parse().unwrap();
    assert_eq!(violated(&pool, &next_day).await, None);

    assert_eq!(violated(&pool, &named(lawn_id, None, None)).await, None);
    assert_eq!(
        violated(&pool, &named(lawn_id, Some(""), None)).await,
        turf,
        "a missing name equals an empty one"
    );
}

// @spec APP-DUP-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn plant_duplicates_are_per_plant(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let holly = plant(&pool, lawn_id, "Holly").await;
    let boxwood = plant(&pool, lawn_id, "Boxwood").await;

    assert_eq!(
        violated(&pool, &named(lawn_id, Some("Daconil"), Some(holly))).await,
        None
    );
    assert_eq!(
        violated(&pool, &named(lawn_id, Some("DACONIL "), Some(holly))).await,
        Some("applications_unique_plant".to_string())
    );
    assert_eq!(
        violated(&pool, &named(lawn_id, Some("Daconil"), Some(boxwood))).await,
        None,
        "another plant"
    );
    assert_eq!(
        violated(&pool, &named(lawn_id, Some("Daconil"), None)).await,
        None,
        "the lawn"
    );
}

const DUPLICATE: &str = "An identical entry (same type, product and date) is already logged. \
     Edit the existing entry, or change the product name if this was a different product.";

// @spec APP-DUP-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_duplicate_create_or_update_is_a_409_that_says_what_to_do(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    let heritage = || {
        let mut req = request("Fungicide", "2026-06-01");
        req.product_name = Some("Heritage G".into());
        req
    };
    let _first = create_application(State(state.clone()), Json(heritage()))
        .await
        .unwrap();

    let err = create_application(State(state.clone()), Json(heritage()))
        .await
        .unwrap_err();
    assert_eq!(
        reply(err).await,
        (StatusCode::CONFLICT, DUPLICATE.to_string())
    );

    let mut other = request("Fungicide", "2026-06-01");
    other.product_name = Some("Banner Maxx".into());
    let (_, Json(banner)) = create_application(State(state.clone()), Json(other))
        .await
        .unwrap();
    let err = update_application(State(state), Path(banner.id.unwrap()), Json(heritage()))
        .await
        .unwrap_err();
    assert_eq!(
        reply(err).await,
        (StatusCode::CONFLICT, DUPLICATE.to_string())
    );
}

// @spec APP-LOG-011
#[sqlx::test(migrations = "src/db/migrations")]
async fn deleting_a_plant_never_collides_with_the_log(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let holly = plant(&pool, lawn_id, "Holly").await;
    let boxwood = plant(&pool, lawn_id, "Boxwood").await;
    let fed = |plant_id| Application {
        plant_id,
        ..entry(
            lawn_id,
            ApplicationType::Fertilizer,
            "2026-05-01",
            "Holly-tone",
        )
    };
    for plant_id in [None, Some(holly), Some(boxwood)] {
        queries::create_application(&pool, &fed(plant_id))
            .await
            .unwrap();
    }

    // Once unlinked, each plant's entry matches the lawn's and the other plant's.
    assert_eq!(plant_queries::delete_plant(&pool, holly).await.unwrap(), 1);
    assert_eq!(
        plant_queries::delete_plant(&pool, boxwood).await.unwrap(),
        1
    );
    assert_eq!(count(&pool).await, 3, "every entry stays in the log");
}
