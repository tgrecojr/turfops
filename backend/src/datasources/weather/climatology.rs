use super::WeatherLakeClient;
use crate::error::Result;
use crate::models::celsius_to_fahrenheit;
use chrono::{Duration, NaiveDate, Utc};
use std::collections::BTreeMap;

/// Minimum hourly soil observations for a day's 5 cm mean to count.
const MIN_SOIL_HOURS: i64 = 12;

/// One station-local day of the multi-year climate record behind the timing windows.
/// Any field can be missing on a given day (sensor outages, sparse days).
#[derive(Debug, Clone, PartialEq)]
pub struct ClimateDay {
    pub date: NaiveDate,
    /// Daily mean soil temperature at 5 cm (°F), from silver hourly.
    pub soil_temp_5_f: Option<f64>,
    /// Daily minimum air temperature (°F), from gold. Drives freeze dates.
    pub air_min_f: Option<f64>,
    pub air_avg_f: Option<f64>,
    pub gdd50: Option<f64>,
}

/// The climate record plus the station's current UTC offset, which callers need to work
/// out the station-local date (the server clock is usually UTC).
#[derive(Debug, Clone)]
pub struct ClimateRecord {
    /// Ascending by date.
    pub days: Vec<ClimateDay>,
    pub utc_offset_minutes: i32,
}

impl ClimateRecord {
    /// Today's date at the station.
    pub fn local_today(&self) -> NaiveDate {
        (Utc::now().naive_utc() + Duration::minutes(self.utc_offset_minutes as i64)).date()
    }
}

impl ClimateDay {
    fn empty(date: NaiveDate) -> Self {
        Self {
            date,
            soil_temp_5_f: None,
            air_min_f: None,
            air_avg_f: None,
            gdd50: None,
        }
    }
}

