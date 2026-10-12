//! The rules judge dates and hour windows on the moment the caller passes, so any season
//! can be tested on any day.

use super::broadleaf_herbicide::BroadleafHerbicideRule;
use super::fall_fertilization::FallFertilizationRule;
use super::heat_stress::HeatStressRule;
use super::rain_delay::RainDelayRule;
use super::spring_nitrogen::SpringNitrogenRule;
use super::test_fixtures::*;
use super::Rule;
use crate::clock::Moment;
use crate::models::{environmental::Trend, EnvironmentalSummary, Severity};
use chrono::{Duration, NaiveDate, TimeZone, Utc};

/// Midday (UTC) on `day`, with `day` as the lawn's date.
fn on(day: NaiveDate) -> Moment {
    Moment {
        now: Utc.from_utc_datetime(&day.and_hms_opt(16, 0, 0).unwrap()),
        today: day,
    }
}

fn fires(rule: &dyn Rule, env: &EnvironmentalSummary, at: Moment) -> Option<String> {
    rule.evaluate(env, &fescue(), &[], at).map(|r| r.id)
}

fn warming_soil(avg: f64) -> EnvironmentalSummary {
    EnvironmentalSummary {
        soil_temp_trend: Trend::Rising,
        gdd_base50_ytd: Some(80.0),
        ..soil(Some(avg), Some(avg))
    }
}

// @spec RULE-ENG-005
#[test]
fn seasonal_rules_open_and_close_on_the_moments_date() {
    let spring = warming_soil(52.0);
    assert!(fires(&SpringNitrogenRule, &spring, on(date(2026, 3, 20))).is_some());
    assert_eq!(
        fires(&SpringNitrogenRule, &spring, on(date(2026, 7, 1))),
        None
    );

    let fall = soil(Some(58.0), Some(58.0));
    assert!(fires(&FallFertilizationRule, &fall, on(date(2026, 10, 5))).is_some());
    assert_eq!(
        fires(&FallFertilizationRule, &fall, on(date(2026, 7, 1))),
        None
    );

    assert_eq!(
        fires(&BroadleafHerbicideRule, &fall, on(date(2026, 10, 5))).as_deref(),
        Some("broadleaf_fall")
    );
    assert_eq!(
        fires(&BroadleafHerbicideRule, &fall, on(date(2026, 8, 1))),
        None
    );
}

// @spec RULE-ENG-005
#[test]
fn forecast_days_and_hours_are_counted_from_the_moment() {
    let first = date(2026, 7, 10);
    let heat = EnvironmentalSummary {
        forecast: Some(forecast(
            (0..5)
                .map(|i| {
                    day(
                        first + Duration::days(i),
                        if i == 4 { 96.0 } else { 80.0 },
                        65.0,
                        0.0,
                        0.0,
                    )
                })
                .collect(),
            Vec::new(),
        )),
        ..conditions(None, None)
    };
    // Jul 14 is three days after Jul 11 (counted) but four after Jul 10 (not yet).
    assert_eq!(fires(&HeatStressRule, &heat, on(first)), None);
    assert!(fires(&HeatStressRule, &heat, on(first + Duration::days(1))).is_some());

    let at = on(first);
    let mut shower = point(0, 0.0, 0.8);
    shower.timestamp = at.now + Duration::hours(6);
    let rain = EnvironmentalSummary {
        forecast: Some(forecast(Vec::new(), vec![shower])),
        ..conditions(None, None)
    };
    let severity = |at| {
        RainDelayRule
            .evaluate(&rain, &fescue(), &[], at)
            .map(|r| r.severity)
    };
    assert_eq!(
        severity(at),
        Some(Severity::Critical),
        "6 h after the moment"
    );
    assert_eq!(
        severity(on(first - Duration::days(2))),
        None,
        "54 h ahead is past 48 h"
    );
}
