//! Each trim checked against a body serialized from its handler's own response model, so
//! renaming or removing a field a trim names fails here instead of leaving bulk in a reply
//! or a picked field `null`.

use super::trim;
use crate::logic::disease::{self, test_support::*};
use crate::logic::timing::context;
use crate::models::disease::DiseaseRiskResponse;
use crate::models::gdd::{CrabgrassModel, CrabgrassStatus, DailyGdd, GddSummary};
use crate::models::nitrogen_budget::{GrassTypeNTarget, NitrogenApplication, NitrogenBudget};
use crate::models::timing::{FreezeDates, SoilNow, TimingResponse};
use crate::models::{
    Application, ApplicationType, DailyForecast, EnvironmentalSummary, ForecastLocation,
    ForecastPoint, GrassType, WeatherCondition, WeatherForecast,
};
use chrono::{NaiveDate, Utc};
use serde_json::Value;

fn today() -> NaiveDate {
    date(7, 22)
}

fn has(value: &Value, key: &str) -> bool {
    value.get(key).is_some()
}

// @spec MCP-TOOL-027
#[test]
fn the_hourly_forecast_trim_matches_the_environmental_summary() {
    let point = ForecastPoint {
        timestamp: Utc::now(),
        temp_f: 80.0,
        feels_like_f: 82.0,
        humidity_percent: 70.0,
        precipitation_mm: 0.0,
        precipitation_prob: 0.1,
        wind_speed_mph: 5.0,
        wind_gust_mph: None,
        cloud_cover_percent: 20.0,
        weather_condition: WeatherCondition::Clear,
    };
    let daily = DailyForecast {
        date: today(),
        high_temp_f: 85.0,
        low_temp_f: 65.0,
        avg_humidity: 70.0,
        total_precipitation_mm: 0.0,
        max_precipitation_prob: 0.1,
        dominant_condition: WeatherCondition::Clear,
        avg_wind_speed_mph: 5.0,
        max_wind_gust_mph: None,
        hours_covered: 24,
    };
    let summary = EnvironmentalSummary {
        forecast: Some(WeatherForecast {
            fetched_at: Utc::now(),
            location: ForecastLocation {
                city: "Avondale".into(),
                country: "US".into(),
                latitude: 39.8,
                longitude: -75.8,
                utc_offset_seconds: -14_400,
            },
            hourly: vec![point],
            daily_summary: vec![daily],
        }),
        ..Default::default()
    };
    let mut body = serde_json::to_value(&summary).unwrap();
    assert!(
        has(&body["forecast"], "hourly"),
        "the field the trim drops exists"
    );
    trim::drop_hourly_forecast(&mut body);
    assert!(!has(&body["forecast"], "hourly"));
    assert!(has(&body["forecast"], "daily_summary"));
}

// @spec MCP-TOOL-027
#[test]
fn the_series_trim_matches_the_timing_response() {
    let response = TimingResponse {
        generated_at: Utc::now(),
        today: today(),
        station: "NOAA USCRN station 3761".into(),
        soil: SoilNow {
            avg_5day_f: Some(70.0),
            as_of: Some(today()),
            depth_cm: 5,
        },
        freeze: FreezeDates {
            last_spring: None,
            first_fall: None,
            last_spring_this_year: None,
            first_fall_this_year: None,
        },
        windows: vec![],
        context: context::build_context(&[], &[], &[]),
        series: context::build_series(&[], today(), &[]),
        history_years: vec![],
        data_notes: vec![],
    };
    let body = serde_json::to_value(&response).unwrap();
    assert!(has(&body, "series"));
    let trimmed = trim::timing_without_series(body);
    assert!(!has(&trimmed, "series"));
    assert!(has(&trimmed, "windows") && has(&trimmed, "data_notes"));
}

