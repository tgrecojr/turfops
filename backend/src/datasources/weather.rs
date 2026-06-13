use crate::config::DataLakeConfig;
use crate::error::{Result, TurfOpsError};
use crate::models::{
    celsius_to_fahrenheit, DataSource, EnvironmentalReading, EnvironmentalSummary, Trend,
};
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use duckdb::Connection;

/// Reads NOAA USCRN weather data from the Dagster data lake (parquet on a mounted
/// filesystem) using an embedded DuckDB engine.
///
/// Replaces the retired SoilData PostgreSQL connection. Two layers are used:
/// - **silver** (`silver_weather_path`): hourly, cleaned + deduped observations in
///   native units (°C, mm, % RH, fractional moisture). Drives the live reading,
///   the 7-day rolling summary, and the soil-temp trend.
/// - **gold** (`gold_weather_path`): daily pre-aggregated means already in °F, plus a
///   precomputed `gdd50`. Drives the seasonal plan, the soil-temp regression, and GDD.
///
/// DuckDB connections are not `Sync`, so every query opens a short-lived in-memory
/// connection inside `spawn_blocking` and scans the parquet directly.
#[derive(Clone)]
pub struct WeatherLakeClient {
    silver_weather_path: String,
    gold_weather_path: String,
    station_wbanno: i32,
}

/// One daily GDD record sourced from the gold layer, before cumulative accumulation.
/// `(date, air_temp_max_f, air_temp_min_f, gdd50)`.
pub type DailyGddRow = (NaiveDate, f64, f64, f64);

impl WeatherLakeClient {
    pub fn new(config: &DataLakeConfig, station_wbanno: i32) -> Self {
        Self {
            silver_weather_path: config.silver_weather_path.clone(),
            gold_weather_path: config.gold_weather_path.clone(),
            station_wbanno,
        }
    }

