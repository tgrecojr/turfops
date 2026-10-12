//! The lake probe, the 7-day summary and the trend.

use super::*;
use crate::config::DataLakeConfig;
use duckdb::Connection;

// @spec RULE-GDD-002, WX-SRC-LAKE-012
#[tokio::test]
async fn gdd_ytd_runs_through_the_lawns_today_in_the_lawns_year() {
    let gold = std::env::temp_dir().join(format!("turfops_gddytd_{}.parquet", std::process::id()));
    let gold_s = gold.to_string_lossy().to_string();
    Connection::open_in_memory()
        .unwrap()
        .execute_batch(&format!(
            "COPY (
                SELECT * FROM (VALUES
                    (DATE '2025-12-30', 60.0, 40.0, 1.0, 24),
                    (DATE '2025-12-31', 60.0, 40.0, 2.0, 24),
                    (DATE '2026-01-01', 60.0, 40.0, 4.0, 24)
                ) t(day, air_temp_max_f, air_temp_min_f, gdd50, hours_observed)
             ) TO '{gold_s}' (FORMAT PARQUET)"
        ))
        .unwrap();
    let client = WeatherLakeClient::new(
        &DataLakeConfig {
            silver_weather_path: "unused.parquet".into(),
            gold_weather_path: gold_s,
        },
        3761,
    );

    // New Year's Eve evening on the lawn: still 2025, and Jan 1's row does not count.
    let total = client
        .fetch_gdd_ytd(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
        .await
        .unwrap();
    let _ = std::fs::remove_file(&gold);
    assert_eq!(total, Some(3.0));
}

fn parquet_with_a_row(name: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "turfops_probe_{name}_{}.parquet",
        std::process::id()
    ));
    let path_s = path.to_string_lossy().to_string();
    Connection::open_in_memory()
        .unwrap()
        .execute_batch(&format!(
            "COPY (SELECT 1 AS x) TO '{path_s}' (FORMAT PARQUET)"
        ))
        .unwrap();
    path_s
}

// @spec WX-VIEW-PROBE-001
#[tokio::test]
async fn the_probe_needs_both_silver_and_gold() {
    let silver = parquet_with_a_row("silver");
    let gold = parquet_with_a_row("gold");
    let client = |silver: &str, gold: &str| {
        WeatherLakeClient::new(
            &DataLakeConfig {
                silver_weather_path: silver.into(),
                gold_weather_path: gold.into(),
            },
            3761,
        )
    };
    assert!(client(&silver, &gold).test_connection().await.unwrap());
    assert!(!client("/nonexistent/silver.parquet", &gold)
        .test_connection()
        .await
        .unwrap());
    assert!(!client(&silver, "/nonexistent/gold.parquet")
        .test_connection()
        .await
        .unwrap());
    let _ = std::fs::remove_file(&silver);
    let _ = std::fs::remove_file(&gold);
}

// @spec WX-SRC-LAKE-013
#[test]
fn no_silver_rows_is_an_unknown_trend() {
    let summary = summarize_week(None, &[], Utc::now());
    assert_eq!(summary.soil_temp_trend, Trend::Unknown);
}

fn soil_hours(
    recent_f: Option<f64>,
    previous_f: Option<f64>,
    count: usize,
) -> Vec<EnvironmentalReading> {
    (0..count)
        .map(|i| {
            let mut r = EnvironmentalReading::new(DataSource::SoilData);
            r.soil_temp_10_f = if i < 24 { recent_f } else { previous_f };
            r
        })
        .collect()
}

// @spec WX-SRC-LAKE-006
#[test]
fn the_trend_compares_the_newest_24_hours_with_the_24_before() {
    assert_eq!(
        calculate_trend(&soil_hours(Some(62.1), Some(60.0), 48)),
        Trend::Rising
    );
    assert_eq!(
        calculate_trend(&soil_hours(Some(57.9), Some(60.0), 48)),
        Trend::Falling
    );
    assert_eq!(
        calculate_trend(&soil_hours(Some(62.0), Some(60.0), 48)),
        Trend::Stable
    );
    assert_eq!(
        calculate_trend(&soil_hours(Some(58.0), Some(60.0), 72)),
        Trend::Stable
    );
}

// @spec WX-SRC-LAKE-006
#[test]
fn the_trend_is_unknown_without_two_full_days_of_10cm_values() {
    assert_eq!(
        calculate_trend(&soil_hours(Some(70.0), Some(60.0), 47)),
        Trend::Unknown
    );
    assert_eq!(
        calculate_trend(&soil_hours(None, Some(60.0), 48)),
        Trend::Unknown
    );
    assert_eq!(
        calculate_trend(&soil_hours(Some(70.0), None, 48)),
        Trend::Unknown
    );
}
