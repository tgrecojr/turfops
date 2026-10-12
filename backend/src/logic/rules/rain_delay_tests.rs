//! The rain delay rule's three windows, judged by their highest chance and total amount.

use super::rain_delay::RainDelayRule;
use super::test_fixtures::*;
use super::Rule;
use crate::models::{EnvironmentalSummary, ForecastPoint, Severity};

fn rain_delay(points: Vec<ForecastPoint>) -> Option<Severity> {
    let env = EnvironmentalSummary {
        forecast: Some(forecast(Vec::new(), points)),
        ..conditions(None, None)
    };
    RainDelayRule
        .evaluate(&env, &fescue(), &[], crate::clock::Clock::System.moment())
        .map(|r| {
            assert_eq!(r.id, "rain_delay");
            r.severity
        })
}

// @spec RULE-WX-001
#[test]
fn the_nearest_window_that_reaches_its_bar_sets_the_severity() {
    use Severity::*;
    let cases = [
        (vec![point(6, 0.0, 0.7)], Some(Critical), "70% within 12 h"),
        (
            vec![point(6, 2.6, 0.1)],
            Some(Critical),
            "2.5 mm within 12 h",
        ),
        (vec![point(20, 0.0, 0.55)], Some(Warning), "50% within 24 h"),
        (
            vec![point(20, 3.0, 0.1)],
            Some(Warning),
            "2.5 mm within 24 h",
        ),
        (
            vec![point(30, 0.2, 0.35)],
            Some(Advisory),
            "30% within 48 h",
        ),
        (
            vec![point(40, 6.0, 0.1)],
            Some(Advisory),
            "5 mm within 48 h",
        ),
        (
            vec![point(40, 0.0, 0.55)],
            Some(Advisory),
            "50% only past 24 h",
        ),
        (vec![point(60, 9.0, 0.9)], None, "past 48 h"),
    ];
    for (points, expected, case) in cases {
        assert_eq!(rain_delay(points), expected, "{case}");
    }
}

// @spec RULE-WX-001
#[test]
fn amounts_are_totalled_over_the_window() {
    let showers = vec![point(3, 1.0, 0.1), point(6, 1.0, 0.1), point(9, 1.0, 0.1)];
    assert_eq!(
        rain_delay(showers),
        Some(Severity::Critical),
        "3 mm in 12 h"
    );
    assert_eq!(
        rain_delay(vec![point(6, 2.0, 0.1)]),
        None,
        "2 mm at a low chance reaches no bar"
    );
}

// @spec RULE-WX-001
#[test]
fn a_modest_chance_with_no_forecast_amount_is_skipped() {
    assert_eq!(rain_delay(vec![point(30, 0.0, 0.35)]), None, "35%, 0 mm");
    assert_eq!(rain_delay(vec![point(6, 0.0, 0.45)]), None, "45%, 0 mm");
    assert_eq!(
        rain_delay(vec![point(30, 0.05, 0.45)]),
        None,
        "under 0.1 mm"
    );
}
