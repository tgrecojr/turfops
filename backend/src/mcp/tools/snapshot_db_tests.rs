//! `lawn_snapshot` composed from the handlers, whole and with parts failing.

use crate::api;
use crate::api::nitrogen_budget::NitrogenBudgetQuery;
use crate::mcp::db_support::*;
use crate::mcp::test_support::*;
use crate::mcp::trim;
use crate::models::ApplicationType;
use axum::extract::{Query, State};
use chrono::Duration;
use serde_json::{json, Value};
use sqlx::PgPool;

/// A lawn with a lake, a forecast, eight logged applications (three with a follow-up due
/// today, so the feed is longer than the dashboard's three) and its snapshot.
async fn snapshot(pool: PgPool, name: &str) -> (crate::state::AppState, Lake, Value) {
    let lawn = lawn(&pool).await;
    let lake = Lake::with_days(name, 40, &[]);
    let state = state_with_forecast(pool.clone(), &lake).await;
    let today = state.clock.today();
    for i in 0..8 {
        let kind = if i % 2 == 0 {
            ApplicationType::Mowing
        } else {
            ApplicationType::Fertilizer
        };
        let id = log(
            &pool,
            lawn,
            kind,
            today - Duration::days(i + 1),
            &format!("App {i}"),
        )
        .await;
        if i < 3 {
            sqlx::query("UPDATE applications SET follow_up_date = $1 WHERE id = $2")
                .bind(today)
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
    let reply = call_tool(&state, "lawn_snapshot", json!({})).await;
    assert!(!reply.is_error);
    (state, lake, reply.json())
}

// @spec MCP-SNAP-001
#[test]
fn the_parts_are_fetched_together_from_the_handlers_never_the_dashboard() {
    let source = include_str!("lawn.rs");
    let snapshot = source
        .split("async fn lawn_snapshot")
        .nth(1)
        .and_then(|rest| rest.split("async fn current_conditions").next())
        .unwrap();
    assert!(snapshot.contains("tokio::join!("));
    for handler in [
        "api::profile::get_profile",
        "api::environmental::get_environmental",
        "api::recommendations::list_recommendations",
        "api::applications::list_applications",
        "api::timing::get_timing_windows",
        "api::disease_risk::get_disease_risk",
        "api::nitrogen_budget::get_nitrogen_budget",
    ] {
        assert!(snapshot.contains(handler), "{handler}");
    }
    assert!(!snapshot.contains("dashboard"));
}

// @spec MCP-SNAP-001, MCP-SNAP-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_snapshot_carries_each_part_shaped_as_its_tool_would(pool: PgPool) {
    let (state, _lake, snap) = snapshot(pool, "snap-parts").await;
    let s = || State(state.clone());

    assert_eq!(snap["profile"], body(api::profile::get_profile(s()).await));

    let mut conditions = body(api::environmental::get_environmental(s()).await);
    trim::drop_hourly_forecast(&mut conditions);
    assert!(snap["conditions"]["forecast"].get("hourly").is_none());
    assert!(snap["conditions"]["forecast"]["daily_summary"].is_array());
    assert_eq!(timeless(snap["conditions"].clone()), timeless(conditions));

    let feed = call_tool(&state, "recommendations", json!({})).await.json();
    assert!(
        feed.as_array().unwrap().len() > 3,
        "the full feed, not the dashboard's 3"
    );
    assert_eq!(timeless(snap["recommendations"].clone()), timeless(feed));

    let recent = names(&snap["recent_applications"], "product_name");
    assert_eq!(recent, ["App 0", "App 1", "App 2", "App 3", "App 4"]);

    let timing = body(api::timing::get_timing_windows(s()).await);
    assert!(snap["timing_windows"].get("series").is_none());
    assert_eq!(
        timeless(snap["timing_windows"].clone()),
        timeless(trim::timing_without_series(timing))
    );
}

// @spec MCP-SNAP-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_disease_part_is_one_headline_per_disease(pool: PgPool) {
    let (state, _lake, snap) = snapshot(pool, "snap-disease").await;
    let full = body(api::disease_risk::get_disease_risk(State(state.clone())).await);
    let part = &snap["disease_risk"];
    let mut keys: Vec<&str> = part
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(keys, ["data_notes", "diseases", "today"]);
    assert_eq!(part["today"], full["today"]);
    assert_eq!(part["data_notes"], full["data_notes"]);
    let mut expected = [
        "slug",
        "name",
        "tier",
        "tier_note",
        "score_label",
        "as_of",
        "action",
        "headline",
        "protected_through",
    ];
    expected.sort();
    for (d, f) in part["diseases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(full["diseases"].as_array().unwrap())
    {
        let mut fields: Vec<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        fields.sort();
        assert_eq!(fields, expected);
        assert_eq!(d["slug"], f["slug"]);
        assert_eq!(d["tier"], f["tier"]);
        assert_eq!(d["headline"], f["management"]["headline"]);
    }
}

// @spec MCP-SNAP-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_nitrogen_part_is_this_years_totals_without_the_feedings(pool: PgPool) {
    let (state, _lake, snap) = snapshot(pool, "snap-nitrogen").await;
    let budget = body(
        api::nitrogen_budget::get_nitrogen_budget(
            State(state.clone()),
            Query(NitrogenBudgetQuery { year: None }),
        )
        .await,
    );
    assert!(budget["applications"]
        .as_array()
        .is_some_and(|a| !a.is_empty()));
    assert_eq!(snap["nitrogen_budget"], trim::nitrogen_totals(budget));
    assert!(snap["nitrogen_budget"].get("applications").is_none());
    assert!(
        snap["nitrogen_budget"]["applied_lbs_per_1000sqft"]
            .as_f64()
            .unwrap()
            > 0.0
    );
}

