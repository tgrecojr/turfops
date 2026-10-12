//! When the broadleaf herbicide rule speaks. The spring window opens on GDD alone on any
//! date, so it is checked on every run; the fall window is a calendar range and its
//! decision table only runs between Sep 20 and Oct 31 (the rule reads `chrono::Local`).

use super::*;
use crate::logic::rules::test_fixtures::{app, date, fescue, soil, today};
use crate::models::environmental::Trend;
use chrono::Duration;

fn env(avg: f64, trend: Trend, gdd: Option<f64>) -> EnvironmentalSummary {
    let mut env = soil(Some(avg), None);
    env.soil_temp_trend = trend;
    env.gdd_base50_ytd = gdd;
    env
}

fn id(env: &EnvironmentalSummary, history: &[Application]) -> Option<String> {
    BroadleafHerbicideRule
        .evaluate(env, &fescue(), history)
        .map(|r| r.id)
}

fn in_fall_window() -> bool {
    let t = today();
    t >= date(t.year(), 9, 20) && t <= date(t.year(), 10, 31)
}

fn spring() -> Option<String> {
    Some("broadleaf_spring".into())
}

// @spec RULE-BROAD-001
#[test]
fn a_recent_overseed_silences_the_herbicide_rule() {
    let springlike = env(50.0, Trend::Rising, Some(100.0));
    let seeded = |days_ago| {
        [app(
            ApplicationType::Overseed,
            today() - Duration::days(days_ago),
            None,
            None,
        )]
    };
    assert_eq!(id(&springlike, &seeded(60)), None);
    assert_eq!(id(&springlike, &seeded(0)), None);
    assert_eq!(id(&springlike, &seeded(61)), spring());
}

// @spec RULE-BROAD-002, RULE-ENG-004
#[test]
fn spring_needs_gdd_or_march_cool_rising_soil_and_no_spring_spray() {
    let year = today().year();
    let march = today().month() == 3;
    // GDD 50–150 opens the window on any date.
    for gdd in [50.0, 100.0, 150.0] {
        assert_eq!(id(&env(47.0, Trend::Rising, Some(gdd)), &[]), spring());
    }
    // Outside it, only March does (with or without a GDD record).
    for gdd in [Some(49.0), Some(151.0), None] {
        let fired = id(&env(47.0, Trend::Rising, gdd), &[]) == spring();
        assert_eq!(fired, march, "gdd {gdd:?}");
    }
    // Soil 45–55 inclusive, rising.
    assert_eq!(id(&env(45.0, Trend::Rising, Some(100.0)), &[]), spring());
    assert_eq!(id(&env(55.0, Trend::Rising, Some(100.0)), &[]), spring());
    assert_ne!(id(&env(44.9, Trend::Rising, Some(100.0)), &[]), spring());
    assert_ne!(id(&env(55.1, Trend::Rising, Some(100.0)), &[]), spring());
    for trend in [Trend::Stable, Trend::Falling, Trend::Unknown] {
        assert_eq!(id(&env(47.0, trend, Some(100.0)), &[]), None);
    }
    // A post-emergent this year Jan–Apr is the spring spray; May or last year is not.
    let spray = |on| [app(ApplicationType::PostEmergent, on, None, None)];
    let ready = env(47.0, Trend::Rising, Some(100.0));
    assert_eq!(id(&ready, &spray(date(year, 4, 30))), None);
    assert_eq!(id(&ready, &spray(date(year, 1, 2))), None);
    assert_eq!(id(&ready, &spray(date(year, 5, 1))), spring());
    assert_eq!(id(&ready, &spray(date(year - 1, 4, 1))), spring());

    let rec = build_spring_herbicide_rec(47.0, None);
    assert_eq!(rec.severity, Severity::Advisory);
    assert!(rec.data_points.iter().all(|d| !d.label.contains("GDD")));
}

// @spec RULE-BROAD-003
#[test]
fn fall_runs_sep_20_to_oct_31_on_soil_from_50_to_65() {
    let rec = build_fall_herbicide_rec(58.0);
    assert_eq!(
        (rec.id.as_str(), rec.severity),
        ("broadleaf_fall", Severity::Warning)
    );

    let fall = |avg| id(&env(avg, Trend::Falling, None), &[]);
    let expected = in_fall_window().then(|| "broadleaf_fall".to_string());
    assert_eq!(fall(50.0), expected);
    assert_eq!(fall(65.0), expected);
    assert_eq!(fall(49.9), None);
    assert_eq!(fall(65.1), None);
    // A post-emergent from September on is the fall spray; one in August is not.
    let year = today().year();
    let spray = |m| {
        [app(
            ApplicationType::PostEmergent,
            date(year, m, 1),
            None,
            None,
        )]
    };
    let ready = env(58.0, Trend::Falling, None);
    assert_eq!(id(&ready, &spray(9)), None);
    assert_eq!(id(&ready, &spray(8)), expected);
}

// @spec RULE-BROAD-004
#[test]
fn both_windows_call_for_a_post_emergent_broadleaf_herbicide() {
    for rec in [
        build_spring_herbicide_rec(50.0, Some(90.0)),
        build_fall_herbicide_rec(58.0),
    ] {
        assert_eq!(rec.needs.len(), 1, "{}", rec.id);
        let need = &rec.needs[0];
        assert_eq!(need.categories, vec![ProductCategory::Herbicide]);
        assert_eq!(need.herbicide_timing, Some(HerbicideTiming::PostEmergent));
        assert_eq!(need.targets, vec![ProductTarget::Broadleaf]);
        assert!(!need.optional);
    }
}
