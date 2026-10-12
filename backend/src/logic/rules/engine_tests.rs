//! The engine: the rule order, at most one recommendation per rule, no fallback for a
//! missing input, and the local calendar date.

use super::fall_fertilization::FallFertilizationRule;
use super::grub_control::GrubControlRule;
use super::spring_nitrogen::SpringNitrogenRule;
use super::test_fixtures::*;
use super::{Rule, RulesEngine};
use crate::models::{EnvironmentalSummary, Severity};
use chrono::Datelike;

/// The engine's rule order, as the id prefix each rule emits.
const ORDER: [&str; 10] = [
    "spring_n_",
    "broadleaf_",
    "grub_control_",
    "fertilizer_block",
    "fall_fert_",
    "rain_delay",
    "irrigation_forecast",
    "heat_stress_forecast",
    "application_window",
    "mowing_height",
];

fn rule_index(id: &str) -> usize {
    ORDER
        .iter()
        .position(|prefix| id.starts_with(prefix))
        .unwrap_or_else(|| panic!("{id} is not a rule id"))
}

/// Conditions that set off as many rules as one day can: hot dry air, dry soil, rain due
/// within 12 hours on an otherwise dry forecast, a warm week, mid-season soil and GDD.
fn busy_day() -> EnvironmentalSummary {
    let mut env = conditions(Some(88.0), Some(0.08));
    if let Some(current) = env.current.as_mut() {
        current.soil_temp_10_f = Some(62.0);
    }
    EnvironmentalSummary {
        soil_temp_7day_avg_f: Some(62.0),
        ambient_temp_7day_avg_f: Some(70.0),
        precipitation_7day_total_mm: Some(0.0),
        gdd_base50_ytd: Some(600.0),
        forecast: Some(forecast(
            dry_days_from_today(&[85.0, 80.0, 80.0, 80.0, 80.0]),
            vec![point(6, 0.0, 0.8)],
        )),
        ..env
    }
}

// @spec RULE-ENG-002
#[test]
fn the_engine_returns_rules_in_their_fixed_order() {
    let ids: Vec<String> = RulesEngine::new()
        .evaluate(
            &busy_day(),
            &fescue(),
            &[],
            crate::clock::Clock::System.moment(),
        )
        .into_iter()
        .map(|r| r.id)
        .collect();
    let positions: Vec<usize> = ids.iter().map(|id| rule_index(id)).collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{ids:?}");
    // The year-round rules all fired, so the order was really exercised.
    for id in [
        "fertilizer_block",
        "rain_delay",
        "irrigation_forecast",
        "heat_stress_forecast",
        "application_window",
        "mowing_height",
    ] {
        assert!(ids.iter().any(|i| i == id), "{id} missing from {ids:?}");
    }
}

// @spec RULE-ENG-001
#[test]
fn each_rule_is_a_pure_function_giving_at_most_one_recommendation() {
    let engine = RulesEngine::new();
    let first = engine.evaluate(
        &busy_day(),
        &fescue(),
        &[],
        crate::clock::Clock::System.moment(),
    );
    let again = engine.evaluate(
        &busy_day(),
        &fescue(),
        &[],
        crate::clock::Clock::System.moment(),
    );
    let key = |recs: &[crate::models::Recommendation]| -> Vec<(String, Severity, String)> {
        recs.iter()
            .map(|r| (r.id.clone(), r.severity, r.description.clone()))
            .collect()
    };
    assert_eq!(key(&first), key(&again), "same inputs, same answer");
    assert!(first.len() <= ORDER.len());
    let mut rules: Vec<usize> = first.iter().map(|r| rule_index(&r.id)).collect();
    rules.dedup();
    assert_eq!(rules.len(), first.len(), "one recommendation per rule");
}

// @spec RULE-ENG-003
#[test]
fn a_missing_input_means_no_recommendation_not_a_calendar_guess() {
    let engine = RulesEngine::new();
    assert!(engine
        .evaluate(
            &EnvironmentalSummary::default(),
            &fescue(),
            &[],
            crate::clock::Clock::System.moment()
        )
        .is_empty());
    // GDD and a forecast, but no soil: none of the soil-driven rules speaks.
    let no_soil = EnvironmentalSummary {
        gdd_base50_ytd: Some(600.0),
        forecast: Some(forecast(dry_days_from_today(&[70.0; 5]), Vec::new())),
        ..Default::default()
    };
    for rec in engine.evaluate(
        &no_soil,
        &fescue(),
        &[],
        crate::clock::Clock::System.moment(),
    ) {
        assert!(
            ![
                "spring_n_",
                "broadleaf_",
                "grub_control_",
                "fall_fert_",
                "fertilizer_block"
            ]
            .iter()
            .any(|p| rec.id.starts_with(p)),
            "{} spoke without its input",
            rec.id
        );
    }
}

// @spec RULE-ENG-005
#[test]
fn the_rules_read_the_local_calendar_date() {
    let today = today();
    // The grub rule's `evaluate` is `evaluate_on` the local today, and its id carries the
    // local year.
    let env = busy_day();
    let rec = GrubControlRule.evaluate(&env, &fescue(), &[], crate::clock::Clock::System.moment());
    if today.month() >= 4 {
        assert_eq!(
            rec.map(|r| r.id),
            Some(format!("grub_control_{}", today.year()))
        );
    } else {
        assert!(rec.is_none(), "the GDD window opens on Apr 1");
    }
    // The month gates follow the local month.
    let fall = FallFertilizationRule
        .evaluate(
            &soil(Some(55.0), None),
            &fescue(),
            &[],
            crate::clock::Clock::System.moment(),
        )
        .map(|r| r.id);
    let expected_fall = match today.month() {
        9 => Some("fall_fert_early"),
        10 => Some("fall_fert_mid"),
        11 => Some("fall_fert_winterizer"),
        _ => None,
    };
    assert_eq!(fall.as_deref(), expected_fall);
    let spring = SpringNitrogenRule
        .evaluate(
            &soil(Some(58.0), None),
            &fescue(),
            &[],
            crate::clock::Clock::System.moment(),
        )
        .map(|r| r.id);
    let expected_spring = match today.month() {
        2..=4 => Some("spring_n_ready"),
        5 => Some("spring_n_may_cutoff"),
        _ => None,
    };
    assert_eq!(spring.as_deref(), expected_spring);
}