impl WeatherLakeClient {
    /// Daily climate record from `start` onward, with the station's UTC offset.
    ///
    /// Soil comes from the silver hourly layer because gold only aggregates the 10 cm
    /// probe, and germination thresholds are defined at ~2 in (5 cm). Air temperature
    /// and `gdd50` come from gold, which already carries them per local day.
    pub async fn fetch_climate_record(&self, start: NaiveDate) -> Result<ClimateRecord> {
        let silver = Self::parquet(&self.silver_weather_path);
        let gold = Self::parquet(&self.gold_weather_path);
        let station = self.station_wbanno;
        let start_s = start.to_string();
        Self::run(move |conn| {
            let mut days: BTreeMap<NaiveDate, ClimateDay> = BTreeMap::new();

            // `> -50` drops any raw USCRN missing-data sentinel (-9999) that survived cleaning.
            let soil_sql = format!(
                "SELECT obs_date_local, avg(soil_temp_5) \
                 FROM {silver} \
                 WHERE CAST(wbanno AS INTEGER) = ? AND obs_date_local >= ?::DATE \
                   AND soil_temp_5 IS NOT NULL AND soil_temp_5 > -50 \
                 GROUP BY obs_date_local HAVING count(*) >= {MIN_SOIL_HOURS} \
                 ORDER BY obs_date_local ASC"
            );
            let mut stmt = conn.prepare(&soil_sql)?;
            let mut rows = stmt.query(duckdb::params![station, start_s])?;
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get(0)?;
                let mean_c: f64 = row.get(1)?;
                days.entry(date)
                    .or_insert_with(|| ClimateDay::empty(date))
                    .soil_temp_5_f = Some(celsius_to_fahrenheit(mean_c));
            }

            let air_sql = format!(
                "SELECT day, air_temp_min_f, air_temp_avg_f, gdd50 \
                 FROM {gold} \
                 WHERE day >= ?::DATE AND hours_observed >= 12 \
                 ORDER BY day ASC"
            );
            let mut stmt = conn.prepare(&air_sql)?;
            let mut rows = stmt.query(duckdb::params![start_s])?;
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get(0)?;
                let day = days.entry(date).or_insert_with(|| ClimateDay::empty(date));
                day.air_min_f = row.get(1)?;
                day.air_avg_f = row.get(2)?;
                day.gdd50 = row.get(3)?;
            }

            let offset_sql = format!(
                "SELECT CAST(date_diff('minute', obs_ts_utc, obs_ts_local) AS INTEGER) \
                 FROM {silver} WHERE CAST(wbanno AS INTEGER) = ? \
                 ORDER BY obs_ts_utc DESC LIMIT 1"
            );
            let mut stmt = conn.prepare(&offset_sql)?;
            let mut rows = stmt.query(duckdb::params![station])?;
            let utc_offset_minutes = match rows.next()? {
                Some(row) => row.get::<_, Option<i32>>(0)?.unwrap_or(0),
                None => 0,
            };

            Ok(ClimateRecord {
                days: days.into_values().collect(),
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

    /// Silver: a full day at 10 °C, a full day at 20 °C, then 6 hours of a third day
    /// (too sparse). Gold: those three days plus one the silver layer lacks.
    fn write_fixtures(silver: &str, gold: &str) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "COPY (
                SELECT '03761' AS wbanno,
                       TIMESTAMP '2026-03-01 05:00:00' + h * INTERVAL 1 HOUR AS obs_ts_utc,
                       TIMESTAMP '2026-03-01 00:00:00' + h * INTERVAL 1 HOUR AS obs_ts_local,
                       CAST(TIMESTAMP '2026-03-01 00:00:00' + h * INTERVAL 1 HOUR AS DATE)
                           AS obs_date_local,
                       CASE WHEN h = 3 THEN -9999.0 WHEN h < 24 THEN 10.0 ELSE 20.0 END
                           AS soil_temp_5
                FROM range(54) t(h)
             ) TO '{silver}' (FORMAT PARQUET);
             COPY (
                SELECT DATE '2026-03-01' + CAST(d AS INTEGER) AS day,
                       30.0 + d AS air_temp_min_f,
                       CASE WHEN d = 1 THEN NULL ELSE 40.0 + d END AS air_temp_avg_f,
                       CAST(d AS DOUBLE) AS gdd50,
                       CASE WHEN d = 2 THEN 6 ELSE 24 END AS hours_observed
                FROM range(4) t(d)
             ) TO '{gold}' (FORMAT PARQUET)"
        ))
        .unwrap();
    }

    #[tokio::test]
    async fn merges_silver_soil_with_gold_air_by_local_day() {
        let dir = std::env::temp_dir();
        let silver = dir.join(format!(
            "turfops_clim_silver_{}.parquet",
            std::process::id()
        ));
        let gold = dir.join(format!("turfops_clim_gold_{}.parquet", std::process::id()));
        let (silver_s, gold_s) = (
            silver.to_string_lossy().to_string(),
            gold.to_string_lossy().to_string(),
        );
        write_fixtures(&silver_s, &gold_s);
        let client = WeatherLakeClient::new(
            &DataLakeConfig {
                silver_weather_path: silver_s,
                gold_weather_path: gold_s,
            },
            3761,
        );

        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let record = client.fetch_climate_record(start).await.unwrap();
        let days = record.days;
        std::fs::remove_file(&silver).ok();
        std::fs::remove_file(&gold).ok();

        assert_eq!(record.utc_offset_minutes, -300);

        // Mar 1, 2, 4 — Mar 3 is sparse in both layers.
        assert_eq!(days.len(), 3);

        let first = &days[0];
        assert_eq!(first.date, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
        assert_eq!(first.soil_temp_5_f, Some(50.0)); // sentinel hour excluded from the mean
        assert_eq!(first.air_min_f, Some(30.0));
        assert_eq!(first.gdd50, Some(0.0));

        assert_eq!(days[1].soil_temp_5_f, Some(68.0));
        assert_eq!(days[1].air_avg_f, None);

        let last = &days[2];
        assert_eq!(last.date, NaiveDate::from_ymd_opt(2026, 3, 4).unwrap());
        assert_eq!(last.soil_temp_5_f, None);
        assert_eq!(last.air_min_f, Some(33.0));
    }
}
