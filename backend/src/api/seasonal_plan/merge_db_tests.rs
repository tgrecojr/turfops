//! `GET /api/v1/seasonal-plan`: the request and response, the completion log, the timing
//! swap and the final order.

use super::db_support::*;
use crate::datasources::weather::{ClimateDay, ClimateRecord};
use crate::models::seasonal_plan::SeasonalPlan;
use crate::models::ApplicationType;
use crate::state::AppState;
use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use std::sync::Arc;

fn ids(plan: &SeasonalPlan) -> Vec<&str> {
    plan.activities.iter().map(|a| a.id.as_str()).collect()
}

fn status(plan: &SeasonalPlan, id: &str) -> String {
    let a = plan.activities.iter().find(|a| a.id == id).unwrap();
    format!("{:?}", a.status)
}

// @spec PLAN-MERGE-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_plan_defaults_to_this_year_and_carries_every_field(pool: PgPool) {
    create_profile(&pool, "Front").await;
    settled_history(&pool, STATION, this_year()).await;
    let state = state(&pool, None);

    let plan = get_plan(&state, None).await.unwrap();
    assert_eq!(plan.year, this_year());
    let later = get_plan(&state, Some(this_year() + 1)).await.unwrap();
    assert_eq!(
        later.year,
        this_year() + 1,
        "the year parameter picks the plan"
    );

    let json = serde_json::to_value(&later).unwrap();
    for key in ["year", "activities", "data_years_used", "generated_at"] {
        assert!(json.get(key).is_some(), "plan has {key}");
    }
    let activity = &json["activities"][0];
    for key in ["id", "name", "category", "description", "status"] {
        assert!(activity.get(key).is_some(), "activity has {key}");
    }
    for key in [
        "predicted_start",
        "predicted_end",
        "earliest_historical",
        "latest_historical",
        "confidence",
    ] {
        assert!(
            activity["date_window"].get(key).is_some(),
            "window has {key}"
        );
    }
    for key in ["soil_temp_trigger", "product_suggestions", "rate", "notes"] {
        assert!(activity["details"].get(key).is_some(), "details has {key}");
    }
}

// @spec PLAN-MERGE-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn no_profile_is_404_and_a_year_with_no_january_first_is_400(pool: PgPool) {
    let state = state(&pool, None);
    assert_eq!(
        get_plan(&state, Some(2025)).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    create_profile(&pool, "Front").await;
    assert_eq!(
        get_plan(&state, Some(400_000)).await.unwrap_err(),
        StatusCode::BAD_REQUEST
    );
}

// @spec PLAN-MERGE-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn completion_reads_the_default_profiles_log_for_the_plan_year(pool: PgPool) {
    let lawn = create_profile(&pool, "Front").await;
    let other = create_profile(&pool, "Back").await;
    let year = this_year() + 1;
    settled_history(&pool, STATION, year).await;
    let state = state(&pool, None);
    let aerated = |profile, on| application(profile, ApplicationType::Aeration, on);

    log(&pool, aerated(lawn, d(year - 1, 12, 31))).await;
    log(&pool, aerated(other, d(year, 10, 1))).await;
    let plan = get_plan(&state, Some(year)).await.unwrap();
    assert_eq!(
        status(&plan, "core_aeration"),
        "Upcoming",
        "last year / other lawn"
    );

    log(&pool, aerated(lawn, d(year, 1, 1))).await;
    let plan = get_plan(&state, Some(year)).await.unwrap();
    assert_eq!(status(&plan, "core_aeration"), "Completed", "Jan 1 counts");

    sqlx::query("DELETE FROM applications")
        .execute(&pool)
        .await
        .unwrap();
    log(&pool, aerated(lawn, d(year, 12, 31))).await;
    let plan = get_plan(&state, Some(year)).await.unwrap();
    assert_eq!(status(&plan, "core_aeration"), "Completed", "Dec 31 counts");
}

