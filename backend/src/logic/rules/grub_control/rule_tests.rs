//! The grub control window, its log check, inputs and severities, on fixed dates.

use super::*;
use crate::logic::rules::test_fixtures::{app, date, soil};

fn env(avg: Option<f64>, live: Option<f64>, gdd: Option<f64>) -> EnvironmentalSummary {
    let mut env = soil(avg, live);
    env.gdd_base50_ytd = gdd;
    env
}

fn grub(
    on: NaiveDate,
    env: &EnvironmentalSummary,
    history: &[Application],
) -> Option<(String, Severity)> {
    GrubControlRule
        .evaluate_on(on, env, history)
        .map(|r| (r.id, r.severity))
}

fn d(m: u32, day: u32) -> NaiveDate {
    date(2026, m, day)
}

// @spec RULE-GRUB-001
#[test]
fn the_window_is_may_15_to_jul_4_or_gdd_500_to_1000_from_apr_1() {
    // The calendar window, with or without GDD.
    assert!(window_open(d(5, 15), None));
    assert!(window_open(d(7, 4), None));
    assert!(!window_open(d(5, 14), None));
    assert!(!window_open(d(7, 5), None));
    // A low GDD does not close the calendar window.
    assert!(window_open(d(6, 1), Some(100.0)));
    // The GDD extension: from Apr 1, while GDD is in [500, 1000).
    assert!(window_open(d(4, 1), Some(500.0)));
    assert!(window_open(d(8, 15), Some(999.0)));
    assert!(!window_open(d(3, 31), Some(600.0)));
    assert!(!window_open(d(4, 20), Some(499.0)));
    assert!(!window_open(d(8, 15), Some(1000.0)));
}

// @spec RULE-GRUB-002
#[test]
fn grub_or_insect_control_since_apr_1_silences_the_rule() {
    let ready = env(Some(65.0), Some(66.0), None);
    let logged = |kind, on| [app(kind, on, None, None)];
    assert_eq!(
        grub(
            d(6, 1),
            &ready,
            &logged(ApplicationType::GrubControl, d(4, 1))
        ),
        None
    );
    assert_eq!(
        grub(
            d(6, 1),
            &ready,
            &logged(ApplicationType::Insecticide, d(5, 20))
        ),
        None
    );
    // Before Apr 1, or last season, does not count.
    assert!(grub(
        d(6, 1),
        &ready,
        &logged(ApplicationType::GrubControl, d(3, 31))
    )
    .is_some());
    let last_year = date(2025, 6, 1);
    assert!(grub(
        d(6, 1),
        &ready,
        &logged(ApplicationType::GrubControl, last_year)
    )
    .is_some());
    // Other products are not grub control.
    assert!(grub(
        d(6, 1),
        &ready,
        &logged(ApplicationType::Fungicide, d(5, 1))
    )
    .is_some());
}

// @spec RULE-GRUB-003
#[test]
fn grub_control_needs_the_soil_average_and_the_live_reading() {
    assert!(grub(d(6, 1), &env(Some(65.0), Some(66.0), None), &[]).is_some());
    assert_eq!(grub(d(6, 1), &env(None, Some(66.0), None), &[]), None);
    assert_eq!(grub(d(6, 1), &env(Some(65.0), None, None), &[]), None);
}

// @spec RULE-GRUB-004, RULE-ENG-004
#[test]
fn optimal_soil_is_advisory_until_peak_hatch_or_the_last_two_weeks() {
    let at = |avg, gdd| env(Some(avg), Some(avg), gdd);
    let id = "grub_control_2026".to_string();
    // No GDD record: the calendar and the soil still decide.
    assert_eq!(
        grub(d(6, 1), &at(60.0, None), &[]),
        Some((id.clone(), Severity::Advisory))
    );
    assert_eq!(
        grub(d(6, 1), &at(75.0, None), &[]),
        Some((id.clone(), Severity::Advisory))
    );
    let rec = GrubControlRule
        .evaluate_on(d(6, 1), &at(65.0, None), &[])
        .unwrap();
    assert!(rec.data_points.iter().all(|p| !p.label.contains("GDD")));
    // GDD only escalates: peak hatch at 700.
    assert_eq!(
        grub(d(6, 1), &at(65.0, Some(699.0)), &[]),
        Some((id.clone(), Severity::Advisory))
    );
    assert_eq!(
        grub(d(6, 1), &at(65.0, Some(700.0)), &[]),
        Some((id.clone(), Severity::Warning))
    );
    // 14 days or fewer to Jul 4.
    assert_eq!(
        grub(d(6, 19), &at(65.0, None), &[]),
        Some((id.clone(), Severity::Advisory))
    );
    assert_eq!(
        grub(d(6, 20), &at(65.0, None), &[]),
        Some((id, Severity::Warning))
    );
    // Cooler soil: nothing yet.
    assert_eq!(grub(d(6, 1), &at(59.9, None), &[]), None);
}

// @spec RULE-GRUB-005
#[test]
fn soil_above_75_is_a_late_info_note() {
    let warm = env(Some(75.1), Some(78.0), Some(800.0));
    assert_eq!(
        grub(d(6, 25), &warm, &[]),
        Some(("grub_control_late_2026".to_string(), Severity::Info))
    );
}

// @spec RULE-GRUB-006
#[test]
fn both_grub_recommendations_call_for_grub_insect_control() {
    for avg in [65.0, 80.0] {
        let rec = GrubControlRule
            .evaluate_on(d(6, 1), &env(Some(avg), Some(avg), None), &[])
            .unwrap();
        assert_eq!(rec.needs.len(), 1, "{}", rec.id);
        assert_eq!(
            rec.needs[0].categories,
            vec![ProductCategory::InsectControl]
        );
        assert_eq!(rec.needs[0].targets, vec![ProductTarget::Grubs]);
        assert!(!rec.needs[0].optional);
    }
}
