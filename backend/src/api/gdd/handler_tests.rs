//! `GET /api/v1/gdd` over a gold parquet file, and without a lake.

use super::*;
use crate::clock::Clock;
use crate::logic::data_sync::DataSyncService;
use crate::logic::gdd::lake_tests::{gold_parquet, lake};
use crate::models::CrabgrassStatus;
use axum::response::IntoResponse;
use chrono::Duration;

fn state(gold: Option<&str>) -> AppState {
    // The handler never touches the database.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    let sync = DataSyncService::with_lake_only(pool.clone(), gold.map(lake), Clock::System);
    AppState::new(pool, sync, None, false, Clock::System)
}

async fn gdd(state: &AppState, year: Option<i32>) -> Result<GddSummary, TurfOpsError> {
    get_gdd(State(state.clone()), Query(GddQuery { year }))
        .await
        .map(|Json(summary)| summary)
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// @spec RULE-GDD-006
#[tokio::test]
async fn gdd_runs_through_today_this_year_and_the_whole_calendar_year_otherwise() {
    let today = Clock::System.today();
    let gold = gold_parquet(
        "endpoint",
        &[
            (day(2020, 1, 1), 40.0, 30.0, 1.0),
            (day(2020, 12, 31), 40.0, 30.0, 3.0),
            (day(2021, 1, 1), 40.0, 30.0, 50.0),
            (today, 80.0, 60.0, 20.0),
            (today + Duration::days(1), 80.0, 60.0, 400.0),
        ],
    );
    let state = state(Some(&gold));

    // A past year: Jan 1 through Dec 31, nothing from the next year.
    let past = gdd(&state, Some(2020)).await.unwrap();
    assert_eq!(past.year, 2020);
    let dates: Vec<NaiveDate> = past.daily_history.iter().map(|d| d.date).collect();
    assert_eq!(dates, vec![day(2020, 1, 1), day(2020, 12, 31)]);
    assert_eq!(past.current_gdd_total, 4.0);
    assert_eq!(past.last_computed_date, Some(day(2020, 12, 31)));
    assert_eq!(past.crabgrass_model.status, CrabgrassStatus::PreGermination);

    // The default is this year, through today only.
    let now = gdd(&state, None).await.unwrap();
    let _ = std::fs::remove_file(&gold);
    assert_eq!(now.year, today.year());
    assert_eq!(now.current_gdd_total, 20.0);
    assert_eq!(now.last_computed_date, Some(today));
    assert_eq!(now.crabgrass_model.current_gdd, 20.0);

    // A year chrono cannot represent.
    let err = gdd(&state, Some(300_000)).await.unwrap_err();
    assert_eq!(err.into_response().status(), 400);
}

// @spec RULE-GDD-007
#[tokio::test]
async fn without_a_lake_gdd_is_unavailable_not_zero() {
    let err = gdd(&state(None), None).await.unwrap_err();
    assert!(matches!(err, TurfOpsError::Unavailable { .. }), "{err:?}");
    assert_eq!(err.into_response().status(), 503);
}
