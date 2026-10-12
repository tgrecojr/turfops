//! Conditions, the season and disease risk: each tool's reply against its handler's body.

use crate::api;
use crate::api::gdd::GddQuery;
use crate::api::nitrogen_budget::NitrogenBudgetQuery;
use crate::api::seasonal_plan::SeasonalPlanQuery;
use crate::mcp::db_support::*;
use crate::mcp::test_support::*;
use crate::mcp::trim;
use crate::models::ApplicationType;
use axum::extract::{Query, State};
use serde_json::json;
use sqlx::PgPool;

// @spec MCP-TOOL-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn current_conditions_is_the_environmental_body_without_the_hourly_forecast(pool: PgPool) {
    lawn(&pool).await;
    let lake = Lake::with_days("conditions", 10, &[]);
    let state = state_with_forecast(pool, &lake).await;
    let reply = call_tool(&state, "current_conditions", json!({}))
        .await
        .json();
    let full = body(api::environmental::get_environmental(State(state.clone())).await);
    assert!(full["forecast"]["hourly"]
        .as_array()
        .is_some_and(|h| !h.is_empty()));
    assert!(
        reply["forecast"].get("hourly").is_none(),
        "the 3-hourly list is dropped"
    );
    assert!(
        reply["forecast"]["daily_summary"]
            .as_array()
            .is_some_and(|d| d.len() >= 5),
        "the daily summaries are kept"
    );
    let mut expected = full;
    trim::drop_hourly_forecast(&mut expected);
    assert_eq!(timeless(reply.clone()), timeless(expected));
    assert!(
        reply["current"].is_object(),
        "the lake's reading is relayed"
    );
}

// @spec MCP-TOOL-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn timing_windows_drop_the_soil_series_unless_detail_is_asked(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "timing", 40).await;
    let full = timeless(body(
        api::timing::get_timing_windows(State(state.clone())).await,
    ));
    assert!(full.get("series").is_some());

    let brief = call_tool(&state, "timing_windows", json!({})).await.json();
    assert!(brief.get("series").is_none());
    assert_eq!(timeless(brief), trim::timing_without_series(full.clone()));

    let detail = call_tool(&state, "timing_windows", json!({"detail": true}))
        .await
        .json();
    assert_eq!(timeless(detail), full);
}

// @spec MCP-TOOL-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn seasonal_plan_is_the_plan_body_for_the_year_asked(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "plan", 10).await;
    let this_year = call_tool(&state, "seasonal_plan", json!({})).await.json();
    let handler = body(
        api::seasonal_plan::get_seasonal_plan(
            State(state.clone()),
            Query(SeasonalPlanQuery { year: None }),
        )
        .await,
    );
    assert_eq!(timeless(this_year), timeless(handler));

    let next = call_tool(&state, "seasonal_plan", json!({"year": 2030}))
        .await
        .json();
    let handler = body(
        api::seasonal_plan::get_seasonal_plan(
            State(state.clone()),
            Query(SeasonalPlanQuery { year: Some(2030) }),
        )
        .await,
    );
    assert_eq!(next["year"], 2030);
    assert_eq!(timeless(next), timeless(handler));
}

// @spec MCP-TOOL-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn gdd_is_the_gdd_body_for_the_year_without_daily_history(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "gdd", 10).await;
    let year = state
        .clock
        .today()
        .format("%Y")
        .to_string()
        .parse::<i32>()
        .unwrap();
    let reply = call_tool(&state, "gdd", json!({"year": year})).await.json();
    let handler =
        body(api::gdd::get_gdd(State(state.clone()), Query(GddQuery { year: Some(year) })).await);
    assert!(handler["daily_history"]
        .as_array()
        .is_some_and(|d| !d.is_empty()));
    assert!(reply.get("daily_history").is_none());
    assert_eq!(reply, trim::without(handler, &["daily_history"]));
}

// @spec MCP-TOOL-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn soil_temp_outlook_is_the_soil_forecast_body(pool: PgPool) {
    lawn(&pool).await;
    let lake = Lake::with_days("outlook", 40, &[]);
    let state = state_with_forecast(pool, &lake).await;
    let reply = call_tool(&state, "soil_temp_outlook", json!({}))
        .await
        .json();
    let handler =
        body(api::soil_temp_prediction::get_soil_temp_forecast(State(state.clone())).await);
    assert_eq!(timeless(reply), timeless(handler));
}

