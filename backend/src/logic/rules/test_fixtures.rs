//! Inputs the rule tests build: a lawn, log entries, an environmental summary and a forecast.

use crate::models::{
    Application, ApplicationType, DailyForecast, DataSource, EnvironmentalReading,
    EnvironmentalSummary, ForecastLocation, ForecastPoint, GrassType, LawnProfile, Recommendation,
    WeatherCondition, WeatherForecast,
};
use chrono::{Duration, NaiveDate, Utc};

/// Today on the host's clock, for tests that judge the rules at the real time.
pub fn today() -> NaiveDate {
    crate::clock::Clock::System.today()
}

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// A 5000 sq ft lawn of `grass`.
pub fn profile(grass: GrassType) -> LawnProfile {
    LawnProfile {
        id: Some(1),
        name: "Test".into(),
        grass_type: grass,
        usda_zone: "7a".into(),
        soil_type: None,
        lawn_size_sqft: Some(5000.0),
        irrigation_type: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub fn fescue() -> LawnProfile {
    profile(GrassType::TallFescue)
}

/// A lawn log entry. `rate` is product lb per 1000 sq ft.
pub fn app(
    kind: ApplicationType,
    on: NaiveDate,
    n_pct: Option<f64>,
    rate: Option<f64>,
) -> Application {
    Application {
        id: None,
        lawn_profile_id: 1,
        application_type: kind,
        product_name: None,
        application_date: on,
        rate_per_1000sqft: rate,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: n_pct,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

/// A lawn feeding of 1 lb N / 1000 sq ft (20% N at 5 lb).
pub fn feeding_on(on: NaiveDate) -> Application {
    app(ApplicationType::Fertilizer, on, Some(20.0), Some(5.0))
}

/// The 7-day soil average and the live reading (10 cm soil temperature and moisture,
/// patio air); everything else is missing.
pub fn soil(avg: Option<f64>, live: Option<f64>) -> EnvironmentalSummary {
    let mut reading = EnvironmentalReading::new(DataSource::SoilData);
    reading.soil_temp_10_f = live;
    EnvironmentalSummary {
        current: Some(reading),
        soil_temp_7day_avg_f: avg,
        ..Default::default()
    }
}

/// A live reading with only air temperature and soil moisture.
pub fn conditions(air: Option<f64>, moisture: Option<f64>) -> EnvironmentalSummary {
    let mut reading = EnvironmentalReading::new(DataSource::SoilData);
    reading.ambient_temp_f = air;
    reading.soil_moisture_10 = moisture;
    EnvironmentalSummary {
        current: Some(reading),
        ..Default::default()
    }
}

/// A full (24 h) forecast day: high, low, rain mm, rain probability.
pub fn day(on: NaiveDate, high: f64, low: f64, mm: f64, prob: f64) -> DailyForecast {
    DailyForecast {
        date: on,
        high_temp_f: high,
        low_temp_f: low,
        avg_humidity: 60.0,
        total_precipitation_mm: mm,
        max_precipitation_prob: prob,
        dominant_condition: WeatherCondition::default(),
        avg_wind_speed_mph: 4.0,
        max_wind_gust_mph: None,
        hours_covered: 24,
    }
}

/// A 3-hourly forecast point `hours` from now.
pub fn point(hours: i64, mm: f64, prob: f64) -> ForecastPoint {
    ForecastPoint {
        timestamp: Utc::now() + Duration::hours(hours),
        temp_f: 70.0,
        feels_like_f: 70.0,
        humidity_percent: 60.0,
        precipitation_mm: mm,
        precipitation_prob: prob,
        wind_speed_mph: 4.0,
        wind_gust_mph: None,
        cloud_cover_percent: 20.0,
        weather_condition: WeatherCondition::default(),
    }
}

pub fn forecast(daily: Vec<DailyForecast>, hourly: Vec<ForecastPoint>) -> WeatherForecast {
    WeatherForecast {
        fetched_at: Utc::now(),
        location: ForecastLocation {
            city: "Test".into(),
            country: "US".into(),
            latitude: 39.8,
            longitude: -75.8,
            utc_offset_seconds: 0,
        },
        hourly,
        daily_summary: daily,
    }
}

/// Days `today`, `today + 1`, … each with the given high (low 20°F under it), dry.
pub fn dry_days_from_today(highs: &[f64]) -> Vec<DailyForecast> {
    highs
        .iter()
        .enumerate()
        .map(|(i, high)| {
            day(
                today() + Duration::days(i as i64),
                *high,
                high - 20.0,
                0.0,
                0.0,
            )
        })
        .collect()
}

/// Every piece of text a recommendation shows.
pub fn all_text(rec: &Recommendation) -> String {
    let points: Vec<String> = rec
        .data_points
        .iter()
        .map(|d| format!("{}: {}", d.label, d.value))
        .collect();
    format!(
        "{}\n{}\n{}\n{}\n{}",
        rec.title,
        rec.description,
        rec.explanation,
        rec.suggested_action.as_deref().unwrap_or_default(),
        points.join("\n")
    )
}
