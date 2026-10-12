//! The fall feeding program. What counts as a feeding is checked on fixed dates; the rule
//! itself reads `chrono::Local`, so each month's decision table runs only in that month
//! and every other day the same inputs must produce nothing.

use super::*;
use crate::logic::rules::test_fixtures::{all_text, app, date, feeding_on, fescue, soil, today};
use crate::models::{ApplicationType, Severity};
use chrono::Duration;

fn month() -> u32 {
    today().month()
}

fn decide(avg: Option<f64>, history: &[Application]) -> Option<(String, Severity)> {
    FallFertilizationRule
        .evaluate(&soil(avg, None), &fescue(), history)
        .map(|r| (r.id, r.severity))
}

fn expect(id: &str, severity: Severity) -> Option<(String, Severity)> {
    Some((id.to_string(), severity))
}

/// A lawn feeding on Sep `day` of this year.
fn sep(day: u32) -> Application {
    feeding_on(date(today().year(), 9, day))
}

// @spec RULE-FALL-001
#[test]
fn fall_feeding_runs_sep_1_to_nov_30_and_needs_the_soil_average() {
    assert_eq!(decide(None, &[]), None);
    let fires = decide(Some(55.0), &[]).is_some();
    assert_eq!(fires, (9..=11).contains(&month()), "month {}", month());
}

// @spec RULE-FALL-002
#[test]
fn only_this_years_lawn_feedings_so_far_count() {
    let on = date(2026, 10, 15);
    let fert = ApplicationType::Fertilizer;
    let ignored = [
        app(fert, date(2026, 9, 10), Some(0.0), Some(4.0)), // 0-0-50 potash
        app(
            ApplicationType::PlantFertilizer,
            date(2026, 9, 10),
            Some(10.0),
            Some(1.0),
        ),
        app(fert, date(2026, 10, 20), Some(24.0), Some(4.0)), // future-dated
        app(fert, date(2025, 10, 1), Some(24.0), Some(4.0)),  // last year
    ];
    assert!(feedings_this_year(&ignored, on).is_empty());
    assert_eq!(fall_feeding_count(&ignored, on), 0);
    let counted = [
        app(fert, date(2026, 9, 10), None, None), // analysis not recorded
        app(fert, date(2026, 8, 20), Some(24.0), Some(4.0)),
    ];
    assert_eq!(feedings_this_year(&counted, on).len(), 2);
    assert_eq!(fall_feeding_count(&counted, on), 1, "August is not fall");
}

// @spec RULE-FALL-003
#[test]
fn a_feeding_in_the_last_21_days_holds_the_next_one() {
    let ago = |days| [feeding_on(today() - Duration::days(days))];
    let in_window = (9..=11).contains(&month());
    assert_eq!(decide(Some(55.0), &ago(20)), None);
    assert_eq!(decide(Some(55.0), &ago(0)), None);
    assert_eq!(decide(Some(55.0), &ago(21)).is_some(), in_window);
    // A potash-only entry is not a feeding and does not hold anything.
    let potash = [app(
        ApplicationType::Fertilizer,
        today(),
        Some(0.0),
        Some(4.0),
    )];
    assert_eq!(decide(Some(55.0), &potash).is_some(), in_window);
}

// @spec RULE-FALL-004
#[test]
fn september_asks_for_the_first_fall_feeding() {
    let rec = build_early_fall_rec(55.0, &fescue(), &soil(Some(55.0), None));
    assert_eq!(
        (rec.id.as_str(), rec.severity),
        ("fall_fert_early", Severity::Warning)
    );
    if month() == 9 {
        assert_eq!(
            decide(Some(45.0), &[]),
            expect("fall_fert_early", Severity::Warning)
        );
        assert_eq!(
            decide(Some(65.0), &[]),
            expect("fall_fert_early", Severity::Warning)
        );
        assert_eq!(decide(Some(44.9), &[]), None);
        assert_eq!(decide(Some(65.1), &[]), None);
        let fed_sep_1 = [feeding_on(date(today().year(), 9, 1))];
        assert_eq!(decide(Some(55.0), &fed_sep_1), None);
    }
}

// @spec RULE-FALL-005
#[test]
fn october_asks_for_the_main_feeding_until_two_are_down() {
    for (count, severity) in [(0, Severity::Warning), (1, Severity::Advisory)] {
        let rec = build_mid_fall_rec(55.0, count, &fescue(), &soil(Some(55.0), None));
        assert_eq!((rec.id.as_str(), rec.severity), ("fall_fert_mid", severity));
    }
    if month() == 10 {
        assert_eq!(
            decide(Some(45.0), &[]),
            expect("fall_fert_mid", Severity::Warning)
        );
        assert_eq!(
            decide(Some(65.0), &[sep(1)]),
            expect("fall_fert_mid", Severity::Advisory)
        );
        assert_eq!(decide(Some(55.0), &[sep(1), sep(5)]), None);
        assert_eq!(decide(Some(44.9), &[]), None);
        assert_eq!(decide(Some(65.1), &[]), None);
    }
}

// @spec RULE-FALL-006
#[test]
fn november_asks_for_the_winterizer_until_three_are_down() {
    for (count, severity) in [(0, Severity::Warning), (2, Severity::Advisory)] {
        let rec = build_late_fall_rec(45.0, count, &fescue());
        assert_eq!(
            (rec.id.as_str(), rec.severity),
            ("fall_fert_winterizer", severity)
        );
    }
    if month() == 11 {
        let id = "fall_fert_winterizer";
        assert_eq!(decide(Some(40.0), &[]), expect(id, Severity::Warning));
        assert_eq!(
            decide(Some(70.0), &[sep(1)]),
            expect(id, Severity::Advisory)
        );
        assert_eq!(decide(Some(50.0), &[sep(1), sep(5), sep(9)]), None);
        assert_eq!(decide(Some(39.9), &[]), None);
    }
}

// @spec RULE-ENG-008
#[test]
fn fall_feedings_state_the_rate_and_the_lawns_total() {
    let env = soil(Some(55.0), None);
    let mut unsized_lawn = fescue();
    unsized_lawn.lawn_size_sqft = None;
    let sized = [
        build_early_fall_rec(55.0, &fescue(), &env),
        build_mid_fall_rec(55.0, 0, &fescue(), &env),
        build_late_fall_rec(45.0, 0, &fescue()),
    ];
    let totals = [
        "about 5 lb of nitrogen for your 5000 sq ft lawn (1 lb N/1000 sq ft)",
        "about 3.75 lb of nitrogen for your 5000 sq ft lawn (0.75 lb N/1000 sq ft)",
        "about 5 lb of nitrogen for your 5000 sq ft lawn (1 lb N/1000 sq ft)",
    ];
    for (rec, total) in sized.iter().zip(totals) {
        let action = rec.suggested_action.as_deref().unwrap_or_default();
        assert!(action.contains(total), "{action}");
        assert!(!all_text(rec).contains("label rate"));
    }
    let rec = build_mid_fall_rec(55.0, 0, &unsized_lawn, &env);
    let action = rec.suggested_action.as_deref().unwrap_or_default();
    assert!(
        action.starts_with("Apply 0.75 lb N/1000 sq ft."),
        "{action}"
    );
}
