//! The season context: anomalies against the station's history and forecast alerts.

use super::*;
use crate::logic::timing::context::build_context;

fn forecast_with(lows_highs: &[(f64, f64)]) -> Vec<DailyForecast> {
    lows_highs
        .iter()
        .enumerate()
        .map(|(i, &(low, high))| DailyForecast {
            low_temp_f: low,
            high_temp_f: high,
            ..forecast_day(date(2026, 9, 21) + Duration::days(i as i64), 60.0)
        })
        .collect()
}

// @spec TIME-ENG-042
#[test]
fn the_context_compares_this_season_with_history_and_flags_forecast_extremes() {
    let years: Vec<i32> = (2020..2026).collect();
    let warm = station(date(2026, 9, 19), 3.0);
    let context = build_context(&warm, &years, &[]);
    let soil = context.soil_anomaly_7d_f.unwrap();
    assert!((soil - 3.0).abs() < 0.2, "{soil}");
    let air = context.air_anomaly_30d_f.unwrap();
    assert!((air - 3.0).abs() < 0.2, "{air}");
    // The summary prefers soil.
    let summary = context.summary.unwrap();
    assert!(
        summary.starts_with("5 cm soil over the last week"),
        "{summary}"
    );
    assert!(summary.contains("°F warmer than normal"), "{summary}");
    assert!(context.forecast_alerts.is_empty());

    // Fewer than 3 baseline years: no anomaly.
    let thin = build_context(&warm, &[2024, 2025], &[]);
    assert_eq!(thin.soil_anomaly_7d_f, None);
    assert_eq!(thin.summary, None);

    // The first forecast low at or below 32°F and the number of 90°F highs.
    let forecast = forecast_with(&[(40.0, 90.0), (32.0, 70.0), (28.0, 91.0), (45.0, 85.0)]);
    let alerts = build_context(&warm, &years, &forecast).forecast_alerts;
    assert_eq!(
        alerts,
        vec![
            "Freeze in the forecast: low of 32°F on Sep 22.".to_string(),
            "2 forecast day(s) at 90°F or hotter — new seed will need extra water.".to_string(),
        ]
    );
}

// @spec TIME-ENG-042
#[test]
fn an_anomaly_never_moves_a_windows_dates() {
    let today = date(2026, 7, 1);
    let typical_dates = |bias: f64| {
        let days = station(date(2026, 6, 30), bias);
        let result = run(today, &days, &[]);
        let context = build_context(&days, &result.history_years, &[]);
        // Dormant seeding is left out: last winter's season closes in this year's
        // data, so a warmer 2026 is a real observation for it, not an anomaly.
        let dates: Vec<_> = result
            .windows
            .iter()
            .filter(|w| w.id != WindowId::DormantSeeding)
            .flat_map(|w| [w.opens.typical.clone(), w.closes.typical.clone()])
            .collect();
        (context.soil_anomaly_7d_f.unwrap(), dates)
    };
    let (normal_anomaly, normal) = typical_dates(0.0);
    let (warm_anomaly, warm) = typical_dates(6.0);
    assert!(warm_anomaly - normal_anomaly > 5.0);
    assert_eq!(normal, warm);
}
