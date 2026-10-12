//! Irrigation and the application window: the rules that read the daily forecast.

use super::application_window::ApplicationWindowRule;
use super::irrigation_forecast::IrrigationForecastRule;
use super::test_fixtures::*;
use super::Rule;
use crate::logic::inventory::{enrich, shopping};
use crate::models::{DailyForecast, EnvironmentalSummary, ProductCategory, Severity};
use chrono::Duration;

/// Live soil moisture plus a daily forecast of `(rain mm, probability)` from today.
fn dry_spell(moisture: Option<f64>, rain: &[(f64, f64)]) -> EnvironmentalSummary {
    let days = rain
        .iter()
        .enumerate()
        .map(|(i, (mm, prob))| day(today() + Duration::days(i as i64), 75.0, 55.0, *mm, *prob))
        .collect();
    EnvironmentalSummary {
        forecast: Some(forecast(days, Vec::new())),
        ..conditions(None, moisture)
    }
}

fn irrigation(env: &EnvironmentalSummary) -> Option<Severity> {
    IrrigationForecastRule
        .evaluate(env, &fescue(), &[])
        .map(|r| {
            assert_eq!(r.id, "irrigation_forecast");
            r.severity
        })
}

const DRY_WEEK: [(f64, f64); 5] = [(0.0, 0.0); 5];

// @spec RULE-WX-002
#[test]
fn dry_soil_with_no_real_rain_coming_needs_water() {
    let at = |m| irrigation(&dry_spell(Some(m), &DRY_WEEK));
    assert_eq!(at(0.20), None);
    assert_eq!(at(0.19), Some(Severity::Advisory));
    assert_eq!(at(0.15), Some(Severity::Advisory));
    assert_eq!(at(0.149), Some(Severity::Warning));
    assert_eq!(at(0.10), Some(Severity::Warning));
    assert_eq!(at(0.099), Some(Severity::Critical));
    // Up to 2.5 mm over the five days still leaves the soil dry; more cancels the alert.
    let trace = [(1.0, 0.2), (0.0, 0.0), (1.5, 0.3), (0.0, 0.0), (0.0, 0.0)];
    assert_eq!(
        irrigation(&dry_spell(Some(0.12), &trace)),
        Some(Severity::Warning)
    );
    let shower = [(1.0, 0.2), (0.0, 0.0), (1.6, 0.3), (0.0, 0.0), (0.0, 0.0)];
    assert_eq!(irrigation(&dry_spell(Some(0.12), &shower)), None);
    // Both inputs are required.
    assert_eq!(irrigation(&dry_spell(None, &DRY_WEEK)), None);
    let no_forecast = conditions(None, Some(0.05));
    assert_eq!(irrigation(&no_forecast), None);
}

// @spec RULE-WX-003
#[test]
fn a_high_chance_of_no_rain_does_not_cancel_the_alert() {
    let likely_but_dry = [(0.0, 0.9), (0.0, 0.8), (0.5, 0.7), (0.0, 0.9), (0.0, 0.6)];
    assert_eq!(
        irrigation(&dry_spell(Some(0.08), &likely_but_dry)),
        Some(Severity::Critical)
    );
}

// @spec RULE-WX-004
#[test]
fn the_wetting_agent_is_optional_and_never_shopped_for() {
    let rec = IrrigationForecastRule
        .evaluate(&dry_spell(Some(0.08), &DRY_WEEK), &fescue(), &[])
        .expect("dry soil needs water");
    assert_eq!(rec.needs.len(), 1);
    assert!(rec.needs[0].optional);
    assert_eq!(rec.needs[0].categories, vec![ProductCategory::Surfactant]);
    // An empty shelf: the optional need is not "need to buy" and adds nothing to shop for.
    let mut feed = vec![rec];
    enrich(&mut feed, &[]);
    assert!(shopping::build(&feed, &[]).items.is_empty());
}

