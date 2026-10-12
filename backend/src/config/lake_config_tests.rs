//! The weather sources' settings: lake paths, station and the patio sensor's defaults.

use super::load::from_vars;
use super::Config;
use std::collections::HashMap;

fn load(pairs: &[(&'static str, &str)]) -> (Config, Vec<String>) {
    let mut vars: HashMap<&'static str, String> =
        HashMap::from([("DATABASE_PASSWORD", "pw".to_string())]);
    for (k, v) in pairs {
        vars.insert(k, v.to_string());
    }
    from_vars(&|key| vars.get(key).cloned()).unwrap()
}

// @spec WX-SRC-CFG-001
#[test]
fn lake_paths_derive_from_the_root_and_can_be_overridden() {
    let (defaults, _) = load(&[]);
    assert_eq!(
        defaults.datalake.silver_weather_path,
        "/data/silver/weather/silver_weather.parquet"
    );
    assert_eq!(
        defaults.datalake.gold_weather_path,
        "/data/gold/weather/daily_weather.parquet"
    );

    let (rooted, _) = load(&[("DATALAKE_ROOT", "/mnt/lake")]);
    assert_eq!(
        rooted.datalake.silver_weather_path,
        "/mnt/lake/silver/weather/silver_weather.parquet"
    );
    assert_eq!(
        rooted.datalake.gold_weather_path,
        "/mnt/lake/gold/weather/daily_weather.parquet"
    );

    let (overridden, _) = load(&[
        ("DATALAKE_ROOT", "/mnt/lake"),
        ("WEATHER_SILVER_PATH", "/x/silver.parquet"),
        ("WEATHER_GOLD_PATH", "/y/gold.parquet"),
    ]);
    assert_eq!(overridden.datalake.silver_weather_path, "/x/silver.parquet");
    assert_eq!(overridden.datalake.gold_weather_path, "/y/gold.parquet");
}

// @spec WX-SRC-CFG-001
#[test]
fn the_station_defaults_to_3761_and_an_unparsable_one_warns() {
    let (defaults, warnings) = load(&[]);
    assert_eq!(defaults.noaa.station_wbanno, 3761);
    assert!(warnings.is_empty(), "{warnings:?}");

    let (set, _) = load(&[("NOAA_STATION_WBANNO", "54902")]);
    assert_eq!(set.noaa.station_wbanno, 54902);

    let (bad, warnings) = load(&[("NOAA_STATION_WBANNO", "avondale")]);
    assert_eq!(bad.noaa.station_wbanno, 3761);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("NOAA_STATION_WBANNO") && w.contains("3761")),
        "{warnings:?}"
    );
}

// @spec WX-SRC-CFG-002
#[test]
fn home_assistant_defaults_to_localhost_with_no_token() {
    let (config, _) = load(&[]);
    assert_eq!(config.homeassistant.url, "http://localhost:8123");
    assert!(config.homeassistant.token.is_empty());
}