// @spec MCP-TOOL-011
#[sqlx::test(migrations = "src/db/migrations")]
async fn nitrogen_budget_is_the_budget_body_with_each_feeding(pool: PgPool) {
    let (state, _lake, lawn) = lawn_with_lake(pool.clone(), "nitrogen", 2).await;
    log(
        &pool,
        lawn,
        ApplicationType::Fertilizer,
        date("2026-04-10"),
        "Spring feed",
    )
    .await;
    log(
        &pool,
        lawn,
        ApplicationType::Fertilizer,
        date("2025-09-10"),
        "Fall feed",
    )
    .await;
    let reply = call_tool(&state, "nitrogen_budget", json!({"year": 2026}))
        .await
        .json();
    let handler = body(
        api::nitrogen_budget::get_nitrogen_budget(
            State(state.clone()),
            Query(NitrogenBudgetQuery { year: Some(2026) }),
        )
        .await,
    );
    assert_eq!(reply, handler);
    assert_eq!(
        names(&reply["applications"], "product_name"),
        ["Spring feed"]
    );
}

/// The fields `disease_risk` keeps per disease.
const OVERVIEW_FIELDS: [&str; 13] = [
    "slug",
    "name",
    "tier",
    "tier_note",
    "score_label",
    "as_of",
    "summary",
    "factors",
    "action",
    "headline",
    "protection",
    "notes",
    "recommended_preventative",
];

// @spec MCP-TOOL-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn disease_risk_keeps_the_assessment_and_the_recommended_preventative(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "disease", 20).await;
    let full = body(api::disease_risk::get_disease_risk(State(state.clone())).await);
    let reply = call_tool(&state, "disease_risk", json!({})).await.json();
    let mut keys: Vec<&str> = reply
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, ["data_notes", "diseases", "today"]);
    assert_eq!(reply["today"], full["today"]);
    assert_eq!(reply["data_notes"], full["data_notes"]);
    let diseases = reply["diseases"].as_array().unwrap();
    assert_eq!(diseases.len(), full["diseases"].as_array().unwrap().len());
    for (d, f) in diseases.iter().zip(full["diseases"].as_array().unwrap()) {
        let mut fields: Vec<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        fields.sort();
        let mut expected = OVERVIEW_FIELDS.to_vec();
        expected.sort();
        assert_eq!(fields, expected, "{}", d["slug"]);
        assert_eq!(d["tier"], f["tier"]);
        assert_eq!(d["factors"], f["factors"]);
        assert_eq!(d["action"], f["management"]["action"]);
        let recommended = f["management"]["preventative"]["options"]
            .as_array()
            .and_then(|o| o.iter().find(|o| o["recommended"] == true))
            .cloned()
            .unwrap_or_default();
        assert_eq!(d["recommended_preventative"], recommended);
    }
}

// @spec MCP-TOOL-013
#[sqlx::test(migrations = "src/db/migrations")]
async fn disease_detail_normalizes_the_slug_and_returns_the_whole_entry(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "detail", 20).await;
    let full = body(api::disease_risk::get_disease_risk(State(state.clone())).await);
    let entry = full["diseases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["slug"] == "brown-patch")
        .unwrap()
        .clone();
    for slug in ["brown-patch", "  Brown Patch ", "BROWN_PATCH"] {
        let reply = call_tool(&state, "disease_detail", json!({"slug": slug}))
            .await
            .json();
        assert_eq!(reply["today"], full["today"], "{slug}");
        assert_eq!(reply["data_notes"], full["data_notes"], "{slug}");
        assert_eq!(reply["disease"], entry, "{slug}");
    }
    for part in ["daily", "factors", "methodology"] {
        assert!(entry.get(part).is_some(), "{part}");
    }
    let management = &entry["management"];
    assert!(management["cultural"].is_array());
    assert!(management["preventative"]["options"].is_array());
    assert!(management["curative"]["options"].is_array());
}

// @spec MCP-TOOL-014
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_unknown_disease_lists_the_slugs_there_are(pool: PgPool) {
    let (state, _lake, _) = lawn_with_lake(pool, "unknown", 20).await;
    let full = body(api::disease_risk::get_disease_risk(State(state.clone())).await);
    let slugs = names(&full["diseases"], "slug").join(", ");
    let reply = call_tool(&state, "disease_detail", json!({"slug": "Snow Mold"})).await;
    assert!(reply.is_error);
    assert_eq!(
        reply.text,
        format!("Unknown disease `Snow Mold`. Use one of: {slugs}")
    );
    assert!(slugs.contains("brown-patch") && slugs.contains("red-thread"));
}
