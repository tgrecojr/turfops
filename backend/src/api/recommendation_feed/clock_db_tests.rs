//! The feed on a stopped clock (`--features db-tests`).

use super::fixtures::{application, state_without_lake};
use crate::api::timing::db_harness::{app_state_on, lawn, Lake};
use crate::clock::Clock;
use crate::db::queries;
use crate::models::{ApplicationType, GrassType, LawnProfile, Recommendation, Severity};
use crate::state::AppState;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;

fn stopped_on(day: NaiveDate) -> Clock {
    Clock::Frozen {
        at: Utc.from_utc_datetime(&day.and_hms_opt(16, 0, 0).unwrap()),
        zone: chrono_tz::America::New_York,
    }
}

async fn feed(state: &AppState, profile: &LawnProfile) -> Vec<Recommendation> {
    let summary = state
        .sync_service
        .write()
        .await
        .get_or_refresh()
        .await
        .unwrap();
    super::all(state, profile, &summary).await.unwrap()
}

// @spec FEED-PIPE-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_feed_judges_every_date_on_the_clocks_one_reading(pool: PgPool) {
    let due = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let seed = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    let profile_id = queries::create_lawn_profile(&pool, &seed).await.unwrap();
    let profile = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    let logged = application(
        profile_id,
        ApplicationType::Fungicide,
        due - Duration::days(21),
        Some(due),
    );
    let id = queries::create_application(&pool, &logged).await.unwrap();
    let reminder = format!("application_followup_{id}");

    let on_the_day = state_without_lake(pool.clone(), stopped_on(due)).await;
    assert!(feed(&on_the_day, &profile)
        .await
        .iter()
        .any(|r| r.id == reminder));

    let month_before = state_without_lake(pool, stopped_on(due - Duration::days(30))).await;
    assert!(!feed(&month_before, &profile)
        .await
        .iter()
        .any(|r| r.id == reminder));
}

// @spec FEED-PIPE-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn timing_rewrites_the_rules_fall_broadleaf_after_they_run(pool: PgPool) {
    // Sep 25: the rules' fall broadleaf window (Sep 20 – Oct 31, soil 50–65 °F) is open
    // and the fall seeding window is still usable, so timing must find the rules' alert
    // already in the feed and turn it into the "only if NOT seeding" Advisory.
    let day = NaiveDate::from_ymd_opt(2025, 9, 25).unwrap();
    let clock = stopped_on(day);
    let lake = Lake::full_until(6, clock.now());
    let state = app_state_on(&pool, &lake, clock).await;
    let profile = lawn(&pool).await;

    let feed = feed(&state, &profile).await;
    let broadleaf = feed
        .iter()
        .find(|r| r.id == "broadleaf_fall")
        .unwrap_or_else(|| {
            panic!(
                "no broadleaf_fall in {:?}",
                feed.iter().map(|r| &r.id).collect::<Vec<_>>()
            )
        });
    assert_eq!(broadleaf.severity, Severity::Advisory);
    assert!(
        broadleaf
            .description
            .starts_with("Only if you are NOT seeding this fall."),
        "{}",
        broadleaf.description
    );
}
