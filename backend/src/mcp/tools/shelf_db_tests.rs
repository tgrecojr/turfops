//! The profile, the feed, soil tests and the shelf: each tool's reply against its
//! handler's body.

use crate::api;
use crate::api::products::ListQuery;
use crate::api::recommendations::ListRecommendationsQuery;
use crate::api::soil_tests::ListSoilTestsQuery;
use crate::db::queries;
use crate::mcp::db_support::*;
use crate::mcp::test_support::*;
use crate::models::product::ProductCategory;
use crate::models::{Application, ApplicationType, Severity};
use axum::extract::{Query, State};
use serde_json::{json, Value};
use sqlx::PgPool;

// @spec MCP-TOOL-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn lawn_profile_is_the_profile_body(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "profile", 2).await;
    let reply = call_tool(&state, "lawn_profile", json!({})).await.json();
    assert_eq!(
        reply,
        body(api::profile::get_profile(State(state.clone())).await)
    );
    assert_eq!(reply["name"], "Front");
    assert_eq!(reply["lawn_size_sqft"], 5000.0);
}

async fn feed(state: &crate::state::AppState, include_inactive: bool) -> Value {
    body(
        api::recommendations::list_recommendations(
            State(state.clone()),
            Query(ListRecommendationsQuery { include_inactive }),
        )
        .await,
    )
}

/// Three follow-ups due today, so the feed has several items whatever the season.
async fn follow_ups(pool: &PgPool, lawn: i64, today: chrono::NaiveDate) {
    for (i, kind) in [
        ApplicationType::Fungicide,
        ApplicationType::Overseed,
        ApplicationType::GrubControl,
    ]
    .into_iter()
    .enumerate()
    {
        let app = Application {
            id: None,
            lawn_profile_id: lawn,
            application_type: kind,
            product_name: Some(format!("Product {i}")),
            application_date: today - chrono::Duration::days(10 + i as i64),
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            plant_name: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: Some(today),
            frac_classes: None,
            product_id: None,
            created_at: chrono::Utc::now(),
        };
        queries::create_application(pool, &app).await.unwrap();
    }
}

// @spec MCP-TOOL-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn recommendations_relay_the_feed_unchanged_and_in_order(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "feed", 20).await;
    follow_ups(&pool, lawn, state.clock.today()).await;
    let active = feed(&state, false).await;
    assert!(active.as_array().unwrap().len() >= 3, "{active}");
    let answered = active[0]["id"].as_str().unwrap().to_string();
    queries::upsert_recommendation_state(&pool, &answered, true, false, Some(Severity::Critical))
        .await
        .unwrap();

    let reply = call_tool(&state, "recommendations", json!({})).await.json();
    let handler = feed(&state, false).await;
    assert_eq!(timeless(reply.clone()), timeless(handler));
    assert!(
        !names(&reply, "id").contains(&answered.as_str()),
        "default: active only"
    );

    let all = call_tool(&state, "recommendations", json!({"include_inactive": true}))
        .await
        .json();
    let handler = feed(&state, true).await;
    assert_eq!(timeless(all.clone()), timeless(handler));
    assert!(names(&all, "id").contains(&answered.as_str()));
}

// @spec MCP-TOOL-019
#[sqlx::test(migrations = "src/db/migrations")]
async fn soil_tests_are_newest_first_five_by_default_and_clamped(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "soil", 2).await;
    for month in 1..=7 {
        soil_test(
            &pool,
            lawn,
            date(&format!("2025-0{month}-01")),
            6.0 + month as f64 / 10.0,
        )
        .await;
    }
    let list = |limit: Option<i64>| {
        let state = state.clone();
        async move {
            body(
                api::soil_tests::list_soil_tests(
                    State(state),
                    Query(ListSoilTestsQuery {
                        limit,
                        offset: None,
                    }),
                )
                .await,
            )
        }
    };
    let reply = call_tool(&state, "soil_tests", json!({})).await.json();
    assert_eq!(reply, list(Some(5)).await);
    assert_eq!(
        names(&reply, "test_date"),
        [
            "2025-07-01",
            "2025-06-01",
            "2025-05-01",
            "2025-04-01",
            "2025-03-01"
        ]
    );
    let one = call_tool(&state, "soil_tests", json!({"limit": 0}))
        .await
        .json();
    assert_eq!(names(&one, "test_date"), ["2025-07-01"]);
    let all = call_tool(&state, "soil_tests", json!({"limit": 500}))
        .await
        .json();
    assert_eq!(all, list(Some(50)).await);
    assert_eq!(all.as_array().unwrap().len(), 7);
}

