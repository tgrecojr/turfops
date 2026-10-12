//! The `applications` tool over a real log.

use crate::api;
use crate::api::applications::ListApplicationsQuery;
use crate::db::queries;
use crate::mcp::db_support::*;
use crate::mcp::test_support::*;
use crate::models::ApplicationType;
use axum::extract::{Query, State};
use chrono::Duration;
use serde_json::json;
use sqlx::PgPool;

/// 25 mowings on consecutive days from 2026-05-01 and a fertilizer every 5th day.
async fn season_log(pool: &PgPool, lawn: i64) {
    let start = date("2026-05-01");
    for i in 0..25 {
        let on = start + Duration::days(i);
        log(pool, lawn, ApplicationType::Mowing, on, &format!("Mow {i}")).await;
        if i % 5 == 0 {
            log(
                pool,
                lawn,
                ApplicationType::Fertilizer,
                on,
                &format!("Feed {i}"),
            )
            .await;
        }
    }
}

fn dates(reply: &serde_json::Value) -> Vec<&str> {
    names(reply, "application_date")
}

// @spec MCP-TOOL-015
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_a_range_it_is_the_list_endpoint_newest_first_and_clamped(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "apps", 2).await;
    season_log(&pool, lawn).await;
    let list = |app_type: Option<&str>, limit: i64| {
        let state = state.clone();
        let app_type = app_type.map(String::from);
        async move {
            body(
                api::applications::list_applications(
                    State(state),
                    Query(ListApplicationsQuery {
                        app_type,
                        limit: Some(limit),
                        offset: None,
                        scope: None,
                    }),
                )
                .await,
            )
        }
    };

    let default = call_tool(&state, "applications", json!({})).await.json();
    assert_eq!(default.as_array().unwrap().len(), 20);
    assert_eq!(default, list(None, 20).await);
    assert_eq!(dates(&default)[0], "2026-05-25");

    let feeds = call_tool(&state, "applications", json!({"type": "Fertilizer"}))
        .await
        .json();
    assert_eq!(feeds, list(Some("Fertilizer"), 20).await);
    assert_eq!(
        dates(&feeds),
        [
            "2026-05-21",
            "2026-05-16",
            "2026-05-11",
            "2026-05-06",
            "2026-05-01"
        ]
    );

    let one = call_tool(&state, "applications", json!({"limit": 0}))
        .await
        .json();
    assert_eq!(one.as_array().unwrap().len(), 1);
    let most = call_tool(&state, "applications", json!({"limit": 1000}))
        .await
        .json();
    assert_eq!(most, list(None, 100).await);
    assert_eq!(most.as_array().unwrap().len(), 30);
}

// @spec MCP-TOOL-016
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_date_range_is_inclusive_typed_newest_first_and_limited(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "range", 2).await;
    season_log(&pool, lawn).await;

    let week = call_tool(
        &state,
        "applications",
        json!({"since": "2026-05-06", "until": "2026-05-11"}),
    )
    .await
    .json();
    let got = dates(&week);
    assert_eq!(got.len(), 8, "6 mowings + 2 feedings: {got:?}");
    assert_eq!((got[0], got[7]), ("2026-05-11", "2026-05-06"));
    assert!(got.windows(2).all(|w| w[0] >= w[1]), "newest first");

    let feeds = call_tool(
        &state,
        "applications",
        json!({"type": "fertilizer", "since": "2026-05-06"}),
    )
    .await
    .json();
    assert_eq!(
        dates(&feeds),
        ["2026-05-21", "2026-05-16", "2026-05-11", "2026-05-06"]
    );

    let until = call_tool(
        &state,
        "applications",
        json!({"until": "2026-05-02", "limit": 2}),
    )
    .await
    .json();
    assert_eq!(dates(&until), ["2026-05-02", "2026-05-01"]);

    let defaulted = call_tool(&state, "applications", json!({"since": "2026-01-01"}))
        .await
        .json();
    assert_eq!(defaulted.as_array().unwrap().len(), 20);
    let clamped = call_tool(
        &state,
        "applications",
        json!({"since": "2026-01-01", "limit": 0}),
    )
    .await
    .json();
    assert_eq!(dates(&clamped), ["2026-05-25"]);
}

// @spec MCP-TOOL-026
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_range_query_applies_the_type_and_limit_itself(pool: PgPool) {
    let lawn = lawn(&pool).await;
    season_log(&pool, lawn).await;
    let (start, end) = (date("2026-01-01"), date("2027-01-01"));
    let page = queries::get_applications_in_range_page(&pool, lawn, None, start, end, 3)
        .await
        .unwrap();
    assert_eq!(page.len(), 3, "never more than the limit");
    let feeds = queries::get_applications_in_range_page(
        &pool,
        lawn,
        Some(ApplicationType::Fertilizer),
        start,
        end,
        2,
    )
    .await
    .unwrap();
    let got: Vec<(ApplicationType, String)> = feeds
        .iter()
        .map(|a| (a.application_type, a.application_date.to_string()))
        .collect();
    assert_eq!(
        got,
        [
            (ApplicationType::Fertilizer, "2026-05-21".to_string()),
            (ApplicationType::Fertilizer, "2026-05-16".to_string())
        ]
    );
}
