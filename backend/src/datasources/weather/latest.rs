//! The live "current conditions" reading.

use super::{row_to_reading, WeatherLakeClient};
use crate::error::Result;
use crate::models::EnvironmentalReading;

/// How far back a sensor's last good value may come from.
const LOOKBACK_HOURS: i64 = 24;

const COLUMNS: [&str; 13] = [
    "soil_temp_5",
    "soil_temp_10",
    "soil_temp_20",
    "soil_temp_50",
    "soil_temp_100",
    "soil_moisture_5",
    "soil_moisture_10",
    "soil_moisture_20",
    "soil_moisture_50",
    "soil_moisture_100",
    "air_temp_c",
    "rh_pct",
    "precip_mm",
];

impl WeatherLakeClient {
    /// The newest value of **each sensor** within the last day of station data, stamped
    /// with the newest observation time.
    ///
    /// Not simply the newest row: the station's soil probes drop individual hours (in 2026
    /// mostly daytime ones — the 10 cm probe reports about 16 h a day), so the newest row
    /// regularly exists with empty soil columns. Taking it as-is blanked the soil gauges
    /// and silenced every rule that needs the current soil temperature each afternoon,
    /// with a perfectly good reading from an hour or two earlier sitting right below it.
    pub async fn fetch_latest(&self) -> Result<Option<EnvironmentalReading>> {
        let src = Self::parquet(&self.silver_weather_path);
        let station = self.station_wbanno;
        Self::run(move |conn| {
            let latest_of = |col: &str| {
                format!(
                    "arg_max({col}, obs_ts_utc) FILTER (WHERE {col} IS NOT NULL AND {col} > -50)"
                )
            };
            let values = COLUMNS.map(latest_of).join(", ");
            let sql = format!(
                "WITH station AS (SELECT * FROM {src} WHERE CAST(wbanno AS INTEGER) = ?), \
                      recent AS (SELECT * FROM station WHERE obs_ts_utc >= \
                          (SELECT max(obs_ts_utc) FROM station) - INTERVAL {LOOKBACK_HOURS} HOUR) \
                 SELECT max(obs_ts_utc), {values} FROM recent HAVING count(*) > 0"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(duckdb::params![station])?;
            match rows.next()? {
                Some(row) => Ok(Some(row_to_reading(row)?)),
                None => Ok(None),
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DataLakeConfig;
    use duckdb::Connection;

    #[tokio::test]
    async fn a_dropped_soil_hour_falls_back_to_the_last_good_value() {
        let path = std::env::temp_dir().join(format!(
            "turfops_latest_silver_{}.parquet",
            std::process::id()
        ));
        let path_s = path.to_string_lossy().to_string();
        // Hours 0-5; the two newest rows exist but their 10 cm probe reported nothing, and
        // a sentinel slipped through at hour 3.
        Connection::open_in_memory()
            .unwrap()
            .execute_batch(&format!(
                "COPY (
                    SELECT '03761' AS wbanno,
                           TIMESTAMP '2026-09-20 12:00:00' + h * INTERVAL 1 HOUR AS obs_ts_utc,
                           20.0 + h AS soil_temp_5,
                           CASE WHEN h >= 4 THEN NULL WHEN h = 3 THEN -9999.0 ELSE 18.0 + h END
                               AS soil_temp_10,
                           NULL::DOUBLE AS soil_temp_20, NULL::DOUBLE AS soil_temp_50,
                           NULL::DOUBLE AS soil_temp_100,
                           0.25 AS soil_moisture_5, 0.30 AS soil_moisture_10,
                           NULL::DOUBLE AS soil_moisture_20, NULL::DOUBLE AS soil_moisture_50,
                           NULL::DOUBLE AS soil_moisture_100,
                           22.0 AS air_temp_c, 60.0 AS rh_pct, 0.0 AS precip_mm
                    FROM range(6) t(h)
                 ) TO '{path_s}' (FORMAT PARQUET)"
            ))
            .unwrap();
        let client = WeatherLakeClient::new(
            &DataLakeConfig {
                silver_weather_path: path_s,
                gold_weather_path: String::new(),
            },
            3761,
        );

        let reading = client.fetch_latest().await.unwrap().expect("a reading");
        std::fs::remove_file(&path).ok();

        // Stamped with the newest observation…
        assert_eq!(reading.timestamp.to_rfc3339(), "2026-09-20T17:00:00+00:00");
        // …5 cm from that newest row (25 °C = 77 °F)…
        assert_eq!(reading.soil_temp_5_f, Some(77.0));
        // …and 10 cm from hour 2 (20 °C = 68 °F): hours 4-5 are empty, hour 3 is a sentinel.
        assert_eq!(reading.soil_temp_10_f, Some(68.0));
        assert_eq!(reading.soil_temp_20_f, None);
    }
}