// @spec MCP-TOOL-020
#[sqlx::test(migrations = "src/db/migrations")]
async fn soil_test_advice_is_the_advice_body(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "advice", 2).await;
    soil_test(&pool, lawn, date("2026-04-01"), 5.6).await;
    let reply = call_tool(&state, "soil_test_advice", json!({}))
        .await
        .json();
    let handler = body(api::soil_tests::get_soil_test_recommendations(State(state.clone())).await);
    assert_eq!(timeless(reply.clone()), timeless(handler));
    assert!(reply.is_object() && !reply.as_object().unwrap().is_empty());
}

// @spec MCP-TOOL-020
#[test]
fn soil_test_advice_is_the_only_tool_reading_soil_test_advice() {
    let sources = [
        include_str!("lawn.rs"),
        include_str!("season.rs"),
        include_str!("weather.rs"),
        include_str!("disease.rs"),
        include_str!("shelf.rs"),
    ];
    for source in sources {
        assert!(!source.contains("soil_test_recommendations"));
    }
    let log = include_str!("log.rs");
    assert_eq!(log.matches("get_soil_test_recommendations").count(), 1);
}

async fn shelf(
    state: &crate::state::AppState,
    category: Option<ProductCategory>,
    archived: bool,
) -> Value {
    body(
        api::products::list_products(
            State(state.clone()),
            Query(ListQuery {
                category,
                include_archived: archived,
            }),
        )
        .await,
    )
}

// @spec MCP-TOOL-021
#[sqlx::test(migrations = "src/db/migrations")]
async fn products_filter_by_category_ignoring_case_and_hide_archived_by_default(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "products", 2).await;
    product(&pool, lawn, "Heritage G", "Fungicide", false).await;
    product(&pool, lawn, "Old Daconil", "Fungicide", true).await;
    product(&pool, lawn, "Milorganite", "Fertilizer", false).await;

    let all = call_tool(&state, "products", json!({})).await.json();
    assert_eq!(all, shelf(&state, None, false).await);
    let mut active = names(&all, "name");
    active.sort();
    assert_eq!(active, ["Heritage G", "Milorganite"]);

    let fungicides = call_tool(&state, "products", json!({"category": "fungicide"}))
        .await
        .json();
    assert_eq!(
        fungicides,
        shelf(&state, Some(ProductCategory::Fungicide), false).await
    );
    assert_eq!(names(&fungicides, "name"), ["Heritage G"]);

    let with_archived = call_tool(
        &state,
        "products",
        json!({"category": "FUNGICIDE", "include_archived": true}),
    )
    .await
    .json();
    assert_eq!(
        with_archived,
        shelf(&state, Some(ProductCategory::Fungicide), true).await
    );
    assert_eq!(with_archived.as_array().unwrap().len(), 2);
}

// @spec MCP-TOOL-024
#[sqlx::test(migrations = "src/db/migrations")]
async fn shopping_list_is_the_shopping_list_body(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "shopping", 20).await;
    product(&pool, lawn, "Heritage G", "Fungicide", false).await;
    let reply = call_tool(&state, "shopping_list", json!({})).await.json();
    let handler = body(api::inventory::get_shopping_list(State(state.clone())).await);
    assert_eq!(timeless(reply), timeless(handler));
}

// @spec MCP-TOOL-025
#[sqlx::test(migrations = "src/db/migrations")]
async fn plants_are_the_plant_list_with_full_care_plans(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "plants", 2).await;
    plant(&pool, lawn, "Bigleaf hydrangea").await;
    let reply = call_tool(&state, "plants", json!({})).await.json();
    assert_eq!(
        reply,
        body(api::plants::list_plants(State(state.clone())).await)
    );
    let plan = &reply[0]["maintenance_plan"];
    assert_eq!(plan["tasks"][0]["description"], "Prune after flowering.");
    assert_eq!(plan["warnings"][0], "Do not prune in spring.");
}
