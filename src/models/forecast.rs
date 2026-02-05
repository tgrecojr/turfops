use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Weather forecast data from OpenWeatherMap 5-day/3-hour API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherForecast {
    pub fetched_at: DateTime<Utc>,
    pub location: ForecastLocation,
    pub hourly: Vec<ForecastPoint>,        // 3-hour intervals
    pub daily_summary: Vec<DailyForecast>, // Aggregated by day
}

impl WeatherForecast {
    /// Get forecast points for the next N hours
    pub fn next_hours(&self, hours: u32) -> Vec<&ForecastPoint> {
        let cutoff = Utc::now() + chrono::Duration::hours(hours as i64);
        self.hourly
            .iter()
            .filter(|p| p.timestamp <= cutoff)
            .collect()
    }

    /// Get forecast points for the next N days
    pub fn next_days(&self, days: u32) -> Vec<&DailyForecast> {
        let today = Utc::now().date_naive();
        let cutoff = today + chrono::Duration::days(days as i64);
        self.daily_summary
            .iter()
            .filter(|d| d.date <= cutoff)
            .collect()
    }

    /// Check if significant rain is expected within hours
    pub fn rain_expected_within(&self, hours: u32, threshold_mm: f64) -> Option<RainForecast> {
        let points = self.next_hours(hours);
        let mut total_precip = 0.0;
        let mut max_prob = 0.0;
        let mut first_rain_time: Option<DateTime<Utc>> = None;

        for point in points {
            total_precip += point.precipitation_mm;
            if point.precipitation_prob > max_prob {
                max_prob = point.precipitation_prob;
            }
            if first_rain_time.is_none()
                && (point.precipitation_mm > 0.1 || point.precipitation_prob > 0.5)
            {
                first_rain_time = Some(point.timestamp);
            }
        }

        if total_precip >= threshold_mm || max_prob >= 0.5 {
            Some(RainForecast {
                expected_mm: total_precip,
                max_probability: max_prob,
                first_expected: first_rain_time,
            })
        } else {
            None
        }
    }

    /// Find the maximum temperature in the forecast period
    pub fn max_temp_next_days(&self, days: u32) -> Option<f64> {
        self.next_days(days)
            .iter()
            .map(|d| d.high_temp_f)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Count consecutive days with high humidity
    pub fn consecutive_high_humidity_days(&self, threshold: f64) -> u32 {
        let mut count = 0;
        for day in &self.daily_summary {
            if day.avg_humidity >= threshold {
                count += 1;
            } else {
                break;
            }
        }
        count
    }
}

#[derive(Debug, Clone)]
pub struct RainForecast {
    pub expected_mm: f64,
    pub max_probability: f64,
    pub first_expected: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastLocation {
    pub city: String,
    pub country: String,
    pub latitude: f64,
    pub longitude: f64,
}

/// A single 3-hour forecast point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub timestamp: DateTime<Utc>,
    pub temp_f: f64,
    pub feels_like_f: f64,
    pub humidity_percent: f64,
    pub precipitation_mm: f64,   // rain + snow
    pub precipitation_prob: f64, // 0.0-1.0
    pub wind_speed_mph: f64,
    pub wind_gust_mph: Option<f64>,
    pub cloud_cover_percent: f64,
    pub weather_condition: WeatherCondition,
}

/// Aggregated daily forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyForecast {
    pub date: NaiveDate,
    pub high_temp_f: f64,
    pub low_temp_f: f64,
    pub avg_humidity: f64,
    pub total_precipitation_mm: f64,
    pub max_precipitation_prob: f64,
    pub dominant_condition: WeatherCondition,
    pub avg_wind_speed_mph: f64,
    pub max_wind_gust_mph: Option<f64>,
}

/// Weather condition categories from OpenWeatherMap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WeatherCondition {
    #[default]
    Clear,
    Clouds,
    Rain,
    Drizzle,
    Thunderstorm,
    Snow,
    Mist,
    Fog,
    Other,
}

impl WeatherCondition {
    pub fn from_owm_id(id: u32) -> Self {
        match id {
            200..=232 => WeatherCondition::Thunderstorm,
            300..=321 => WeatherCondition::Drizzle,
            500..=531 => WeatherCondition::Rain,
            600..=622 => WeatherCondition::Snow,
            701 => WeatherCondition::Mist,
            741 => WeatherCondition::Fog,
            800 => WeatherCondition::Clear,
            801..=804 => WeatherCondition::Clouds,
            _ => WeatherCondition::Other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "Clear",
            WeatherCondition::Clouds => "Cloudy",
            WeatherCondition::Rain => "Rain",
            WeatherCondition::Drizzle => "Drizzle",
            WeatherCondition::Thunderstorm => "Thunderstorm",
            WeatherCondition::Snow => "Snow",
            WeatherCondition::Mist => "Mist",
            WeatherCondition::Fog => "Fog",
            WeatherCondition::Other => "Other",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            WeatherCondition::Clear => "☀",
            WeatherCondition::Clouds => "☁",
            WeatherCondition::Rain => "🌧",
            WeatherCondition::Drizzle => "🌦",
            WeatherCondition::Thunderstorm => "⛈",
            WeatherCondition::Snow => "❄",
            WeatherCondition::Mist => "🌫",
            WeatherCondition::Fog => "🌫",
            WeatherCondition::Other => "?",
        }
    }

    /// Whether this condition involves precipitation
    pub fn has_precipitation(&self) -> bool {
        matches!(
            self,
            WeatherCondition::Rain
                | WeatherCondition::Drizzle
                | WeatherCondition::Thunderstorm
                | WeatherCondition::Snow
        )
    }
}

impl std::fmt::Display for WeatherCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_condition_from_owm_id() {
        assert_eq!(
            WeatherCondition::from_owm_id(200),
            WeatherCondition::Thunderstorm
        );
        assert_eq!(WeatherCondition::from_owm_id(500), WeatherCondition::Rain);
        assert_eq!(WeatherCondition::from_owm_id(800), WeatherCondition::Clear);
        assert_eq!(WeatherCondition::from_owm_id(801), WeatherCondition::Clouds);
        assert_eq!(WeatherCondition::from_owm_id(600), WeatherCondition::Snow);
    }

    #[test]
    fn weather_condition_has_precipitation() {
        assert!(WeatherCondition::Rain.has_precipitation());
        assert!(WeatherCondition::Thunderstorm.has_precipitation());
        assert!(!WeatherCondition::Clear.has_precipitation());
        assert!(!WeatherCondition::Clouds.has_precipitation());
    }
}