// @spec MCP-TOOL-027
#[test]
fn the_daily_history_trim_matches_the_gdd_summary() {
    let gdd = GddSummary {
        year: 2026,
        current_gdd_total: 1200.0,
        daily_history: vec![DailyGdd {
            date: today(),
            high_temp_f: 88.0,
            low_temp_f: 68.0,
            gdd_base50: 28.0,
            cumulative_gdd_base50: 1200.0,
        }],
        crabgrass_model: CrabgrassModel {
            germination_threshold: 200.0,
            current_gdd: 1200.0,
            status: CrabgrassStatus::PostGermination,
            estimated_germination_date: None,
        },
        last_computed_date: Some(today()),
    };
    let body = serde_json::to_value(&gdd).unwrap();
    assert!(has(&body, "daily_history"));
    let trimmed = trim::without(body, &["daily_history"]);
    assert!(!has(&trimmed, "daily_history") && has(&trimmed, "current_gdd_total"));
}

// @spec MCP-TOOL-027
#[test]
fn the_applications_trim_matches_the_nitrogen_budget() {
    let budget = NitrogenBudget {
        year: 2026,
        target_lbs_per_1000sqft: 4.0,
        applied_lbs_per_1000sqft: 1.0,
        remaining_lbs_per_1000sqft: 3.0,
        percent_of_target: 25.0,
        applications: vec![NitrogenApplication {
            date: today(),
            product_name: None,
            nitrogen_pct: 20.0,
            rate_per_1000sqft: 5.0,
            n_lbs_per_1000sqft: 1.0,
        }],
        grass_type_target: GrassTypeNTarget {
            grass_type: GrassType::TallFescue,
            min_lbs_per_1000sqft: 3.0,
            max_lbs_per_1000sqft: 5.0,
            recommended_lbs_per_1000sqft: 4.0,
        },
    };
    let body = serde_json::to_value(&budget).unwrap();
    assert!(has(&body, "applications"));
    let trimmed = trim::nitrogen_totals(body);
    assert!(!has(&trimmed, "applications") && has(&trimmed, "remaining_lbs_per_1000sqft"));
}

/// A hot, humid week with a strobilurin logged five days ago: some diseases come out
/// Protected, others call for a preventative, so every field a disease trim picks is set
/// somewhere.
fn disease_body() -> Value {
    let series = run_of(today(), 12, |d| day(d, 26.0, 21.0, 32.0, 92.0));
    let heritage = Application {
        id: None,
        lawn_profile_id: 1,
        application_type: ApplicationType::Fungicide,
        product_name: Some("Heritage G".into()),
        application_date: today() - chrono::Duration::days(5),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    };
    let ctx = disease::context_from_history(&[heritage], today());
    let response = DiseaseRiskResponse {
        generated_at: Utc::now(),
        today: today(),
        station: "NOAA USCRN station 3761".into(),
        diseases: disease::assess_all(&series, today(), &ctx),
        data_notes: vec!["The lake lags a day.".into()],
    };
    serde_json::to_value(&response).unwrap()
}

/// Every `field` is non-null on at least one disease of `trimmed`.
fn each_field_set_somewhere(trimmed: &Value, fields: &[&str]) {
    let diseases = trimmed["diseases"].as_array().expect("diseases");
    assert!(!diseases.is_empty());
    for field in fields {
        assert!(
            diseases.iter().any(|d| !d[*field].is_null()),
            "`{field}` is null on every disease: the model field it picks was renamed or removed"
        );
    }
}

// @spec MCP-TOOL-027
#[test]
fn the_disease_trims_pick_fields_the_response_still_has() {
    let body = disease_body();
    let headlines = trim::disease_headlines(&body);
    each_field_set_somewhere(
        &headlines,
        &[
            "slug",
            "name",
            "tier",
            "score_label",
            "as_of",
            "action",
            "headline",
            "protected_through",
        ],
    );
    assert_eq!(headlines["today"], body["today"]);
    assert_eq!(headlines["data_notes"], body["data_notes"]);

    let overview = trim::disease_overview(&body);
    each_field_set_somewhere(
        &overview,
        &[
            "summary",
            "factors",
            "action",
            "headline",
            "protection",
            "recommended_preventative",
        ],
    );

    let slug = trim::disease_slugs(&body)[0].clone();
    let detail = trim::disease_detail(&body, &slug).unwrap();
    assert_eq!(detail["disease"]["slug"], slug.as_str());
    assert!(!detail["today"].is_null());
}
