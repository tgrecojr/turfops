//! Data sync judges ages and windows at the app clock's "now".

use super::*;
use crate::datasources::weather::fixtures::{self, SilverRow};
use chrono::Utc;
use chrono::{Duration, NaiveDate, TimeZone};

fn stopped_at(at: chrono::NaiveDateTime) -> Clock {
    Clock::Frozen {
        at: Utc.from_utc_datetime(&at),
        zone: chrono_tz::America::New_York,
    }
}

// @spec SYNC-REFRESH-009
#[tokio::test]
async fn a_frozen_clock_judges_the_week_and_the_soils_age_at_its_instant() {
    let end = NaiveDate::from_ymd_opt(2026, 3, 8)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let rows: Vec<SilverRow> = (1..=168)
        .map(|h| SilverRow {
            at: end - Duration::hours(h),
            soil_temp_10: Some(10.0),
            soil_moisture_10: Some(0.25),
            air_temp_c: Some(5.0),
            rh_pct: Some(70.0),
            precip_mm: Some(0.0),
            ..Default::default()
        })
        .collect();
    let silver = fixtures::silver(&rows);
    let lake = || Some(fixtures::client(&silver.path, "unused.parquet"));

    let mut fresh = DataSyncService::for_tests(lake(), None, stopped_at(end));
    let summary = fresh.get_or_refresh().await.unwrap();
    assert!(
        !summary.soil_stale,
        "observed an hour before the clock's now"
    );
    assert!((summary.soil_temp_7day_avg_f.unwrap() - 50.0).abs() < 1e-9);

    let mut late = DataSyncService::for_tests(lake(), None, stopped_at(end + Duration::hours(80)));
    let summary = late.get_or_refresh().await.unwrap();
    assert!(summary.soil_stale, "81 hours old at the clock's now");
}
