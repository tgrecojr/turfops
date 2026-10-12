//! `GET /soil-temp-forecast` over test parquet and a stored forecast.

use super::*;
use crate::datasources::weather::fixtures::{self, GoldRow, SilverRow, TempParquet};
use crate::logic::data_sync::test_support::{app_state, eastern, forecast, forecast_day};
use crate::models::{DailyForecast, WeatherForecast};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{NaiveDate, Timelike};

/// Gold pairs on `days_back` days before today (soil = air + 2°F, so lag 0 fits
/// perfectly), plus five wild pairs 35-40 days back that the 30-day window must skip.
fn lake(days_back: i64) -> (TempParquet, TempParquet) {
    let today = eastern().today();
    let air = |i: i64| 60.0 + 10.0 * ((i as f64) * 0.9).sin();
    let mut rows: Vec<GoldRow> = (1..=days_back)
        .map(|i| GoldRow {
            air_avg_f: Some(air(i)),
            soil_10_f: Some(air(i) + 2.0),
            ..GoldRow::day(today - Duration::days(i))
        })
        .collect();
    rows.extend((35..40).map(|i| GoldRow {
        air_avg_f: Some(40.0),
        soil_10_f: Some(200.0 + i as f64),
        ..GoldRow::day(today - Duration::days(i))
    }));
    let now = Utc::now().naive_utc().with_nanosecond(0).unwrap();
    let silver = fixtures::silver(&[SilverRow {
        at: now - Duration::hours(2),
        soil_temp_10: Some(20.0),
        air_temp_c: Some(20.0),
        rh_pct: Some(60.0),
        ..Default::default()
    }]);
    (silver, fixtures::gold(&rows))
}

fn day(offset: i64, hours: u32, high: f64, low: f64) -> DailyForecast {
    forecast_day(eastern().today() + Duration::days(offset), hours, high, low)
}

/// Three full days, then a six-hour partial day.
fn three_full_days() -> WeatherForecast {
    forecast(vec![
        day(1, 24, 80.0, 60.0),
        day(2, 21, 84.0, 64.0),
        day(3, 18, 70.0, 50.0),
        day(4, 6, 100.0, 90.0),
    ])
}

async fn outlook(
    lake: Option<&(TempParquet, TempParquet)>,
    forecast: Option<WeatherForecast>,
) -> Result<SoilTempForecast, StatusCode> {
    let client = lake.map(|(silver, gold)| fixtures::client(&silver.path, &gold.path));
    get_soil_temp_forecast(State(app_state(client, forecast)))
        .await
        .map(|Json(f)| f)
        .map_err(|e| e.into_response().status())
}

// @spec WX-OUTLOOK-API-001
#[tokio::test]
async fn the_outlook_returns_predictions_crossings_model_info_and_its_time() {
    let lake = lake(20);
    let before = Utc::now();
    let f = outlook(Some(&lake), Some(three_full_days())).await.unwrap();
    assert!(!f.predictions.is_empty());
    let heat = f
        .threshold_crossings
        .iter()
        .find(|c| c.threshold_temp_f == 75.0)
        .expect("68°F now, 76°F in two days");
    assert_eq!(heat.days_until_crossing, 2);
    assert!(f.generated_at >= before);

    let json = serde_json::to_value(&f).unwrap();
    let info = json["model_info"].as_object().unwrap();
    let mut keys: Vec<&str> = info.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(
        keys,
        ["lag_days", "quality", "r_squared", "training_window_days"]
    );
    assert_eq!(info["quality"], "Good");
    for key in ["predictions", "threshold_crossings", "generated_at"] {
        assert!(json.get(key).is_some(), "{key}");
    }
}

// @spec WX-OUTLOOK-MODEL-004
#[tokio::test]
async fn the_model_trains_on_the_last_30_days_and_needs_14_pairs() {
    let enough = lake(14);
    let f = outlook(Some(&enough), Some(three_full_days()))
        .await
        .unwrap();
    assert_eq!(
        f.model_info.training_window_days, 14,
        "older pairs are not used"
    );
    assert_eq!(f.model_info.lag_days, 0);

    let thin = lake(13);
    assert_eq!(
        outlook(Some(&thin), Some(three_full_days()))
            .await
            .unwrap_err(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

// @spec WX-OUTLOOK-MODEL-005
#[tokio::test]
async fn forecast_days_drive_with_their_mean_and_partial_days_are_left_out() {
    let lake = lake(20);
    let f = outlook(Some(&lake), Some(three_full_days())).await.unwrap();
    let on = |offset: i64| -> Option<(f64, f64)> {
        let date: NaiveDate = eastern().today() + Duration::days(offset);
        f.predictions
            .iter()
            .find(|p| p.date == date)
            .map(|p| (p.air_temp_used_f, p.predicted_soil_temp_f))
    };
    assert_eq!(on(1), Some((70.0, 72.0)), "(80 + 60) / 2 drives day 1");
    assert_eq!(on(2), Some((74.0, 76.0)));
    assert_eq!(on(3), Some((60.0, 62.0)));
    assert_eq!(on(4), None, "the six-hour day has no mean to drive it");
}

// @spec WX-OUTLOOK-API-002
#[tokio::test]
async fn no_forecast_or_no_lake_client_is_a_503() {
    let lake = lake(20);
    assert_eq!(
        outlook(Some(&lake), None).await.unwrap_err(),
        StatusCode::SERVICE_UNAVAILABLE,
        "no forecast"
    );
    let partial_only = forecast(vec![day(1, 12, 80.0, 60.0)]);
    assert_eq!(
        outlook(Some(&lake), Some(partial_only)).await.unwrap_err(),
        StatusCode::SERVICE_UNAVAILABLE,
        "no full forecast day"
    );
    assert_eq!(
        outlook(None, Some(three_full_days())).await.unwrap_err(),
        StatusCode::SERVICE_UNAVAILABLE,
        "no lake client"
    );
}