fn window(days: Vec<DailyForecast>, lake_rain_mm: f64) -> Option<(String, Severity, String)> {
    let env = EnvironmentalSummary {
        forecast: Some(forecast(days, Vec::new())),
        precipitation_7day_total_mm: Some(lake_rain_mm),
        ..Default::default()
    };
    ApplicationWindowRule
        .evaluate(&env, &fescue(), &[])
        .map(|r| (r.id, r.severity, r.description))
}

fn on(offset: i64) -> chrono::NaiveDate {
    today() + Duration::days(offset)
}

// @spec RULE-WX-006
#[test]
fn the_best_good_day_in_the_next_five_is_announced() {
    let mut windy = day(on(2), 70.0, 50.0, 0.0, 0.0);
    windy.avg_wind_speed_mph = 12.0;
    let days = vec![
        day(on(0), 70.0, 50.0, 5.0, 0.6), // rain
        day(on(1), 70.0, 50.0, 0.0, 0.0), // dry, but the day before was wet
        windy,                            // good, but windy
        day(on(3), 70.0, 50.0, 0.0, 0.0), // good and calm: the best
        day(on(4), 88.0, 65.0, 0.0, 0.0), // too hot
    ];
    let (id, severity, description) = window(days, 0.0).expect("two good days");
    assert_eq!(
        (id.as_str(), severity),
        ("application_window", Severity::Info)
    );
    assert!(
        description.contains(&on(3).format("%b %d").to_string()),
        "{description}"
    );
    assert!(description.contains("2 good day(s)"), "{description}");

    // Rain the next day (2.5 mm, or a 50% chance) spoils a day; so does a mean under 50°F.
    let spoiled = vec![
        day(on(0), 70.0, 50.0, 0.0, 0.0),
        day(on(1), 70.0, 50.0, 2.5, 0.1),
        day(on(2), 70.0, 50.0, 0.0, 0.5),
        day(on(3), 60.0, 39.0, 0.0, 0.0),
        day(on(4), 70.0, 50.0, 0.0, 0.5),
    ];
    assert_eq!(window(spoiled, 0.0), None, "no good day, no recommendation");
}

// @spec RULE-WX-006
#[test]
fn the_first_days_yesterday_is_the_lakes_week_of_rain() {
    let days = || {
        vec![
            day(on(0), 72.0, 52.0, 0.0, 0.0),
            day(on(1), 92.0, 70.0, 0.0, 0.0), // dry but too hot
            day(on(2), 70.0, 50.0, 8.0, 0.9),
        ]
    };
    let (_, _, description) = window(days(), 24.9).expect("a soggy-free week");
    assert!(description.contains(&on(0).format("%b %d").to_string()));
    assert_eq!(window(days(), 25.0), None, "an inch of rain this week");
}

// @spec RULE-WX-007
#[test]
fn a_partial_forecast_day_is_never_judged() {
    let mut tonight = day(on(0), 70.0, 50.0, 0.0, 0.0);
    tonight.hours_covered = 6;
    let mut last_morning = day(on(5), 70.0, 50.0, 0.0, 0.0);
    last_morning.hours_covered = 9;
    // The only fair-weather days are partial: nothing to announce.
    let partial_only = vec![
        tonight.clone(),
        day(on(1), 70.0, 50.0, 6.0, 0.8),
        day(on(2), 70.0, 50.0, 6.0, 0.8),
        day(on(3), 90.0, 70.0, 0.0, 0.0),
        last_morning,
    ];
    assert_eq!(window(partial_only, 0.0), None);
    // A full day after tonight's partial one is still judged (and announced).
    let days = vec![
        tonight,
        day(on(1), 70.0, 50.0, 0.0, 0.0),
        day(on(2), 90.0, 70.0, 0.0, 0.0),
    ];
    let (_, _, description) = window(days, 0.0).expect("tomorrow is good");
    assert!(description.contains(&on(1).format("%b %d").to_string()));
    assert!(description.contains("1 good day(s)"), "{description}");
}
