//! When the weather holds a watering reminder back, and what the reminder shows.

use super::tests::{make_plant, make_task};
use super::*;
use crate::logic::rules::thresholds::{
    PRECIP_HEAVY_7DAY_MM, PRECIP_TRACE_MM, SOIL_MOISTURE_ADEQUATE,
};
use crate::models::{
    EnvironmentalReading, ForecastLocation, ForecastPoint, Severity, WeatherCondition,
    WeatherForecast,
};
use chrono::{Duration, Utc};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn feed(plant: &Plant, env: &EnvironmentalSummary, today: NaiveDate) -> Vec<Recommendation> {
    generate_plant_maintenance_recommendations(std::slice::from_ref(plant), &[], env, today)
}

fn point(label: &str, rec: &Recommendation) -> Option<String> {
    rec.data_points
        .iter()
        .find(|p| p.label == label)
        .map(|p| p.value.clone())
}

fn forecast(points: &[(i64, f64)]) -> WeatherForecast {
    WeatherForecast {
        fetched_at: Utc::now(),
        location: ForecastLocation {
            city: "Test".into(),
            country: "US".into(),
            latitude: 0.0,
            longitude: 0.0,
            utc_offset_seconds: 0,
        },
        hourly: points
            .iter()
            .map(|(hours, mm)| ForecastPoint {
                timestamp: Utc::now() + Duration::hours(*hours),
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

fn reading(m10: Option<f64>, m5: Option<f64>, m20: Option<f64>) -> EnvironmentalReading {
    let mut r = EnvironmentalReading::new(DataSource::SoilData);
    r.soil_moisture_10 = m10;
    r.soil_moisture_5 = m5;
    r.soil_moisture_20 = m20;
    r
}

/// Whether a Watering task in its window is recommended under `env`.
fn waters(env: &EnvironmentalSummary) -> bool {
    let plant = make_plant(
        7,
        vec![make_task(
            TaskType::Watering,
            "06-01",
            "09-30",
            Severity::Advisory,
        )],
    );
    !feed(&plant, env, d(2026, 7, 15)).is_empty()
}

fn with_forecast(points: &[(i64, f64)]) -> EnvironmentalSummary {
    EnvironmentalSummary {
        forecast: Some(forecast(points)),
        ..Default::default()
    }
}

// @spec PLANT-WATER-001
#[test]
fn rain_forecast_within_48_hours_holds_the_watering_reminder() {
    assert!(
        !waters(&with_forecast(&[(0, 1.5), (45, 1.0)])),
        "2.5 mm in 48 h"
    );
    assert!(waters(&with_forecast(&[(0, 1.4), (45, 1.0)])), "2.4 mm");
    assert!(
        waters(&with_forecast(&[(0, 0.5), (51, 10.0)])),
        "rain after 48 h"
    );
}

// @spec PLANT-WATER-002
#[test]
fn an_inch_of_rain_in_the_last_week_holds_the_watering_reminder() {
    let week = |mm: f64| EnvironmentalSummary {
        precipitation_7day_total_mm: Some(mm),
        ..Default::default()
    };
    assert!(!waters(&week(25.0)));
    assert!(waters(&week(24.9)));
}

// @spec PLANT-WATER-003
#[test]
fn moist_soil_at_the_preferred_depth_holds_the_watering_reminder() {
    let soil = |r: EnvironmentalReading| EnvironmentalSummary {
        current: Some(r),
        ..Default::default()
    };
    assert!(!waters(&soil(reading(Some(0.20), None, None))));
    assert!(waters(&soil(reading(Some(0.19), None, None))));
    assert!(
        !waters(&soil(reading(None, Some(0.25), Some(0.05)))),
        "5 cm next"
    );
    assert!(
        !waters(&soil(reading(None, None, Some(0.22)))),
        "then 20 cm"
    );
    assert!(
        waters(&soil(reading(Some(0.10), Some(0.30), None))),
        "10 cm wins"
    );
}

// @spec PLANT-WATER-004
#[test]
fn the_hold_back_uses_the_irrigation_rules_own_thresholds() {
    assert_eq!(
        (
            PRECIP_TRACE_MM,
            PRECIP_HEAVY_7DAY_MM,
            SOIL_MOISTURE_ADEQUATE
        ),
        (2.5, 25.0, 0.20)
    );
    let at = |forecast_mm, week_mm, moisture| WaterSignals {
        forecast_rain_mm: Some(forecast_mm),
        recent_rain_mm: Some(week_mm),
        soil_moisture: Some(moisture),
    };
    assert!(at(PRECIP_TRACE_MM, 0.0, 0.0).wet_reason().is_some());
    assert!(at(0.0, PRECIP_HEAVY_7DAY_MM, 0.0).wet_reason().is_some());
    assert!(at(0.0, 0.0, SOIL_MOISTURE_ADEQUATE).wet_reason().is_some());
    let just_under = at(
        PRECIP_TRACE_MM - 0.01,
        PRECIP_HEAVY_7DAY_MM - 0.01,
        SOIL_MOISTURE_ADEQUATE - 0.001,
    );
    assert!(just_under.wet_reason().is_none());
}

// @spec PLANT-WATER-005
#[test]
fn a_missing_signal_never_holds_the_reminder_back() {
    assert!(waters(&EnvironmentalSummary::default()));
    let only_dry_week = EnvironmentalSummary {
        precipitation_7day_total_mm: Some(1.0),
        ..Default::default()
    };
    assert!(waters(&only_dry_week));
}

// @spec PLANT-WATER-006
#[test]
fn a_watering_reminder_shows_the_signals_that_are_known_in_inches() {
    let plant = make_plant(
        7,
        vec![make_task(
            TaskType::Watering,
            "06-01",
            "09-30",
            Severity::Advisory,
        )],
    );
    let today = d(2026, 7, 15);
    let env = EnvironmentalSummary {
        precipitation_7day_total_mm: Some(12.7),
        forecast: Some(forecast(&[(0, 0.0), (3, 1.27)])),
        current: Some(reading(Some(0.12), None, None)),
        ..Default::default()
    };
    let rec = &feed(&plant, &env, today)[0];
    assert_eq!(point("Rain last 7 d", rec).as_deref(), Some("0.50\""));
    assert_eq!(
        point("Rain forecast", rec).as_deref(),
        Some("0.05\" in 48 h")
    );
    assert_eq!(point("Soil Moisture", rec).as_deref(), Some("12%"));

    let rec = &feed(&plant, &EnvironmentalSummary::default(), today)[0];
    for label in ["Rain last 7 d", "Rain forecast", "Soil Moisture"] {
        assert_eq!(point(label, rec), None, "{label} is unknown");
    }
}

// @spec PLANT-WATER-007
#[test]
fn a_watering_plan_activity_ignores_the_weather() {
    let plant = make_plant(
        7,
        vec![make_task(
            TaskType::Watering,
            "06-01",
            "09-30",
            Severity::Advisory,
        )],
    );
    let today = d(2026, 7, 15);
    let soaked = EnvironmentalSummary {
        precipitation_7day_total_mm: Some(80.0),
        ..Default::default()
    };
    assert!(feed(&plant, &soaked, today).is_empty(), "held in the feed");
    let plan = build_plant_activities(&[plant], &[], 2026, today);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].status, ActivityStatus::Active);
}
