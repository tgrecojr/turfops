use crate::config::SoilDataConfig;
use crate::error::Result;
use crate::models::{
    celsius_to_fahrenheit, DataSource, EnvironmentalReading, EnvironmentalSummary, Trend,
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

pub struct SoilDataClient {
    pool: PgPool,
    station_wbanno: i32,
}

impl SoilDataClient {
    /// Create a client with a lazy connection pool that connects on first query.
    pub fn connect_lazy(config: &SoilDataConfig, station_wbanno: i32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_lazy_with(config.connect_options());

        Ok(Self {
            pool,
            station_wbanno,
        })
    }

    pub async fn fetch_latest(&self) -> Result<Option<EnvironmentalReading>> {
        let row = sqlx::query(
            r#"
            SELECT
                utc_datetime,
                soil_temp_5, soil_temp_10, soil_temp_20, soil_temp_50, soil_temp_100,
                soil_moisture_5, soil_moisture_10, soil_moisture_20, soil_moisture_50, soil_moisture_100,
                t_calc, rh_hr_avg, p_calc
            FROM observations
            WHERE wbanno = $1
            ORDER BY utc_datetime DESC
            LIMIT 1
            "#,
        )
        .bind(self.station_wbanno)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_reading(&row)?)),
            None => Ok(None),
        }
    }

    pub async fn fetch_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<EnvironmentalReading>> {
        let rows = sqlx::query(
            r#"
            SELECT
                utc_datetime,
                soil_temp_5, soil_temp_10, soil_temp_20, soil_temp_50, soil_temp_100,
                soil_moisture_5, soil_moisture_10, soil_moisture_20, soil_moisture_50, soil_moisture_100,
                t_calc, rh_hr_avg, p_calc
            FROM observations
            WHERE wbanno = $1 AND utc_datetime >= $2 AND utc_datetime <= $3
            ORDER BY utc_datetime DESC
            "#,
        )
        .bind(self.station_wbanno)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let readings: Vec<EnvironmentalReading> = rows
            .iter()
            .filter_map(|row| self.row_to_reading(row).ok())
            .collect();

        Ok(readings)
    }

    pub async fn fetch_summary(&self) -> Result<EnvironmentalSummary> {
        let now = Utc::now();
        let seven_days_ago = now - Duration::days(7);

        // Get current reading
        let current = self.fetch_latest().await?;

        // Get 7-day readings for averages
        let readings = self.fetch_range(seven_days_ago, now).await?;

        let mut summary = EnvironmentalSummary {
            current,
            last_updated: Some(now),
            ..Default::default()
        };

        if readings.is_empty() {
            return Ok(summary);
        }

        // Calculate 7-day averages
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

        // Sum precipitation
        let precip_sum: f64 = readings
            .iter()
            .filter_map(|r| r.precipitation_mm)
            .filter(|p| *p >= 0.0) // Filter out missing data markers
            .sum();
        summary.precipitation_7day_total_mm = Some(precip_sum);

        // Calculate trend (compare last 24h to previous 24h)
        summary.soil_temp_trend = self.calculate_trend(&readings);

        Ok(summary)
    }

    fn row_to_reading(&self, row: &sqlx::postgres::PgRow) -> Result<EnvironmentalReading> {
        let timestamp: DateTime<Utc> = row.try_get("utc_datetime")?;

        let mut reading = EnvironmentalReading::new(DataSource::SoilData);
        reading.timestamp = timestamp;

        // Convert soil temps from Celsius to Fahrenheit
        if let Ok(Some(t)) = row.try_get::<Option<f32>, _>("soil_temp_5") {
            reading.soil_temp_5_f = Some(celsius_to_fahrenheit(t as f64));
        }
        if let Ok(Some(t)) = row.try_get::<Option<f32>, _>("soil_temp_10") {
            reading.soil_temp_10_f = Some(celsius_to_fahrenheit(t as f64));
        }
        if let Ok(Some(t)) = row.try_get::<Option<f32>, _>("soil_temp_20") {
            reading.soil_temp_20_f = Some(celsius_to_fahrenheit(t as f64));
        }
        if let Ok(Some(t)) = row.try_get::<Option<f32>, _>("soil_temp_50") {
            reading.soil_temp_50_f = Some(celsius_to_fahrenheit(t as f64));
        }
        if let Ok(Some(t)) = row.try_get::<Option<f32>, _>("soil_temp_100") {
            reading.soil_temp_100_f = Some(celsius_to_fahrenheit(t as f64));
        }

        // Soil moisture (fractional 0.0-1.0)
        if let Ok(Some(m)) = row.try_get::<Option<f32>, _>("soil_moisture_5") {
            if m >= 0.0 {
                reading.soil_moisture_5 = Some(m as f64);
            }
        }
        if let Ok(Some(m)) = row.try_get::<Option<f32>, _>("soil_moisture_10") {
            if m >= 0.0 {
                reading.soil_moisture_10 = Some(m as f64);
            }
        }
        if let Ok(Some(m)) = row.try_get::<Option<f32>, _>("soil_moisture_20") {
            if m >= 0.0 {
                reading.soil_moisture_20 = Some(m as f64);
            }
        }
        if let Ok(Some(m)) = row.try_get::<Option<f32>, _>("soil_moisture_50") {
            if m >= 0.0 {
                reading.soil_moisture_50 = Some(m as f64);
            }
        }
        if let Ok(Some(m)) = row.try_get::<Option<f32>, _>("soil_moisture_100") {
            if m >= 0.0 {
                reading.soil_moisture_100 = Some(m as f64);
            }
        }

        // Ambient temperature (Celsius -> Fahrenheit)
        if let Ok(Some(t)) = row.try_get::<Option<f32>, _>("t_calc") {
            if t > -9999.0 {
                reading.ambient_temp_f = Some(celsius_to_fahrenheit(t as f64));
            }
        }

        // Humidity
        if let Ok(Some(h)) = row.try_get::<Option<f32>, _>("rh_hr_avg") {
            if h >= 0.0 {
                reading.humidity_percent = Some(h as f64);
            }
        }

        // Precipitation
        if let Ok(Some(p)) = row.try_get::<Option<f32>, _>("p_calc") {
            if p >= 0.0 {
                reading.precipitation_mm = Some(p as f64);
            }
        }

        Ok(reading)
    }

    fn calculate_trend(&self, readings: &[EnvironmentalReading]) -> Trend {
        if readings.len() < 48 {
            return Trend::Unknown;
        }

        // Recent 24 readings (24 hours)
        let recent: Vec<f64> = readings
            .iter()
            .take(24)
            .filter_map(|r| r.soil_temp_10_f)
            .collect();

        // Previous 24 readings
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

    /// Fetch daily average soil_temp_10 values aggregated server-side.
    /// Returns one row per day with the mean soil temp at 10cm depth (°F).
    /// Used for historical threshold analysis in the seasonal plan.
    pub async fn fetch_daily_soil_temp_averages(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<crate::models::seasonal_plan::DailySoilTempAvg>> {
        let rows = sqlx::query(
            r#"
            SELECT
                DATE(utc_datetime) as day,
                AVG(soil_temp_10) as avg_temp_c,
                COUNT(*) as readings
            FROM observations
            WHERE wbanno = $1
              AND utc_datetime >= $2
              AND utc_datetime <= $3
              AND soil_temp_10 IS NOT NULL
              AND soil_temp_10 > -9999
            GROUP BY DATE(utc_datetime)
            HAVING COUNT(*) >= 12
            ORDER BY day ASC
            "#,
        )
        .bind(self.station_wbanno)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let results: Vec<crate::models::seasonal_plan::DailySoilTempAvg> = rows
            .iter()
            .filter_map(|row| {
                let day: NaiveDate = row.try_get("day").ok()?;
                let avg_c: f64 = row.try_get::<f64, _>("avg_temp_c").ok()?;
                Some(crate::models::seasonal_plan::DailySoilTempAvg {
                    date: day,
                    avg_temp_f: celsius_to_fahrenheit(avg_c),
                })
            })
            .collect();

        Ok(results)
    }

    /// Fetch daily average paired (air_temp, soil_temp_10) values.
    /// Returns (date, avg_air_temp_f, avg_soil_temp_10_f) tuples.
    /// Used by the soil temperature prediction model.
    pub async fn fetch_daily_paired_averages(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(NaiveDate, f64, f64)>> {
        let rows = sqlx::query(
            r#"
            SELECT
                DATE(utc_datetime) as day,
                AVG(t_calc) as avg_air_c,
                AVG(soil_temp_10) as avg_soil_c,
                COUNT(*) as readings
            FROM observations
            WHERE wbanno = $1
              AND utc_datetime >= $2
              AND utc_datetime <= $3
              AND t_calc IS NOT NULL
              AND t_calc > -9999
              AND soil_temp_10 IS NOT NULL
              AND soil_temp_10 > -9999
            GROUP BY DATE(utc_datetime)
            HAVING COUNT(*) >= 12
            ORDER BY day ASC
            "#,
        )
        .bind(self.station_wbanno)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let results: Vec<(NaiveDate, f64, f64)> = rows
            .iter()
            .filter_map(|row| {
                let day: NaiveDate = row.try_get("day").ok()?;
                let avg_air_c: f64 = row.try_get::<f64, _>("avg_air_c").ok()?;
                let avg_soil_c: f64 = row.try_get::<f64, _>("avg_soil_c").ok()?;
                Some((
                    day,
                    celsius_to_fahrenheit(avg_air_c),
                    celsius_to_fahrenheit(avg_soil_c),
                ))
            })
            .collect();

        Ok(results)
    }

    pub async fn test_connection(&self) -> Result<bool> {
        let result = sqlx::query("SELECT 1").fetch_one(&self.pool).await;

        Ok(result.is_ok())
    }
}