    /// Wrap a blocking DuckDB closure in `spawn_blocking` and normalize errors.
    async fn run<T, F>(f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T> + Send + 'static,
    {
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open_in_memory()?;
            f(&conn)
        })
        .await
        .map_err(|e| TurfOpsError::DataSourceUnavailable(format!("data lake task failed: {e}")))?
    }

    /// `read_parquet('<path>')` with single quotes escaped (paths come from trusted config).
    fn parquet(path: &str) -> String {
        format!("read_parquet('{}')", path.replace('\'', "''"))
    }

    pub async fn fetch_latest(&self) -> Result<Option<EnvironmentalReading>> {
        let src = Self::parquet(&self.silver_weather_path);
        let station = self.station_wbanno;
        Self::run(move |conn| {
            let sql = format!(
                "SELECT obs_ts_utc, soil_temp_5, soil_temp_10, soil_temp_20, soil_temp_50, soil_temp_100, \
                        soil_moisture_5, soil_moisture_10, soil_moisture_20, soil_moisture_50, soil_moisture_100, \
                        air_temp_c, rh_pct, precip_mm \
                 FROM {src} WHERE CAST(wbanno AS INTEGER) = ? ORDER BY obs_ts_utc DESC LIMIT 1"
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

    /// Hourly readings in [start, end], newest first (matches the old ordering the
    /// 7-day summary + trend logic relies on).
    pub async fn fetch_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<EnvironmentalReading>> {
        let src = Self::parquet(&self.silver_weather_path);
        let station = self.station_wbanno;
        let (start_s, end_s) = (fmt_ts(start), fmt_ts(end));
        Self::run(move |conn| {
            let sql = format!(
                "SELECT obs_ts_utc, soil_temp_5, soil_temp_10, soil_temp_20, soil_temp_50, soil_temp_100, \
                        soil_moisture_5, soil_moisture_10, soil_moisture_20, soil_moisture_50, soil_moisture_100, \
                        air_temp_c, rh_pct, precip_mm \
                 FROM {src} \
                 WHERE CAST(wbanno AS INTEGER) = ? AND obs_ts_utc >= ?::TIMESTAMP AND obs_ts_utc <= ?::TIMESTAMP \
                 ORDER BY obs_ts_utc DESC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(duckdb::params![station, start_s, end_s])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                out.push(row_to_reading(row)?);
            }
            Ok(out)
        })
        .await
    }

    /// Current reading plus 7-day rolling averages and the soil-temp trend.
    pub async fn fetch_summary(&self) -> Result<EnvironmentalSummary> {
        let now = Utc::now();
        let seven_days_ago = now - Duration::days(7);

        let current = self.fetch_latest().await?;
        let readings = self.fetch_range(seven_days_ago, now).await?;

        let mut summary = EnvironmentalSummary {
            current,
            last_updated: Some(now),
            ..Default::default()
        };

        if readings.is_empty() {
            return Ok(summary);
        }

        let soil_temps: Vec<f64> = readings.iter().filter_map(|r| r.soil_temp_10_f).collect();
        if !soil_temps.is_empty() {
            summary.soil_temp_7day_avg_f =
                Some(soil_temps.iter().sum::<f64>() / soil_temps.len() as f64);
        }

        let ambient_temps: Vec<f64> = readings.iter().filter_map(|r| r.ambient_temp_f).collect();
        if !ambient_temps.is_empty() {
            summary.ambient_temp_7day_avg_f =
                Some(ambient_temps.iter().sum::<f64>() / ambient_temps.len() as f64);
        }

        let humidities: Vec<f64> = readings.iter().filter_map(|r| r.humidity_percent).collect();
        if !humidities.is_empty() {
            summary.humidity_7day_avg =
                Some(humidities.iter().sum::<f64>() / humidities.len() as f64);
        }

        let precip_sum: f64 = readings
            .iter()
            .filter_map(|r| r.precipitation_mm)
            .filter(|p| *p >= 0.0)
            .sum();
        summary.precipitation_7day_total_mm = Some(precip_sum);

        summary.soil_temp_trend = calculate_trend(&readings);

        Ok(summary)
    }

    /// Daily mean soil temp at 10cm (°F) per day in range, for the seasonal plan.
    /// Sourced from the gold layer (already aggregated + converted to °F).
    pub async fn fetch_daily_soil_temp_averages(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<crate::models::seasonal_plan::DailySoilTempAvg>> {
        let src = Self::parquet(&self.gold_weather_path);
        let (start_s, end_s) = (fmt_date(start), fmt_date(end));
        Self::run(move |conn| {
            let sql = format!(
                "SELECT day, soil_temp_10_f_mean \
                 FROM {src} \
                 WHERE day >= ?::DATE AND day <= ?::DATE AND hours_observed >= 12 \
                   AND soil_temp_10_f_mean IS NOT NULL \
                 ORDER BY day ASC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(duckdb::params![start_s, end_s])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get(0)?;
                let avg_temp_f: f64 = row.get(1)?;
                out.push(crate::models::seasonal_plan::DailySoilTempAvg { date, avg_temp_f });
            }
            Ok(out)
        })
        .await
    }

    /// Daily paired `(date, air_temp_avg_f, soil_temp_10_f_mean)` for the regression model.
    pub async fn fetch_daily_paired_averages(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(NaiveDate, f64, f64)>> {
        let src = Self::parquet(&self.gold_weather_path);
        let (start_s, end_s) = (fmt_date(start), fmt_date(end));
        Self::run(move |conn| {
            let sql = format!(
                "SELECT day, air_temp_avg_f, soil_temp_10_f_mean \
                 FROM {src} \
                 WHERE day >= ?::DATE AND day <= ?::DATE AND hours_observed >= 12 \
                   AND air_temp_avg_f IS NOT NULL AND soil_temp_10_f_mean IS NOT NULL \
                 ORDER BY day ASC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(duckdb::params![start_s, end_s])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get(0)?;
                let air_f: f64 = row.get(1)?;
                let soil_f: f64 = row.get(2)?;
                out.push((date, air_f, soil_f));
            }
            Ok(out)
        })
        .await
    }

    /// Daily GDD rows `(date, high_f, low_f, gdd50)` from the gold layer for [start, end].
    /// `gdd50` is precomputed in the lake with the identical base-50°F formula.
    pub async fn fetch_daily_gdd(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<DailyGddRow>> {
        let src = Self::parquet(&self.gold_weather_path);
        let (start_s, end_s) = (start.to_string(), end.to_string());
        Self::run(move |conn| {
            let sql = format!(
                "SELECT day, air_temp_max_f, air_temp_min_f, gdd50 \
                 FROM {src} \
                 WHERE day >= ?::DATE AND day <= ?::DATE AND gdd50 IS NOT NULL \
                 ORDER BY day ASC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(duckdb::params![start_s, end_s])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let date: NaiveDate = row.get(0)?;
                let high_f: f64 = row.get(1)?;
                let low_f: f64 = row.get(2)?;
                let gdd50: f64 = row.get(3)?;
                out.push((date, high_f, low_f, gdd50));
            }
            Ok(out)
        })
        .await
    }

    /// Year-to-date cumulative GDD (base 50°F) — sum of daily `gdd50` from Jan 1 to today.
    pub async fn fetch_gdd_ytd(&self, year: i32) -> Result<Option<f64>> {
        let jan1 = NaiveDate::from_ymd_opt(year, 1, 1)
            .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {year}")))?;
        let today = Utc::now().date_naive();
        if today < jan1 {
            return Ok(None);
        }
        let rows = self.fetch_daily_gdd(jan1, today).await?;
        if rows.is_empty() {
            return Ok(None);
        }
        Ok(Some(rows.iter().map(|(_, _, _, gdd)| *gdd).sum()))
    }

    /// Cheap readability probe against the gold parquet.
    pub async fn test_connection(&self) -> Result<bool> {
        let src = Self::parquet(&self.gold_weather_path);
        let ok = Self::run(move |conn| {
            let sql = format!("SELECT 1 FROM {src} LIMIT 1");
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;
            Ok(rows.next()?.is_some())
        })
        .await;
        Ok(ok.unwrap_or(false))
    }
}

/// Format a UTC instant as a naive `YYYY-MM-DD HH:MM:SS` string for a TIMESTAMP comparison.
fn fmt_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Format a UTC instant's date as `YYYY-MM-DD` for a DATE comparison.
fn fmt_date(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d").to_string()
}

/// Map a silver hourly row to an `EnvironmentalReading`, converting °C → °F.
/// Column order must match the SELECT lists above.
fn row_to_reading(row: &duckdb::Row) -> duckdb::Result<EnvironmentalReading> {
    let ts: NaiveDateTime = row.get(0)?;
    let mut reading = EnvironmentalReading::new(DataSource::SoilData);
    reading.timestamp = DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc);

    let temp_f = |c: Option<f64>| c.map(celsius_to_fahrenheit);
    reading.soil_temp_5_f = temp_f(row.get(1)?);
    reading.soil_temp_10_f = temp_f(row.get(2)?);
    reading.soil_temp_20_f = temp_f(row.get(3)?);
    reading.soil_temp_50_f = temp_f(row.get(4)?);
    reading.soil_temp_100_f = temp_f(row.get(5)?);

    let moisture = |m: Option<f64>| m.filter(|v| *v >= 0.0);
    reading.soil_moisture_5 = moisture(row.get(6)?);
    reading.soil_moisture_10 = moisture(row.get(7)?);
    reading.soil_moisture_20 = moisture(row.get(8)?);
    reading.soil_moisture_50 = moisture(row.get(9)?);
    reading.soil_moisture_100 = moisture(row.get(10)?);

    reading.ambient_temp_f = temp_f(row.get(11)?);
    reading.humidity_percent = row.get::<_, Option<f64>>(12)?.filter(|h| *h >= 0.0);
    reading.precipitation_mm = row.get::<_, Option<f64>>(13)?.filter(|p| *p >= 0.0);

    Ok(reading)
}

/// Soil-temp trend from hourly readings (newest first): compares the last 24h to the
/// previous 24h. Identical thresholds to the retired SoilData implementation.
fn calculate_trend(readings: &[EnvironmentalReading]) -> Trend {
    if readings.len() < 48 {
        return Trend::Unknown;
    }

    let recent: Vec<f64> = readings
        .iter()
        .take(24)
        .filter_map(|r| r.soil_temp_10_f)
        .collect();
    let previous: Vec<f64> = readings
        .iter()
        .skip(24)
        .take(24)
        .filter_map(|r| r.soil_temp_10_f)
        .collect();

    if recent.is_empty() || previous.is_empty() {
        return Trend::Unknown;
    }

    let recent_avg = recent.iter().sum::<f64>() / recent.len() as f64;
    let previous_avg = previous.iter().sum::<f64>() / previous.len() as f64;
    let diff = recent_avg - previous_avg;

    if diff > 2.0 {
        Trend::Rising
    } else if diff < -2.0 {
        Trend::Falling
    } else {
        Trend::Stable
    }
}
