//! Weather gate for landscape-plant Watering tasks. A plant's care plan says "water
//! during dry spells" across a calendar window; this decides whether today is one.
//! Same signals as the lawn's irrigation rule: station soil moisture, the lake's
//! 7-day rain total, and the OpenWeatherMap forecast (whose first slot is the current
//! weather, so rain falling now counts). Pure — no IO.

use crate::logic::rules::thresholds::{
    PRECIP_HEAVY_7DAY_MM, PRECIP_TRACE_MM, SOIL_MOISTURE_ADEQUATE,
};
use crate::models::{DataSource, EnvironmentalSummary, Recommendation};

/// How far ahead forecast rain relieves a watering reminder.
pub const RAIN_FORECAST_HOURS: u32 = 48;

/// What the weather is doing for the plants right now.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WaterSignals {
    /// Lake 7-day rain total (mm); lags about a day.
    pub recent_rain_mm: Option<f64>,
    /// Forecast rain (mm) over the next [`RAIN_FORECAST_HOURS`], including the current slot.
    pub forecast_rain_mm: Option<f64>,
    /// Station soil moisture (fraction), 10 cm preferred.
    pub soil_moisture: Option<f64>,
}

impl WaterSignals {
    pub fn from_env(env: &EnvironmentalSummary) -> Self {
        Self {
            recent_rain_mm: env.precipitation_7day_total_mm,
            forecast_rain_mm: env.forecast.as_ref().map(|f| {
                f.next_hours(RAIN_FORECAST_HOURS)
                    .iter()
                    .map(|p| p.precipitation_mm)
                    .sum()
            }),
            soil_moisture: env.current.as_ref().and_then(|c| c.primary_soil_moisture()),
        }
    }

    /// Why the weather is watering for you today, or `None` when it is a dry spell (or
    /// nothing is known — no data keeps the reminder, like every other rule).
    pub fn wet_reason(&self) -> Option<String> {
        if let Some(mm) = self.forecast_rain_mm.filter(|mm| *mm >= PRECIP_TRACE_MM) {
            return Some(format!(
                "{:.2}\" of rain forecast within {} h",
                mm / 25.4,
                RAIN_FORECAST_HOURS
            ));
        }
        if let Some(mm) = self.recent_rain_mm.filter(|mm| *mm >= PRECIP_HEAVY_7DAY_MM) {
            return Some(format!("{:.2}\" of rain in the last 7 days", mm / 25.4));
        }
        if let Some(m) = self.soil_moisture.filter(|m| *m >= SOIL_MOISTURE_ADEQUATE) {
            return Some(format!("soil moisture adequate ({:.0}%)", m * 100.0));
        }
        None
    }

    /// The signals a dry-spell reminder was judged on, so the card explains itself.
    pub fn annotate(&self, mut rec: Recommendation) -> Recommendation {
        if let Some(mm) = self.recent_rain_mm {
            rec = rec.with_data_point(
                "Rain last 7 d",
                format!("{:.2}\"", mm / 25.4),
                DataSource::SoilData.as_str(),
            );
        }
        if let Some(mm) = self.forecast_rain_mm {
            rec = rec.with_data_point(
                "Rain forecast",
                format!("{:.2}\" in {} h", mm / 25.4, RAIN_FORECAST_HOURS),
                DataSource::OpenWeatherMap.as_str(),
            );
        }
        if let Some(m) = self.soil_moisture {
            rec = rec.with_data_point(
                "Soil Moisture",
                format!("{:.0}%", m * 100.0),
                DataSource::SoilData.as_str(),
            );
        }
        rec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        EnvironmentalReading, ForecastLocation, ForecastPoint, WeatherCondition, WeatherForecast,
    };
    use chrono::{Duration, Utc};

    fn forecast(points_mm: &[f64]) -> WeatherForecast {
        WeatherForecast {
            fetched_at: Utc::now(),
            location: ForecastLocation {
                city: "Test".into(),
                country: "US".into(),
                latitude: 0.0,
                longitude: 0.0,
                utc_offset_seconds: 0,
            },
            hourly: points_mm
                .iter()
                .enumerate()
                .map(|(i, mm)| ForecastPoint {
                    timestamp: Utc::now() + Duration::hours(3 * i as i64),
                    temp_f: 70.0,
                    feels_like_f: 70.0,
                    humidity_percent: 60.0,
                    precipitation_mm: *mm,
                    precipitation_prob: 0.0,
                    wind_speed_mph: 5.0,
                    wind_gust_mph: None,
                    cloud_cover_percent: 20.0,
                    weather_condition: WeatherCondition::default(),
                })
                .collect(),
            daily_summary: Vec::new(),
        }
    }

    fn env(
        recent_mm: Option<f64>,
        forecast_mm: Option<&[f64]>,
        moisture: Option<f64>,
    ) -> EnvironmentalSummary {
        let current = moisture.map(|m| {
            let mut r = EnvironmentalReading::new(DataSource::SoilData);
            r.soil_moisture_10 = Some(m);
            r
        });
        EnvironmentalSummary {
            current,
            precipitation_7day_total_mm: recent_mm,
            forecast: forecast_mm.map(forecast),
            ..Default::default()
        }
    }

    #[test]
    fn no_data_is_a_dry_spell() {
        assert_eq!(
            WaterSignals::from_env(&EnvironmentalSummary::default()).wet_reason(),
            None
        );
    }

    #[test]
    fn a_genuinely_dry_week_keeps_the_reminder() {
        let s = WaterSignals::from_env(&env(Some(3.0), Some(&[0.0, 0.5, 0.0]), Some(0.12)));
        assert_eq!(s.wet_reason(), None);
    }

    #[test]
    fn rain_falling_now_or_forecast_relieves_it() {
        // 0.46" in the current + next slots, like the rain-delay alert that fired beside it.
        let s = WaterSignals::from_env(&env(Some(0.0), Some(&[6.0, 5.7, 0.0]), Some(0.12)));
        assert!(s.wet_reason().unwrap().contains("forecast"));
    }

    #[test]
    fn a_wet_week_in_the_lake_relieves_it() {
        let s = WaterSignals::from_env(&env(Some(30.0), Some(&[0.0]), Some(0.12)));
        assert!(s.wet_reason().unwrap().contains("last 7 days"));
    }

    #[test]
    fn adequate_soil_moisture_relieves_it() {
        let s = WaterSignals::from_env(&env(Some(0.0), Some(&[0.0]), Some(0.25)));
        assert!(s.wet_reason().unwrap().contains("adequate"));
    }

    #[test]
    fn a_trace_does_not_count() {
        let s = WaterSignals::from_env(&env(Some(10.0), Some(&[1.0, 1.0]), None));
        assert_eq!(s.wet_reason(), None);
    }
}
