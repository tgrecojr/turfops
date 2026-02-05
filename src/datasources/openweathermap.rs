use crate::config::OpenWeatherMapConfig;
use crate::error::{Result, TurfOpsError};
use crate::models::forecast::{
    DailyForecast, ForecastLocation, ForecastPoint, WeatherCondition, WeatherForecast,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use std::collections::HashMap;

const API_BASE_URL: &str = "https://api.openweathermap.org/data/2.5";

pub struct OpenWeatherMapClient {
    client: reqwest::Client,
    config: OpenWeatherMapConfig,
}

// OpenWeatherMap API response structures
#[derive(Debug, Deserialize)]
struct OwmForecastResponse {
    list: Vec<OwmForecastItem>,
    city: OwmCity,
}

#[derive(Debug, Deserialize)]
struct OwmForecastItem {
    dt: i64,
    main: OwmMain,
    weather: Vec<OwmWeather>,
    clouds: OwmClouds,
    wind: OwmWind,
    #[serde(default)]
    pop: f64, // probability of precipitation
    #[serde(default)]
    rain: Option<OwmPrecipitation>,
    #[serde(default)]
    snow: Option<OwmPrecipitation>,
}

#[derive(Debug, Deserialize)]
struct OwmMain {
    temp: f64,
    feels_like: f64,
    humidity: f64,
}

#[derive(Debug, Deserialize)]
struct OwmWeather {
    id: u32,
    #[allow(dead_code)]
    main: String,
    #[allow(dead_code)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct OwmClouds {
    all: f64, // cloudiness percentage
}

#[derive(Debug, Deserialize)]
struct OwmWind {
    speed: f64,
    #[serde(default)]
    gust: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct OwmPrecipitation {
    #[serde(rename = "3h", default)]
    three_hour: f64,
}

#[derive(Debug, Deserialize)]
struct OwmCity {
    name: String,
    country: String,
    coord: OwmCoord,
}

#[derive(Debug, Deserialize)]
struct OwmCoord {
    lat: f64,
    lon: f64,
}

impl OpenWeatherMapClient {
    pub fn new(config: OpenWeatherMapConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Fetch 5-day/3-hour forecast from OpenWeatherMap
    pub async fn fetch_forecast(&self) -> Result<WeatherForecast> {
        let url = format!(
            "{}/forecast?lat={}&lon={}&appid={}&units=imperial",
            API_BASE_URL, self.config.latitude, self.config.longitude, self.config.api_key
        );

        let response =
            self.client.get(&url).send().await.map_err(|e| {
                TurfOpsError::DataSourceUnavailable(format!("OpenWeatherMap: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TurfOpsError::DataSourceUnavailable(format!(
                "OpenWeatherMap returned {}: {}",
                status, body
            )));
        }

        let owm_response: OwmForecastResponse = response.json().await.map_err(|e| {
            TurfOpsError::DataSourceUnavailable(format!(
                "Failed to parse OpenWeatherMap response: {}",
                e
            ))
        })?;

        Ok(self.convert_response(owm_response))
    }

    /// Test connection to OpenWeatherMap API
    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!(
            "{}/weather?lat={}&lon={}&appid={}&units=imperial",
            API_BASE_URL, self.config.latitude, self.config.longitude, self.config.api_key
        );

        let response =
            self.client.get(&url).send().await.map_err(|e| {
                TurfOpsError::DataSourceUnavailable(format!("OpenWeatherMap: {}", e))
            })?;

        Ok(response.status().is_success())
    }

    fn convert_response(&self, response: OwmForecastResponse) -> WeatherForecast {
        let location = ForecastLocation {
            city: response.city.name,
            country: response.city.country,
            latitude: response.city.coord.lat,
            longitude: response.city.coord.lon,
        };

        let hourly: Vec<ForecastPoint> = response
            .list
            .iter()
            .map(|item| self.convert_forecast_item(item))
            .collect();

        let daily_summary = self.aggregate_daily(&hourly);

        WeatherForecast {
            fetched_at: Utc::now(),
            location,
            hourly,
            daily_summary,
        }
    }

    fn convert_forecast_item(&self, item: &OwmForecastItem) -> ForecastPoint {
        let timestamp = DateTime::from_timestamp(item.dt, 0).unwrap_or_else(Utc::now);

        let weather_condition = item
            .weather
            .first()
            .map(|w| WeatherCondition::from_owm_id(w.id))
            .unwrap_or_default();

        // Combine rain and snow precipitation
        let rain_mm = item.rain.as_ref().map(|r| r.three_hour).unwrap_or(0.0);
        let snow_mm = item.snow.as_ref().map(|s| s.three_hour).unwrap_or(0.0);
        let precipitation_mm = rain_mm + snow_mm;

        ForecastPoint {
            timestamp,
            temp_f: item.main.temp,
            feels_like_f: item.main.feels_like,
            humidity_percent: item.main.humidity,
            precipitation_mm,
            precipitation_prob: item.pop,
            wind_speed_mph: item.wind.speed,
            wind_gust_mph: item.wind.gust,
            cloud_cover_percent: item.clouds.all,
            weather_condition,
        }
    }

    fn aggregate_daily(&self, hourly: &[ForecastPoint]) -> Vec<DailyForecast> {
        // Group by date
        let mut by_date: HashMap<NaiveDate, Vec<&ForecastPoint>> = HashMap::new();
        for point in hourly {
            let date = point.timestamp.date_naive();
            by_date.entry(date).or_default().push(point);
        }

        // Convert to sorted daily summaries
        let mut days: Vec<DailyForecast> = by_date
            .into_iter()
            .map(|(date, points)| self.aggregate_day(date, &points))
            .collect();

        days.sort_by_key(|d| d.date);
        days
    }

    fn aggregate_day(&self, date: NaiveDate, points: &[&ForecastPoint]) -> DailyForecast {
        let high_temp_f = points
            .iter()
            .map(|p| p.temp_f)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let low_temp_f = points
            .iter()
            .map(|p| p.temp_f)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let avg_humidity: f64 =
            points.iter().map(|p| p.humidity_percent).sum::<f64>() / points.len().max(1) as f64;

        let total_precipitation_mm: f64 = points.iter().map(|p| p.precipitation_mm).sum();

        let max_precipitation_prob = points
            .iter()
            .map(|p| p.precipitation_prob)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        // Find dominant weather condition (most frequent)
        let mut condition_counts: HashMap<WeatherCondition, usize> = HashMap::new();
        for point in points {
            *condition_counts.entry(point.weather_condition).or_insert(0) += 1;
        }
        let dominant_condition = condition_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(condition, _)| condition)
            .unwrap_or_default();

        let avg_wind_speed_mph: f64 =
            points.iter().map(|p| p.wind_speed_mph).sum::<f64>() / points.len().max(1) as f64;

        let max_wind_gust_mph = points
            .iter()
            .filter_map(|p| p.wind_gust_mph)
            .max_by(|a, b| a.partial_cmp(b).unwrap());

        DailyForecast {
            date,
            high_temp_f,
            low_temp_f,
            avg_humidity,
            total_precipitation_mm,
            max_precipitation_prob,
            dominant_condition,
            avg_wind_speed_mph,
            max_wind_gust_mph,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> OpenWeatherMapConfig {
        OpenWeatherMapConfig {
            api_key: "test_key".to_string(),
            latitude: 39.8561,
            longitude: -75.7872,
            enabled: true,
        }
    }

    #[test]
    fn client_creation() {
        let client = OpenWeatherMapClient::new(sample_config());
        assert!(client.config.enabled);
    }
}
