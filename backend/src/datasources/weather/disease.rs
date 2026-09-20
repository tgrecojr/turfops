use super::WeatherLakeClient;
use crate::error::Result;
use crate::models::DailyWeather;
use chrono::NaiveDate;

/// Daily disease-model inputs from the lake plus the station's current UTC offset,
/// which callers need to bucket forecast points into the same local days.
#[derive(Debug, Clone)]
pub struct LakeDailyWeather {
    /// Ascending by date. The most recent day is usually partial (the lake lags).
    pub days: Vec<DailyWeather>,
    pub utc_offset_minutes: i32,
}

/// Magnus dew point (°C) as a SQL expression over the silver columns.
const DEW_POINT_SQL: &str =
    "243.04 * (ln(rh_pct / 100) + 17.625 * air_temp_c / (243.04 + air_temp_c)) \
     / (17.625 - (ln(rh_pct / 100) + 17.625 * air_temp_c / (243.04 + air_temp_c)))";

impl WeatherLakeClient {
    pub fn station_wbanno(&self) -> i32 {
        self.station_wbanno
    }

    /// Aggregate silver hourly observations into one row per local calendar day from
    /// `start` onward. Hours missing air temp or RH are skipped, so `hours_covered`
    /// reflects usable hours only.
    pub async fn fetch_daily_disease_inputs(&self, start: NaiveDate) -> Result<LakeDailyWeather> {
        let src = Self::parquet(&self.silver_weather_path);
        let station = self.station_wbanno;
        let start_s = start.to_string();
        Self::run(move |conn| {
            let sql = format!(
                "SELECT obs_date_local, \
                        CAST(count(*) AS DOUBLE), \
                        avg(air_temp_c), \
                        min(coalesce(air_temp_min_c, air_temp_c)), \
                        max(coalesce(air_temp_max_c, air_temp_c)), \
                        avg(rh_pct), \
                        CAST(sum(CASE WHEN rh_pct >= 90 THEN 1 ELSE 0 END) AS DOUBLE), \
                        CAST(sum(CASE WHEN rh_pct >= 90 OR precip_mm > 0 THEN 1 ELSE 0 END) AS DOUBLE), \
                        CAST(coalesce(sum(precip_mm), 0) AS DOUBLE), \
                        max({DEW_POINT_SQL}) \
                 FROM {src} \
                 WHERE CAST(wbanno AS INTEGER) = ? AND obs_date_local >= ?::DATE \
                   AND air_temp_c IS NOT NULL AND rh_pct > 0 \
                 GROUP BY obs_date_local ORDER BY obs_date_local ASC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(duckdb::params![station, start_s])?;
            let mut days = Vec::new();
            while let Some(row) = rows.next()? {
                days.push(DailyWeather {
                    date: row.get(0)?,
                    hours_covered: row.get(1)?,
                    temp_mean_c: row.get(2)?,
                    temp_min_c: row.get(3)?,
                    temp_max_c: row.get(4)?,
                    rh_mean: row.get(5)?,
                    hours_rh90: row.get(6)?,
                    leaf_wetness_hours: row.get(7)?,
                    precip_mm: row.get(8)?,
                    dew_point_max_c: row.get(9)?,
                    is_forecast: false,
                });
            }

            let offset_sql = format!(
                "SELECT CAST(date_diff('minute', obs_ts_utc, obs_ts_local) AS INTEGER) \
                 FROM {src} WHERE CAST(wbanno AS INTEGER) = ? \
                 ORDER BY obs_ts_utc DESC LIMIT 1"
            );
            let mut stmt = conn.prepare(&offset_sql)?;
            let mut rows = stmt.query(duckdb::params![station])?;
            let utc_offset_minutes = match rows.next()? {
                Some(row) => row.get::<_, Option<i32>>(0)?.unwrap_or(0),
                None => 0,
            };

            Ok(LakeDailyWeather {
                days,
                utc_offset_minutes,
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DataLakeConfig;
    use duckdb::Connection;

    /// Write a silver-shaped parquet: two full days (the second humid with rain) plus
    /// one hour of a third day, all at UTC-4, and one row with NULL RH to be skipped.
    fn write_fixture(path: &str) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "COPY (
                SELECT ts AS obs_ts_utc,
                       ts - INTERVAL 4 HOUR AS obs_ts_local,
                       CAST(ts - INTERVAL 4 HOUR AS DATE) AS obs_date_local,
                       '03761' AS wbanno,
                       CASE WHEN h < 24 THEN 20.0 ELSE 24.0 END AS air_temp_c,
                       CASE WHEN h < 24 THEN 18.5 ELSE 22.0 END AS air_temp_min_c,
                       CAST(NULL AS DOUBLE) AS air_temp_max_c,
                       CASE WHEN h = 5 THEN NULL WHEN h < 24 THEN 60.0
                            WHEN h < 36 THEN 95.0 ELSE 80.0 END AS rh_pct,
                       CASE WHEN h = 40 THEN 2.0 ELSE 0.0 END AS precip_mm
                FROM (SELECT h, TIMESTAMP '2026-09-18 04:00:00' + h * INTERVAL 1 HOUR AS ts
                      FROM range(49) t(h))
             ) TO '{path}' (FORMAT PARQUET)"
        ))
        .unwrap();
    }

    #[tokio::test]
    async fn aggregates_silver_hours_into_local_days() {
        let path =
            std::env::temp_dir().join(format!("turfops_disease_{}.parquet", std::process::id()));
        let path_s = path.to_string_lossy().to_string();
        write_fixture(&path_s);
        let client = WeatherLakeClient::new(
            &DataLakeConfig {
                silver_weather_path: path_s.clone(),
                gold_weather_path: path_s,
            },
            3761,
        );

        let start = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let lake = client.fetch_daily_disease_inputs(start).await.unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(lake.utc_offset_minutes, -240);
        assert_eq!(lake.days.len(), 3);

        let first = &lake.days[0];
        assert_eq!(first.date, NaiveDate::from_ymd_opt(2026, 9, 18).unwrap());
        assert_eq!(first.hours_covered, 23.0); // NULL-RH hour skipped
        assert_eq!(first.temp_min_c, 18.5); // within-hour minimum preferred
        assert_eq!(first.temp_max_c, 20.0); // falls back to hourly mean
        assert_eq!(first.hours_rh90, 0.0);
        assert!(!first.is_forecast);

        let second = &lake.days[1];
        assert_eq!(second.hours_covered, 24.0);
        assert_eq!(second.hours_rh90, 12.0);
        assert_eq!(second.leaf_wetness_hours, 13.0); // 12 humid hours + 1 rain hour
        assert_eq!(second.precip_mm, 2.0);
        assert!((second.rh_mean - 87.5).abs() < 1e-9);
        assert!((second.dew_point_max_c - 23.1).abs() < 0.2); // 24°C @ 95% RH

        assert_eq!(lake.days[2].hours_covered, 1.0);
    }
}
