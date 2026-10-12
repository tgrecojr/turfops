//! Which weather sources data sync builds, and the summary it publishes from them.

use super::test_support::{eastern, forecast, forecast_day, lazy_pool};
use super::*;
use crate::datasources::weather::fixtures::{self, GoldRow, SilverRow};
use crate::models::Trend;
use chrono::Utc;
use chrono::{Duration, Timelike};
use std::collections::HashMap;

async fn initialized(pairs: &[(&'static str, &str)]) -> DataSyncService {
    let mut vars: HashMap<&'static str, String> =
        HashMap::from([("DATABASE_PASSWORD", "pw".to_string())]);
    for (k, v) in pairs {
        vars.insert(k, v.to_string());
    }
    let (config, _) = crate::config::load::from_vars(&|key| vars.get(key).cloned()).unwrap();
    DataSyncService::initialize(&config, lazy_pool()).await
}

const OWM: [(&str, &str); 3] = [
    ("OWM_API_KEY", "key"),
    ("OWM_LATITUDE", "39.8"),
    ("OWM_LONGITUDE", "-75.8"),
];

// @spec WX-SRC-CFG-002
#[tokio::test]
async fn home_assistant_needs_a_token_and_the_forecast_a_key_and_the_flag() {
    let none = initialized(&[]).await;
    assert!(
        none.weather_client().is_some(),
        "the lake client is always built"
    );
    assert!(!none.has_home_assistant());
    assert!(!none.has_forecast_client());

    assert!(!initialized(&[("HA_TOKEN", "")]).await.has_home_assistant());
    assert!(initialized(&[("HA_TOKEN", "t")]).await.has_home_assistant());

    assert!(
        initialized(&OWM).await.has_forecast_client(),
        "enabled by default"
    );
    let blank_key = [("OWM_API_KEY", "  "), OWM[1], OWM[2]];
    assert!(!initialized(&blank_key).await.has_forecast_client());
    let off = [OWM[0], OWM[1], OWM[2], ("OWM_ENABLED", "false")];
    assert!(!initialized(&off).await.has_forecast_client());
    let on = [OWM[0], OWM[1], OWM[2], ("OWM_ENABLED", "true")];
    assert!(initialized(&on).await.has_forecast_client());
}

// @spec WX-SRC-MODEL-001
#[tokio::test]
async fn the_summary_publishes_lake_soil_patio_air_week_stamp_forecast_and_gdd() {
    let now = Utc::now().naive_utc().with_nanosecond(0).unwrap();
    let newest = now - Duration::hours(2);
    let rows: Vec<SilverRow> = (0..48)
        .map(|h| SilverRow {
            at: newest - Duration::hours(h),
            soil_temp_10: Some(if h < 24 { 20.0 } else { 15.0 }),
            soil_moisture_10: Some(0.3),
            air_temp_c: Some(30.0),
            rh_pct: Some(70.0),
            precip_mm: Some(if h == 0 { 1.5 } else { 0.0 }),
            ..Default::default()
        })
        .collect();
    let silver = fixtures::silver(&rows);
    let today = eastern().today();
    let gold = fixtures::gold(&[GoldRow {
        gdd50: Some(5.0),
        ..GoldRow::day(today)
    }]);
    let mut service = DataSyncService::for_tests(
        Some(fixtures::client(&silver.path, &gold.path)),
        Some(forecast(vec![forecast_day(today, 24, 80.0, 60.0)])),
        eastern(),
    );

    let before = Utc::now();
    let s = service.get_or_refresh().await.unwrap();
    let current = s.current.as_ref().unwrap();
    assert_eq!(current.soil_temp_10_f, Some(68.0));
    assert_eq!(current.soil_moisture_10, Some(0.3));
    assert_eq!(
        current.precipitation_mm,
        Some(1.5),
        "the newest hour's rain"
    );
    assert_eq!(
        (current.ambient_temp_f, current.humidity_percent),
        (None, None),
        "live air and RH come from Home Assistant only, never the station"
    );
    assert!((s.soil_temp_7day_avg_f.unwrap() - 63.5).abs() < 1e-9);
    assert_eq!(s.ambient_temp_7day_avg_f, Some(86.0));
    assert_eq!(s.humidity_7day_avg, Some(70.0));
    assert_eq!(s.precipitation_7day_total_mm, Some(1.5));
    assert_eq!(s.soil_temp_trend, Trend::Rising);
    assert_eq!(s.soil_observed_at, Some(newest.and_utc()));
    assert!(
        s.last_updated.unwrap() >= before,
        "when data sync refreshed"
    );
    assert_eq!(s.forecast.as_ref().unwrap().daily_summary[0].date, today);
    assert_eq!(s.gdd_base50_ytd, Some(5.0));
}
