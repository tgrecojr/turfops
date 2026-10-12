//! `GET /environmental` serves data sync's cached summary; the refresh endpoint refetches.

use super::*;
use crate::datasources::weather::fixtures::{self, GoldRow, SilverRow};
use crate::logic::data_sync::test_support::{app_state, eastern};
use chrono::{Duration, Timelike, Utc};

fn soil_now(state: &EnvironmentalSummary) -> Option<f64> {
    state.current.as_ref().and_then(|c| c.soil_temp_10_f)
}

// @spec WX-VIEW-API-001
#[tokio::test]
async fn the_cached_summary_is_served_until_a_forced_refresh_refetches() {
    let at = Utc::now().naive_utc().with_nanosecond(0).unwrap() - Duration::hours(1);
    let hour = |soil_c| SilverRow {
        at,
        soil_temp_10: Some(soil_c),
        ..Default::default()
    };
    let silver = fixtures::silver(&[hour(20.0)]);
    let gold = fixtures::gold(&[GoldRow::day(eastern().today())]);
    let state = app_state(Some(fixtures::client(&silver.path, &gold.path)), None);

    let Json(first) = get_environmental(State(state.clone())).await.unwrap();
    assert_eq!(soil_now(&first), Some(68.0), "the first call refreshes");

    // The station reports a new value; within five minutes the cache is still served.
    let newer = fixtures::silver(&[hour(25.0)]);
    std::fs::copy(&newer.path, &silver.path).unwrap();
    let Json(cached) = get_environmental(State(state.clone())).await.unwrap();
    assert_eq!(soil_now(&cached), Some(68.0));
    assert_eq!(cached.last_updated, first.last_updated);

    let Json(forced) = refresh_environmental(State(state.clone())).await.unwrap();
    assert_eq!(
        soil_now(&forced),
        Some(77.0),
        "refresh refetches every source"
    );
    assert!(forced.last_updated > first.last_updated);
    let Json(after) = get_environmental(State(state)).await.unwrap();
    assert_eq!(soil_now(&after), Some(77.0));
}
