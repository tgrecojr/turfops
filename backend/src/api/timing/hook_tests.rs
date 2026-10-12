//! The timing windows' hooks into the recommendation feed and the seasonal plan.

use super::db_harness::{app_state, forecast_body, lawn, Lake, ZONE};
use super::*;
use crate::models::seasonal_plan::ActivityStatus;
use crate::models::timing::{WindowId, WindowState};
use crate::models::{RecommendationCategory, Severity};
use crate::test_support::stub_http;
use sqlx::PgPool;

fn fall_herbicide() -> Recommendation {
    Recommendation::new(
        "broadleaf_fall",
        RecommendationCategory::Herbicide,
        Severity::Warning,
        "Fall Broadleaf Herbicide",
        "Best window of the year.",
    )
}

// @spec TIME-FEED-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_unreadable_lake_leaves_the_feed_as_the_rules_made_it(pool: PgPool) {
    let profile = lawn(&pool).await;
    let lake = Lake::missing();
    let state = app_state(&pool, &lake, None).await;
    let mut recs = vec![fall_herbicide()];
    apply_to_feed(&state, &profile, &[], &mut recs).await;
    assert_eq!(recs.len(), 1, "no timing or aeration recommendation added");
    assert_eq!(recs[0].severity, Severity::Warning, "not reconciled");
    assert_eq!(recs[0].description, "Best window of the year.");
}

// @spec TIME-PLAN-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_the_lake_or_its_history_the_plan_keeps_its_own_activities(pool: PgPool) {
    let profile = lawn(&pool).await;
    let year = ZONE.today().year();

    let missing = Lake::missing();
    let state = app_state(&pool, &missing, None).await;
    assert!(plan_activities(&state, &profile, year).await.is_none());

    // Only this year on record: no typical dates, so no activities.
    let no_history = Lake::with_years(0);
    let state = app_state(&pool, &no_history, None).await;
    assert!(plan_activities(&state, &profile, year).await.is_none());

    let lake = Lake::with_years(6);
    let state = app_state(&pool, &lake, None).await;
    assert!(plan_activities(&state, &profile, year).await.is_some());
}

fn activity_id(id: WindowId) -> Option<&'static str> {
    match id {
        WindowId::SpringPreEmergent => Some("pre_emergent"),
        WindowId::FallPreEmergent => Some("fall_pre_emergent"),
        WindowId::FallSeeding => Some("fall_overseeding"),
        _ => None,
    }
}

fn status_for(state: WindowState) -> Option<ActivityStatus> {
    match state {
        WindowState::NotYet | WindowState::OpeningSoon => Some(ActivityStatus::Upcoming),
        WindowState::Open | WindowState::Ideal | WindowState::Closing => {
            Some(ActivityStatus::Active)
        }
        WindowState::Closed => Some(ActivityStatus::Missed),
        WindowState::Done => Some(ActivityStatus::Completed),
        WindowState::Blocked => None,
    }
}

// @spec TIME-PLAN-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_plan_reads_the_windows_as_the_timing_page_does(pool: PgPool) {
    let profile = lawn(&pool).await;
    let lake = Lake::with_years(6);
    // A full week of forecast, so the soil outlook runs.
    let week: Vec<(i64, Vec<f64>)> = (0..7)
        .map(|d| (d, vec![50.0, 48.0, 52.0, 60.0, 66.0, 64.0, 58.0, 54.0]))
        .collect();
    let base = stub_http(200, forecast_body(&week)).await;
    let state = app_state(&pool, &lake, Some(&base)).await;

    let page = compute(&state, &profile).await.unwrap();
    assert!(
        page.series.iter().any(|d| d.forecast.is_some()),
        "the page has a soil outlook"
    );
    let year = page.today.year();
    let plan = plan_activities(&state, &profile, year).await.unwrap();

    let mut compared = 0;
    for window in page.windows.iter().filter(|w| w.season_year == year) {
        let Some(id) = activity_id(window.id) else {
            continue;
        };
        let status = plan.iter().find(|a| a.id == id).map(|a| a.status);
        assert_eq!(status, status_for(window.state), "{id}: {:?}", window.state);
        compared += 1;
    }
    assert!(compared >= 2, "the fall windows are always this year's");
}
