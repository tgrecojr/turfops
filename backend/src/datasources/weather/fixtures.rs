//! Small silver / gold parquet files for lake tests, written with DuckDB into the temp dir
//! and removed when dropped.

use super::WeatherLakeClient;
use crate::config::DataLakeConfig;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use duckdb::Connection;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A parquet file in the temp dir, deleted on drop.
pub(crate) struct TempParquet {
    pub path: String,
}

impl Drop for TempParquet {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// A unique path in the temp dir; `tag` may contain characters a path allows.
pub(crate) fn temp_path(tag: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir()
        .join(format!(
            "turfops_wx_{tag}_{}_{n}.parquet",
            std::process::id()
        ))
        .to_string_lossy()
        .to_string()
}

/// Write the rows of `select` to `path` as parquet.
pub(crate) fn write(path: &str, select: &str) {
    let escaped = path.replace('\'', "''");
    Connection::open_in_memory()
        .unwrap()
        .execute_batch(&format!("COPY ({select}) TO '{escaped}' (FORMAT PARQUET)"))
        .unwrap();
}

/// One silver hour. The local time is UTC − 4 h (EDT) unless `local_date` overrides the
/// lake's `obs_date_local`.
#[derive(Clone, Default)]
pub(crate) struct SilverRow {
    pub wbanno: Option<&'static str>,
    pub at: NaiveDateTime,
    pub local_date: Option<NaiveDate>,
    pub soil_temp_5: Option<f64>,
    pub soil_temp_10: Option<f64>,
    pub soil_moisture_5: Option<f64>,
    pub soil_moisture_10: Option<f64>,
    pub soil_moisture_20: Option<f64>,
    pub air_temp_c: Option<f64>,
    pub rh_pct: Option<f64>,
    pub precip_mm: Option<f64>,
}

fn num(v: Option<f64>) -> String {
    v.map_or("NULL".into(), |v| format!("{v:?}"))
}

/// Silver parquet with the full column set the lake reads.
pub(crate) fn silver(rows: &[SilverRow]) -> TempParquet {
    let values: Vec<String> = rows
        .iter()
        .map(|r| {
            let local = r.at - Duration::hours(4);
            let local_date = r.local_date.unwrap_or(local.date());
            format!(
                "('{}', TIMESTAMP '{}', TIMESTAMP '{}', DATE '{}', {}, {}, {}, {}, {}, {}, {}, {})",
                r.wbanno.unwrap_or("03761"),
                r.at,
                local,
                local_date,
                num(r.soil_temp_5),
                num(r.soil_temp_10),
                num(r.soil_moisture_5),
                num(r.soil_moisture_10),
                num(r.soil_moisture_20),
                num(r.air_temp_c),
                num(r.rh_pct),
                num(r.precip_mm),
            )
        })
        .collect();
    let file = TempParquet {
        path: temp_path("silver"),
    };
    write(
        &file.path,
        &format!(
            "SELECT wbanno, obs_ts_utc, obs_ts_local, obs_date_local,
                    CAST(s5 AS DOUBLE) AS soil_temp_5, CAST(s10 AS DOUBLE) AS soil_temp_10,
                    CAST(NULL AS DOUBLE) AS soil_temp_20, CAST(NULL AS DOUBLE) AS soil_temp_50,
                    CAST(NULL AS DOUBLE) AS soil_temp_100,
                    CAST(m5 AS DOUBLE) AS soil_moisture_5, CAST(m10 AS DOUBLE) AS soil_moisture_10,
                    CAST(m20 AS DOUBLE) AS soil_moisture_20,
                    CAST(NULL AS DOUBLE) AS soil_moisture_50, CAST(NULL AS DOUBLE) AS soil_moisture_100,
                    CAST(air AS DOUBLE) AS air_temp_c,
                    CAST(NULL AS DOUBLE) AS air_temp_min_c, CAST(NULL AS DOUBLE) AS air_temp_max_c,
                    CAST(rh AS DOUBLE) AS rh_pct, CAST(p AS DOUBLE) AS precip_mm
             FROM (VALUES {}) t(wbanno, obs_ts_utc, obs_ts_local, obs_date_local,
                                s5, s10, m5, m10, m20, air, rh, p)",
            values.join(", ")
        ),
    );
    file
}

/// One gold day (°F, with `gdd50` and `hours_observed`).
#[derive(Clone)]
pub(crate) struct GoldRow {
    pub day: NaiveDate,
    pub air_avg_f: Option<f64>,
    pub air_min_f: Option<f64>,
    pub air_max_f: Option<f64>,
    pub soil_10_f: Option<f64>,
    pub gdd50: Option<f64>,
    pub hours_observed: i32,
}

impl GoldRow {
    /// A complete 24-hour day.
    pub(crate) fn day(day: NaiveDate) -> Self {
        Self {
            day,
            air_avg_f: Some(60.0),
            air_min_f: Some(50.0),
            air_max_f: Some(70.0),
            soil_10_f: Some(58.0),
            gdd50: Some(10.0),
            hours_observed: 24,
        }
    }
}

/// Gold parquet with the columns the lake reads.
pub(crate) fn gold(rows: &[GoldRow]) -> TempParquet {
    let values: Vec<String> = rows
        .iter()
        .map(|r| {
            format!(
                "(DATE '{}', {}, {}, {}, {}, {}, {})",
                r.day,
                num(r.air_avg_f),
                num(r.air_min_f),
                num(r.air_max_f),
                num(r.soil_10_f),
                num(r.gdd50),
                r.hours_observed
            )
        })
        .collect();
    let file = TempParquet {
        path: temp_path("gold"),
    };
    write(
        &file.path,
        &format!(
            "SELECT day, CAST(a AS DOUBLE) AS air_temp_avg_f, CAST(lo AS DOUBLE) AS air_temp_min_f,
                    CAST(hi AS DOUBLE) AS air_temp_max_f, CAST(s AS DOUBLE) AS soil_temp_10_f_mean,
                    CAST(g AS DOUBLE) AS gdd50, CAST(h AS INTEGER) AS hours_observed
             FROM (VALUES {}) t(day, a, lo, hi, s, g, h)",
            values.join(", ")
        ),
    );
    file
}

/// A lake client over the given files, filtering silver to station 3761.
pub(crate) fn client(silver: &str, gold: &str) -> WeatherLakeClient {
    WeatherLakeClient::new(
        &DataLakeConfig {
            silver_weather_path: silver.into(),
            gold_weather_path: gold.into(),
        },
        3761,
    )
}
