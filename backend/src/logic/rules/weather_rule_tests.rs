//! Fertilizer block, heat stress and mowing height: the rules driven by the current
//! conditions and the daily highs.

use super::fertilizer::FertilizerRule;
use super::heat_stress::HeatStressRule;
use super::mowing_height::MowingHeightRule;
use super::test_fixtures::*;
use super::Rule;
use crate::models::{EnvironmentalSummary, GrassType, Severity};

fn block(air: Option<f64>, moisture: Option<f64>) -> Option<Severity> {
    FertilizerRule
        .evaluate(
            &conditions(air, moisture),
            &fescue(),
            &[],
            crate::clock::Clock::System.moment(),
        )
        .map(|r| {
            assert_eq!(r.id, "fertilizer_block");
            r.severity
        })
}

// @spec RULE-BLOCK-001
#[test]
fn hot_air_or_dry_or_saturated_soil_blocks_fertilizer() {
    // Inside every limit: nothing to report (the limits themselves are not crossed).
    assert_eq!(block(Some(85.0), Some(0.10)), None);
    assert_eq!(block(Some(70.0), Some(0.40)), None);
    // Air above 85°F; Critical only above 90°F.
    assert_eq!(block(Some(85.5), Some(0.25)), Some(Severity::Warning));
    assert_eq!(block(Some(90.0), Some(0.25)), Some(Severity::Warning));
    assert_eq!(block(Some(90.5), Some(0.25)), Some(Severity::Critical));
    // Dry soil below 0.10; Critical below 0.05.
    assert_eq!(block(Some(70.0), Some(0.09)), Some(Severity::Warning));
    assert_eq!(block(Some(70.0), Some(0.05)), Some(Severity::Warning));
    assert_eq!(block(Some(70.0), Some(0.04)), Some(Severity::Critical));
    // Saturated soil above 0.40 is a Warning.
    assert_eq!(block(Some(70.0), Some(0.45)), Some(Severity::Warning));
}

// @spec RULE-BLOCK-002
#[test]
fn a_missing_reading_drops_only_its_own_check() {
    // No patio air: the soil checks still apply.
    assert_eq!(block(None, Some(0.04)), Some(Severity::Critical));
    assert_eq!(block(None, Some(0.45)), Some(Severity::Warning));
    assert_eq!(block(None, Some(0.25)), None);
    // No live soil moisture: the air check still applies.
    assert_eq!(block(Some(92.0), None), Some(Severity::Critical));
    assert_eq!(block(Some(87.0), None), Some(Severity::Warning));
    assert_eq!(block(Some(72.0), None), None);
}

fn heat(highs: &[f64]) -> Option<Severity> {
    let env = EnvironmentalSummary {
        forecast: Some(forecast(dry_days_from_today(highs), Vec::new())),
        ..Default::default()
    };
    HeatStressRule
        .evaluate(&env, &fescue(), &[], crate::clock::Clock::System.moment())
        .map(|r| {
            assert_eq!(r.id, "heat_stress_forecast");
            r.severity
        })
}

// @spec RULE-WX-005
#[test]
fn heat_stress_follows_the_hottest_high_over_the_next_four_days() {
    assert_eq!(heat(&[80.0, 84.9, 82.0, 79.0]), None);
    assert_eq!(heat(&[80.0, 85.0, 82.0, 79.0]), Some(Severity::Advisory));
    assert_eq!(heat(&[80.0, 82.0, 90.0, 79.0]), Some(Severity::Warning));
    assert_eq!(heat(&[80.0, 82.0, 84.0, 95.0]), Some(Severity::Critical));
    // A heat wave four days out is beyond today and the next three days.
    assert_eq!(heat(&[80.0, 82.0, 84.0, 83.0, 99.0]), None);
}

fn mowing(avg: f64) -> Option<Severity> {
    let env = EnvironmentalSummary {
        ambient_temp_7day_avg_f: Some(avg),
        ..Default::default()
    };
    MowingHeightRule
        .evaluate(&env, &fescue(), &[], crate::clock::Clock::System.moment())
        .map(|r| {
            assert_eq!(r.id, "mowing_height");
            r.severity
        })
}

// @spec RULE-WX-008
#[test]
fn mowing_height_follows_the_weeks_mean_air_temperature() {
    assert_eq!(mowing(86.0), Some(Severity::Warning));
    assert_eq!(mowing(85.0), Some(Severity::Advisory));
    assert_eq!(mowing(75.5), Some(Severity::Advisory));
    assert_eq!(mowing(75.0), Some(Severity::Info));
    assert_eq!(mowing(50.0), Some(Severity::Info));
    assert_eq!(mowing(49.9), None);
}

// @spec RULE-ENG-006
#[test]
fn a_mixed_lawn_gets_the_cool_season_rules() {
    let hot_dry = conditions(Some(92.0), Some(0.04));
    let warm_week = EnvironmentalSummary {
        ambient_temp_7day_avg_f: Some(80.0),
        ..Default::default()
    };
    let heat_wave = EnvironmentalSummary {
        forecast: Some(forecast(dry_days_from_today(&[96.0, 96.0]), Vec::new())),
        ..Default::default()
    };
    let rules: [(&dyn Rule, &EnvironmentalSummary); 3] = [
        (&FertilizerRule, &hot_dry),
        (&MowingHeightRule, &warm_week),
        (&HeatStressRule, &heat_wave),
    ];
    for (rule, env) in rules {
        let mixed = rule.evaluate(
            env,
            &profile(GrassType::Mixed),
            &[],
            crate::clock::Clock::System.moment(),
        );
        let fescue = rule.evaluate(env, &fescue(), &[], crate::clock::Clock::System.moment());
        assert!(mixed.is_some(), "a Mixed lawn is a cool-season lawn");
        assert_eq!(
            mixed.map(|r| (r.id, r.severity)),
            fescue.map(|r| (r.id, r.severity))
        );
        // The cool-season gate is real: a warm-season lawn gets none of these.
        assert!(rule
            .evaluate(
                env,
                &profile(GrassType::Bermuda),
                &[],
                crate::clock::Clock::System.moment()
            )
            .is_none());
    }
}
