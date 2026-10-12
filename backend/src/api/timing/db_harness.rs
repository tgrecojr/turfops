//! A real app state for the timing hooks: a migrated database, a parquet weather lake
//! written by DuckDB, and optionally a forecast served by a local stub.

use crate::clock::Clock;
use crate::config::{Config, OpenWeatherMapConfig};
use crate::datasources::OpenWeatherMapClient;
use crate::db::queries;
use crate::logic::data_sync::DataSyncService;
use crate::models::{GrassType, LawnProfile};
use crate::state::AppState;
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) const ZONE: Clock = Clock::Zone(chrono_tz::America::New_York);

/// The station's typical year (°F): peak 77 on Jul 19, trough 33 in mid-January.
const CURVE: &str = "(55 + 22 * cos(2 * pi() * (dayofyear(d) - 200) / 365.0))";

/// Silver + gold parquet in a directory of its own, removed on drop.
pub(super) struct Lake {
    dir: PathBuf,
}

impl Lake {
    fn new_dir() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "turfops_timing_lake_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Daily records from Jan 1 `years` years back through yesterday (the lawn's date).
    pub(super) fn with_years(years: i32) -> Self {
        let today = ZONE.today();
        let start = NaiveDate::from_ymd_opt(today.year() - years, 1, 1).unwrap();
        let lake = Self {
            dir: Self::new_dir(),
        };
        let days = format!("range(TIMESTAMP '{start}', TIMESTAMP '{today}', INTERVAL 1 DAY) t(d)");
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "COPY (
                SELECT 3761 AS wbanno, CAST(d AS DATE) AS obs_date_local,
                       ({CURVE} - 32) * 5 / 9 AS soil_temp_5
                FROM {days}, range(24) h(hour)
             ) TO '{silver}' (FORMAT PARQUET);
             COPY (
                SELECT CAST(d AS DATE) AS day,
                       {CURVE} - 16.5 AS air_temp_min_f,
                       {CURVE} - 5 AS air_temp_avg_f,
                       greatest({CURVE} - 55, 0) AS gdd50,
                       24 AS hours_observed
                FROM {days}
             ) TO '{gold}' (FORMAT PARQUET);",
            silver = lake.silver(),
            gold = lake.gold(),
        ))
        .unwrap();
        lake
    }

    /// A lake whose parquet files do not exist.
    pub(super) fn missing() -> Self {
        Self {
            dir: Self::new_dir(),
        }
    }

    pub(super) fn silver(&self) -> String {
        self.dir.join("silver.parquet").to_string_lossy().into()
    }

    pub(super) fn gold(&self) -> String {
        self.dir.join("gold.parquet").to_string_lossy().into()
    }

    /// Take the parquet away, as an unmounted lake would.
    pub(super) fn unmount(&self) {
        std::fs::remove_file(self.silver()).ok();
        std::fs::remove_file(self.gold()).ok();
    }
}

impl Drop for Lake {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

/// App state over `lake`, with the forecast fetched from `forecast_base` when given.
pub(super) async fn app_state(pool: &PgPool, lake: &Lake, forecast_base: Option<&str>) -> AppState {
    let mut config: Config = serde_json::from_value(serde_json::json!({
        "lawn": { "name": "Test", "grass_type": "TallFescue", "usda_zone": "7a" },
        "noaa": { "station_wbanno": 3761 },
        "datalake": { "silver_weather_path": lake.silver(), "gold_weather_path": lake.gold() },
        "homeassistant": {
            "url": "http://127.0.0.1:1", "token": "",
            "temperature_entity": "t", "humidity_entity": "h"
        },
        "server": { "host": "127.0.0.1", "port": 3000 },
        "database": {
            "host": "localhost", "port": 5432, "name": "x", "user": "x",
            "password": "x", "max_connections": 1
        },
        "mcp": {}
    }))
    .unwrap();
    config.clock = ZONE;
    let mut sync = DataSyncService::initialize(&config, pool.clone()).await;
    if let Some(base) = forecast_base {
        let owm = OpenWeatherMapConfig {
            api_key: "test-key".into(),
            latitude: 39.86,
            longitude: -75.79,
            enabled: true,
        };
        sync.set_forecast_client(OpenWeatherMapClient::new(owm, ZONE).with_base_url(base));
    }
    AppState::new(pool.clone(), sync, None, false, ZONE)
}

/// Insert the lawn profile and return it as stored.
pub(super) async fn lawn(pool: &PgPool) -> LawnProfile {
    let profile = LawnProfile::new("Test".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(pool, &profile).await.unwrap();
    queries::get_default_lawn_profile(pool)
        .await
        .unwrap()
        .unwrap()
}

/// Local noon-ish instants for `day` at the given local hours.
fn at_hours(day: NaiveDate, hours: &[u32]) -> Vec<DateTime<Utc>> {
    let tz = chrono_tz::America::New_York;
    hours
        .iter()
        .filter_map(|&h| tz.from_local_datetime(&day.and_hms_opt(h, 0, 0)?).single())
        .map(|t| t.with_timezone(&Utc))
        .collect()
}

/// An OpenWeatherMap 5-day/3-hour body: `days` of `(offset from today, temps)` with one
/// 3-hourly point per temperature (8 points make a full day).
pub(super) fn forecast_body(days: &[(i64, Vec<f64>)]) -> &'static str {
    let today = ZONE.today();
    let mut items = Vec::new();
    for (offset, temps) in days {
        let hours: Vec<u32> = (0..temps.len() as u32).map(|i| 1 + 3 * i).collect();
        for (instant, temp) in at_hours(today + Duration::days(*offset), &hours)
            .into_iter()
            .zip(temps)
        {
            items.push(serde_json::json!({
                "dt": instant.timestamp(),
                "main": { "temp": temp, "feels_like": temp, "humidity": 60.0 },
                "weather": [{ "id": 800, "main": "Clear", "description": "clear sky" }],
                "clouds": { "all": 0.0 },
                "wind": { "speed": 5.0 },
                "pop": 0.0
            }));
        }
    }
    let body = serde_json::json!({
        "list": items,
        "city": {
            "name": "Avondale", "country": "US",
            "coord": { "lat": 39.86, "lon": -75.79 }, "timezone": -14400
        }
    });
    Box::leak(body.to_string().into_boxed_str())
}