// @spec MCP-SNAP-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_snapshot_reports_connection_status_under_data_syncs_names(pool: PgPool) {
    let (state, _lake, snap) = snapshot(pool, "snap-sources").await;
    let status = state.sync_service.read().await.check_connections().await;
    assert_eq!(snap["data_sources"], serde_json::to_value(status).unwrap());
    assert_eq!(
        snap["data_sources"],
        json!({"soildata": true, "homeassistant": false, "openweathermap": true})
    );
}

// @spec MCP-SNAP-006, MCP-SNAP-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_lake_outage_nulls_its_parts_with_notes_and_keeps_the_rest(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, &Lake::missing("snap-outage")).await;
    let reply = call_tool(&state, "lawn_snapshot", json!({})).await;
    assert!(!reply.is_error, "the snapshot itself succeeds");
    let snap = reply.json();
    assert!(snap["timing_windows"].is_null());
    assert!(snap["disease_risk"].is_null());
    let lake = "The NOAA weather data lake could not be read, so station soil readings, \
                timing windows and disease risk are unavailable. Tell the user this data is \
                missing rather than estimating it.";
    assert_eq!(
        snap["notes"],
        json!([
            format!("timing_windows unavailable: {lake}"),
            format!("disease_risk unavailable: {lake}"),
        ])
    );
    for kept in [
        "profile",
        "conditions",
        "recommendations",
        "recent_applications",
    ] {
        assert!(!snap[kept].is_null(), "{kept}");
    }
    assert!(snap["nitrogen_budget"].is_object());
}

// @spec MCP-SNAP-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_whole_snapshot_has_an_empty_notes_list(pool: PgPool) {
    let (_state, _lake, snap) = snapshot(pool, "snap-whole").await;
    assert_eq!(snap["notes"], json!([]));
}

// @spec MCP-SNAP-006, MCP-SNAP-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn with_no_lawn_every_part_explains_its_absence(pool: PgPool) {
    let state = state(pool, &Lake::missing("snap-empty")).await;
    let reply = call_tool(&state, "lawn_snapshot", json!({})).await;
    assert!(!reply.is_error);
    let snap = reply.json();
    assert!(snap["profile"].is_null());
    let notes: Vec<&str> = snap["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    assert!(
        notes.contains(&"profile unavailable: No lawn profile found"),
        "{notes:?}"
    );
    assert!(snap["data_sources"].is_object());
}
