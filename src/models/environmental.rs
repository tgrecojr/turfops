use super::forecast::WeatherForecast;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSource {
    SoilData,
    HomeAssistant,
    Cached,
    Manual,
}

impl DataSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataSource::SoilData => "NOAA USCRN",
            DataSource::HomeAssistant => "Patio Sensor",
            DataSource::Cached => "Cached",
            DataSource::Manual => "Manual",
        }
    }
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalReading {
    pub timestamp: DateTime<Utc>,
    pub source: DataSource,
    pub soil_temp_5_f: Option<f64>,
    pub soil_temp_10_f: Option<f64>,
    pub soil_temp_20_f: Option<f64>,
    pub soil_temp_50_f: Option<f64>,
    pub soil_temp_100_f: Option<f64>,
    pub soil_moisture_5: Option<f64>,
    pub soil_moisture_10: Option<f64>,
    pub soil_moisture_20: Option<f64>,
    pub soil_moisture_50: Option<f64>,
    pub soil_moisture_100: Option<f64>,
    pub ambient_temp_f: Option<f64>,
    pub humidity_percent: Option<f64>,
    pub precipitation_mm: Option<f64>,
}

impl EnvironmentalReading {
    pub fn new(source: DataSource) -> Self {
        Self {
            timestamp: Utc::now(),
            source,
            soil_temp_5_f: None,
            soil_temp_10_f: None,
            soil_temp_20_f: None,
            soil_temp_50_f: None,
            soil_temp_100_f: None,
            soil_moisture_5: None,
            soil_moisture_10: None,
            soil_moisture_20: None,
            soil_moisture_50: None,
            soil_moisture_100: None,
            ambient_temp_f: None,
            humidity_percent: None,
            precipitation_mm: None,
        }
    }

    pub fn primary_soil_moisture(&self) -> Option<f64> {
        self.soil_moisture_10
            .or(self.soil_moisture_5)
            .or(self.soil_moisture_20)
    }
}

impl Default for EnvironmentalReading {
    fn default() -> Self {
        Self::new(DataSource::Cached)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentalSummary {
    pub current: Option<EnvironmentalReading>,
    pub soil_temp_7day_avg_f: Option<f64>,
    pub ambient_temp_7day_avg_f: Option<f64>,
    pub humidity_7day_avg: Option<f64>,
    pub precipitation_7day_total_mm: Option<f64>,
    pub soil_temp_trend: Trend,
    pub last_updated: Option<DateTime<Utc>>,
    /// Weather forecast data (5-day/3-hour) from OpenWeatherMap
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast: Option<WeatherForecast>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trend {
    Rising,
    Falling,
    #[default]
    Stable,
    Unknown,
}

impl Trend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Trend::Rising => "↑ Rising",
            Trend::Falling => "↓ Falling",
            Trend::Stable => "→ Stable",
            Trend::Unknown => "? Unknown",
        }
    }
}

impl std::fmt::Display for Trend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn celsius_to_fahrenheit(c: f64) -> f64 {
    c * 9.0 / 5.0 + 32.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celsius_to_fahrenheit_known_values() {
        // Freezing point of water
        assert!((celsius_to_fahrenheit(0.0) - 32.0).abs() < 0.001);
        // Boiling point of water
        assert!((celsius_to_fahrenheit(100.0) - 212.0).abs() < 0.001);
        // Body temperature
        assert!((celsius_to_fahrenheit(37.0) - 98.6).abs() < 0.1);
        // -40 is same in both scales
        assert!((celsius_to_fahrenheit(-40.0) - (-40.0)).abs() < 0.001);
    }

    #[test]
    fn agronomic_temperatures() {
        // Pre-emergent window: 50-60°F = ~10-15.5°C
        assert!((celsius_to_fahrenheit(10.0) - 50.0).abs() < 0.1);
        assert!((celsius_to_fahrenheit(15.5) - 59.9).abs() < 0.1);

        // Grub control window: 60-75°F = ~15.5-24°C
        assert!((celsius_to_fahrenheit(15.5) - 59.9).abs() < 0.1);
        assert!((celsius_to_fahrenheit(24.0) - 75.2).abs() < 0.1);

        // Heat stress threshold: 85°F = ~29.4°C
        assert!((celsius_to_fahrenheit(29.4) - 84.9).abs() < 0.2);
    }

    #[test]
    fn environmental_reading_primary_soil_moisture() {
        let mut reading = EnvironmentalReading::new(DataSource::SoilData);

        // No moisture set - should return None
        assert!(reading.primary_soil_moisture().is_none());

        // Only 20cm set
        reading.soil_moisture_20 = Some(0.30);
        assert_eq!(reading.primary_soil_moisture(), Some(0.30));

        // 10cm takes precedence
        reading.soil_moisture_10 = Some(0.25);
        assert_eq!(reading.primary_soil_moisture(), Some(0.25));
    }

    #[test]
    fn data_source_display() {
        assert_eq!(DataSource::SoilData.as_str(), "NOAA USCRN");
        assert_eq!(DataSource::HomeAssistant.as_str(), "Patio Sensor");
        assert_eq!(DataSource::Cached.as_str(), "Cached");
        assert_eq!(DataSource::Manual.as_str(), "Manual");
    }

    #[test]
    fn trend_display() {
        assert!(Trend::Rising.as_str().contains("Rising"));
        assert!(Trend::Falling.as_str().contains("Falling"));
        assert!(Trend::Stable.as_str().contains("Stable"));
        assert!(Trend::Unknown.as_str().contains("Unknown"));
    }
}