// @spec PLAN-MERGE-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_timing_windows_every_10cm_activity_stays(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let year = this_year() + 1;
    settled_history(&pool, STATION, year).await;
    // A lake whose parquet is missing: the timing windows cannot be dated.
    let dir = scratch("no-timing");
    let state = state(&pool, Some(lake(&dir.join("missing.parquet"))));

    let plan = get_plan(&state, Some(year)).await.unwrap();
    let ids = ids(&plan);
    assert_eq!(ids.len(), 10, "{ids:?}");
    for id in ["pre_emergent", "core_aeration", "fall_overseeding"] {
        assert!(ids.contains(&id), "{id}");
    }
    assert!(!ids.contains(&"fall_pre_emergent"));
}

// @spec PLAN-MERGE-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn activities_come_back_in_start_order_ties_kept_in_place(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let year = this_year() + 1;
    settled_history(&pool, STATION, year).await;
    let plan = get_plan(&state(&pool, None), Some(year)).await.unwrap();

    let starts: Vec<NaiveDate> = plan
        .activities
        .iter()
        .map(|a| a.date_window.predicted_start)
        .collect();
    assert!(starts.windows(2).all(|w| w[0] <= w[1]), "{starts:?}");
    let opening_sep_25: Vec<&str> = plan
        .activities
        .iter()
        .filter(|a| a.date_window.predicted_start == d(year, 9, 25))
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(
        opening_sep_25,
        [
            "core_aeration",
            "fall_overseeding",
            "early_fall_fert",
            "fall_herbicide"
        ],
        "same-day activities keep the builder's order"
    );
}

/// A station record from eight years back through today on one cosine soil curve (5 cm
/// peak 77°F mid-July, trough 33°F mid-January), air tracking soil.
fn climate(today: NaiveDate) -> ClimateRecord {
    let days = d(today.year() - 8, 1, 1)
        .iter_days()
        .take_while(|day| *day <= today)
        .map(|date| {
            let phase = 2.0 * std::f64::consts::PI * (date.ordinal() as f64 - 200.0) / 365.0;
            let soil = 55.0 + 22.0 * phase.cos();
            ClimateDay {
                date,
                soil_temp_5_f: Some(soil),
                air_min_f: Some(soil - 16.5),
                air_avg_f: Some(soil - 5.0),
                gdd50: Some((soil - 55.0).max(0.0)),
            }
        })
        .collect();
    ClimateRecord { days }
}

async fn with_timing(state: &AppState) {
    let record = Arc::new(climate(clock().today()));
    *state.climate_cache.write().await = Some((Utc::now(), record));
}

// @spec PLAN-MERGE-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn timing_activities_replace_their_10cm_twins(pool: PgPool) {
    let lawn = create_profile(&pool, "Front").await;
    let year = this_year() - 1;
    settled_history(&pool, STATION, year).await;
    let dir = scratch("timing");
    let state = state(&pool, Some(lake(&dir.join("missing.parquet"))));
    with_timing(&state).await;

    let plan = get_plan(&state, Some(year)).await.unwrap();
    let swapped = ids(&plan);
    for id in ["pre_emergent", "fall_overseeding", "core_aeration"] {
        assert_eq!(swapped.iter().filter(|i| **i == id).count(), 1, "{id} once");
    }
    assert!(swapped.contains(&"fall_pre_emergent"), "{swapped:?}");
    let pre = plan
        .activities
        .iter()
        .find(|a| a.id == "pre_emergent")
        .unwrap();
    let trigger = pre.details.soil_temp_trigger.as_deref().unwrap();
    assert!(trigger.contains("5 cm"), "the timing version: {trigger}");
    assert!(
        swapped.contains(&"spring_nitrogen"),
        "10 cm-only activities stay"
    );

    // A fall pre-emergent rules out fall seeding: neither version comes back.
    let pre_em = application(lawn, ApplicationType::PreEmergent, d(year, 8, 20));
    log(&pool, pre_em).await;
    let plan = get_plan(&state, Some(year)).await.unwrap();
    let blocked = ids(&plan);
    assert!(!blocked.contains(&"fall_overseeding"), "{blocked:?}");
    assert!(!blocked.contains(&"core_aeration"), "{blocked:?}");
}
