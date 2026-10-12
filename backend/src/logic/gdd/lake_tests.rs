//! GDD from the lake's gold layer, the yearly running total and the crabgrass bands.

use super::*;
use crate::config::DataLakeConfig;
use crate::datasources::WeatherLakeClient;
use chrono::NaiveDate;
use duckdb::Connection;

/// A gold parquet file of `(day, high, low, gdd50)` rows under the temp dir.
pub(crate) fn gold_parquet(name: &str, rows: &[(NaiveDate, f64, f64, f64)]) -> String {
    let path = std::env::temp_dir().join(format!(
        "turfops_gold_{name}_{}.parquet",
        std::process::id()
    ));
    let path_s = path.to_string_lossy().to_string();
    let values: Vec<String> = rows
        .iter()
        .map(|(day, high, low, gdd)| format!("(DATE '{day}', {high:.1}, {low:.1}, {gdd:.1}, 24)"))
        .collect();
    Connection::open_in_memory()
        .unwrap()
        .execute_batch(&format!(
            "COPY (SELECT * FROM (VALUES {}) \
                   t(day, air_temp_max_f, air_temp_min_f, gdd50, hours_observed)) \
             TO '{path_s}' (FORMAT PARQUET)",
            values.join(", ")
        ))
        .unwrap();
    path_s
}

pub(crate) fn lake(gold: &str) -> WeatherLakeClient {
    WeatherLakeClient::new(
        &DataLakeConfig {
            silver_weather_path: "unused.parquet".into(),
            gold_weather_path: gold.into(),
        },
        3761,
    )
}

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

// @spec RULE-GDD-001
#[tokio::test]
async fn each_days_gdd_is_the_lakes_gdd50_summed_as_is() {
    // Highs and lows whose (max+min)/2 − 50 would be 20, 0 and 10: the lake says otherwise,
    // and the lake's value is the one used.
    let gold = gold_parquet(
        "as_is",
        &[
            (d(4, 1), 80.0, 60.0, 7.5),
            (d(4, 2), 50.0, 40.0, 3.0),
            (d(4, 3), 70.0, 50.0, 0.0),
        ],
    );
    let rows = lake(&gold).fetch_daily_gdd(d(4, 1), d(4, 3)).await.unwrap();
    let _ = std::fs::remove_file(&gold);
    let history = accumulate_daily_gdd(&rows);
    let daily: Vec<f64> = history.iter().map(|g| g.gdd_base50).collect();
    assert_eq!(daily, vec![7.5, 3.0, 0.0]);
    assert_eq!(history.last().unwrap().cumulative_gdd_base50, 10.5);
}

// @spec RULE-GDD-003
#[test]
fn the_running_total_restarts_every_january_first() {
    let rows = [
        (
            NaiveDate::from_ymd_opt(2025, 12, 30).unwrap(),
            60.0,
            40.0,
            4.0,
        ),
        (
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            60.0,
            40.0,
            6.0,
        ),
        (d(1, 1), 60.0, 40.0, 1.0),
        (d(1, 2), 60.0, 40.0, 2.0),
    ];
    let totals: Vec<f64> = accumulate_daily_gdd(&rows)
        .iter()
        .map(|g| g.cumulative_gdd_base50)
        .collect();
    assert_eq!(totals, vec![4.0, 10.0, 1.0, 3.0]);
}

// @spec RULE-GDD-004
#[test]
fn crabgrass_status_bands_and_threshold() {
    use CrabgrassStatus::*;
    for (gdd, status) in [
        (0.0, PreGermination),
        (74.9, PreGermination),
        (75.0, ApproachingGermination),
        (149.9, ApproachingGermination),
        (150.0, GerminationLikely),
        (199.9, GerminationLikely),
        (200.0, PostGermination),
        (1500.0, PostGermination),
    ] {
        let model = crabgrass_model(gdd);
        assert_eq!(model.status, status, "{gdd}");
        assert_eq!(model.germination_threshold, 200.0);
        assert_eq!(model.current_gdd, gdd);
    }
}

// @spec RULE-GDD-005
#[test]
fn no_recommendation_source_reads_the_crabgrass_model() {
    // The model lives in logic/gdd.rs and reaches the owner only through `/gdd` (and the
    // MCP tool that calls it). Nothing that builds recommendations may use it.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allowed = [
        "logic/gdd.rs",
        "logic/gdd/",
        "models/",
        "api/gdd.rs",
        "api/gdd/",
        "mcp/",
    ];
    let mut stack = vec![src.clone()];
    let mut readers = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if !rel.ends_with(".rs") || allowed.iter().any(|a| rel.starts_with(a)) {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            if text.contains("crabgrass_model") || text.contains("CrabgrassStatus") {
                readers.push(rel);
            }
        }
    }
    assert!(readers.is_empty(), "crabgrass model read by {readers:?}");
}
