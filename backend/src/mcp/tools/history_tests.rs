//! `weather_history` end to end over a small silver / gold lake.

use crate::mcp::test_support::*;
use chrono::Duration;
use serde_json::{json, Value};

fn today() -> chrono::NaiveDate {
    crate::clock::Clock::System.today()
}

fn day(offset: i64) -> String {
    (today() - Duration::days(offset)).to_string()
}

// @spec MCP-HIST-001
#[tokio::test]
async fn the_window_is_thirty_days_by_default_and_clamped_to_one_through_120() {
    let lake = Lake::with_days("hist-window", 2, &[]);
    let state = state(unreachable_pool(), &lake).await;
    for (days, back) in [
        (json!(null), 30),
        (json!(7), 7),
        (json!(0), 1),
        (json!(-5), 1),
        (json!(500), 120),
    ] {
        let args = if days.is_null() {
            json!({})
        } else {
            json!({"days": days})
        };
        let history = call_tool(&state, "weather_history", args).await.json();
        assert_eq!(history["start"], day(back), "days = {days}");
    }
}

/// Five full days ending yesterday in both layers, a gold-only day before them and a
/// gold-only day older than the window.
async fn five_days() -> (Lake, Value) {
    let old = today() - Duration::days(200);
    let lone = today() - Duration::days(7);
    let lake = Lake::with_days("hist-rows", 5, &[old, lone]);
    let state = state(unreachable_pool(), &lake).await;
    let history = call_tool(&state, "weather_history", json!({"days": 30}))
        .await
        .json();
    (lake, history)
}

// @spec MCP-HIST-003
#[tokio::test]
async fn one_row_per_date_either_source_has_from_start_in_ascending_order() {
    let (_lake, history) = five_days().await;
    let dates: Vec<&str> = history["days"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["date"].as_str().unwrap())
        .collect();
    let expected = [day(7), day(5), day(4), day(3), day(2), day(1)];
    assert_eq!(dates, expected);
}

// @spec MCP-HIST-003
#[test]
fn the_rows_never_come_from_the_historical_chart_endpoint() {
    let source = include_str!("weather.rs");
    let code = source.split("#[cfg(test)]").next().unwrap();
    assert!(!code.contains("historical::"), "uses /historical");
    assert!(code.contains("fetch_daily_disease_inputs(start)"));
    assert!(code.contains("api::timing::climate_record("));
}

// @spec MCP-HIST-004
#[tokio::test]
async fn each_row_carries_air_humidity_rain_hours_soil_and_gdd() {
    let (_lake, history) = five_days().await;
    let row = &history["days"][1];
    assert_eq!(
        row,
        &json!({
            "date": day(5),
            "air_mean_f": 68.0,
            "air_min_f": 66.2,
            "air_max_f": 69.8,
            "rh_mean_pct": 70.0,
            "rain_in": 0.02,
            "hours_observed": 24.0,
            "soil_5cm_f": 64.4,
            "gdd50": 10.0,
        })
    );
}

// @spec MCP-HIST-004, MCP-HIST-005
#[tokio::test]
async fn a_day_silver_lacks_takes_its_air_from_gold_and_leaves_the_rest_null() {
    let (_lake, history) = five_days().await;
    let row = &history["days"][0];
    assert_eq!(
        row,
        &json!({
            "date": day(7),
            "air_mean_f": 51.0,
            "air_min_f": 41.0,
            "air_max_f": null,
            "rh_mean_pct": null,
            "rain_in": null,
            "hours_observed": null,
            "soil_5cm_f": null,
            "gdd50": 1.0,
        })
    );
}

// @spec MCP-HIST-006
#[tokio::test]
async fn the_result_totals_rain_and_gdd_and_explains_partial_days() {
    let (_lake, history) = five_days().await;
    assert_eq!(history["rain_total_in"], 0.1);
    assert_eq!(history["gdd50_total"], 51.0);
    let note = history["note"].as_str().unwrap();
    assert!(
        note.contains("A day with fewer than 24 hours observed is partial"),
        "{note}"
    );
}

// @spec MCP-HIST-007
#[tokio::test]
async fn with_no_lake_configured_the_tool_says_so_and_how_to_fix_it() {
    let pool = unreachable_pool();
    let sync = crate::logic::data_sync::DataSyncService::with_lake_only(
        pool.clone(),
        None,
        crate::clock::Clock::System,
    );
    let state = crate::state::AppState::new(pool, sync, None, true, crate::clock::Clock::System);
    let reply = call_tool(&state, "weather_history", json!({})).await;
    assert!(reply.is_error);
    assert_eq!(
        reply.text,
        format!(
            "Data source unavailable: The weather data lake isn't configured, so soil, disease, \
             timing and history data are unavailable. Set DATALAKE_ROOT and restart TurfOps. {}",
            crate::mcp::tools::MISSING_DATA_HINT
        )
    );
}
