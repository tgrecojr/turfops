//! The 7-day summary's window ends at the instant its caller passes.

use super::fixtures::{self, SilverRow};
use chrono::{Duration, NaiveDate, TimeZone, Utc};

// @spec WX-SRC-LAKE-005
#[tokio::test]
async fn the_week_ends_at_the_callers_now() {
    // A week of hourly 10 cm soil at 10 °C (50 °F) ending Mar 8, 2026 at 00:00 UTC.
    let end = NaiveDate::from_ymd_opt(2026, 3, 8)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let rows: Vec<SilverRow> = (1..=168)
        .map(|h| SilverRow {
            at: end - Duration::hours(h),
            soil_temp_10: Some(10.0),
            air_temp_c: Some(5.0),
            rh_pct: Some(70.0),
            precip_mm: Some(0.1),
            ..Default::default()
        })
        .collect();
    let file = fixtures::silver(&rows);
    let lake = fixtures::client(&file.path, "unused.parquet");

    let then = lake
        .fetch_summary(Utc.from_utc_datetime(&end))
        .await
        .unwrap();
    assert!((then.soil_temp_7day_avg_f.unwrap() - 50.0).abs() < 1e-9);
    assert!((then.precipitation_7day_total_mm.unwrap() - 16.8).abs() < 1e-9);

    let today = lake.fetch_summary(Utc::now()).await.unwrap();
    assert_eq!(today.soil_temp_7day_avg_f, None, "that week is long past");
}
