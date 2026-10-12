//! The N-P-K evaluator: levels, amounts, the nitrogen budget, ratio, example analysis and
//! the explanation.

use super::gap_tests::{d, fed};
use super::tests::{make_profile, make_test, today};
use super::*;
use crate::models::GrassType;

fn npk(p: Option<f64>, k: Option<f64>, apps: &[Application]) -> NpkRecommendation {
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    calculate_npk_recommendation(&make_test(6.5, p, k), &profile, apps, today()).unwrap()
}

/// This year's lawn feedings totalling `lbs` N per 1,000 sq ft.
fn applied(lbs: f64) -> Vec<Application> {
    vec![fed(d(2026, 5, 1), lbs)]
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// @spec SOIL-NPK-001
#[test]
fn p_and_k_are_classified_by_the_mehlich_3_bands() {
    for (p, level) in [
        (14.9, NutrientLevel::Low),
        (15.0, NutrientLevel::Adequate),
        (50.0, NutrientLevel::Adequate),
        (50.1, NutrientLevel::High),
    ] {
        assert_eq!(
            npk(Some(p), Some(140.0), &[]).phosphorus_level,
            level,
            "P {p}"
        );
    }
    for (k, level) in [
        (99.9, NutrientLevel::Low),
        (100.0, NutrientLevel::Adequate),
        (175.0, NutrientLevel::Adequate),
        (175.1, NutrientLevel::High),
    ] {
        assert_eq!(
            npk(Some(30.0), Some(k), &[]).potassium_level,
            level,
            "K {k}"
        );
    }
}

// @spec SOIL-NPK-002
#[test]
fn p2o5_only_for_low_p_and_k2o_by_the_k_level() {
    let p = |ppm| npk(Some(ppm), Some(200.0), &[]).phosphorus_rate_lbs_per_1000sqft;
    assert_eq!((p(10.0), p(30.0), p(60.0)), (0.25, 0.0, 0.0));
    let k = |ppm| npk(Some(30.0), Some(ppm), &[]).potassium_rate_lbs_per_1000sqft;
    assert_eq!((k(80.0), k(140.0), k(200.0)), (0.5, 0.25, 0.0));
}

// @spec SOIL-NPK-003
#[test]
fn remaining_nitrogen_is_the_annual_target_less_this_years_lawn_nitrogen() {
    let remaining = |apps: &[Application]| {
        npk(Some(30.0), Some(140.0), apps).remaining_n_budget_lbs_per_1000sqft
    };
    assert!(
        close(remaining(&[]), 3.5),
        "tall fescue's recommended target"
    );
    assert!(close(remaining(&applied(1.25)), 2.25));

    let mut shrub = fed(d(2026, 5, 1), 1.0);
    shrub.plant_name = Some("Hydrangea".into());
    assert!(
        close(remaining(&[shrub]), 3.5),
        "plant feedings count nothing"
    );

    assert_eq!(remaining(&applied(5.0)), 0.0, "never below zero");
}

// @spec SOIL-NPK-005
#[test]
fn nitrogen_is_none_below_a_quarter_pound_left_else_the_rest_capped_at_three_quarters() {
    let n = |spent| npk(Some(30.0), Some(140.0), &applied(spent)).nitrogen_rate_lbs_per_1000sqft;
    assert_eq!(n(3.3), 0.0, "0.2 lb left");
    assert!((n(3.2) - 0.3).abs() < 1e-6, "0.3 lb left: all of it");
    assert!(close(n(2.9), 0.6));
    assert_eq!(n(0.0), 0.75, "3.5 lb left, capped");
}

// @spec SOIL-NPK-006
#[test]
fn nothing_advised_still_returns_the_recommendation_marked_n_a() {
    let rec = npk(Some(60.0), Some(200.0), &applied(4.0));
    assert_eq!(rec.recommended_ratio, "N/A");
    assert_eq!(rec.example_product_ratio, "N/A");
    for amount in [
        rec.nitrogen_rate_lbs_per_1000sqft,
        rec.phosphorus_rate_lbs_per_1000sqft,
        rec.potassium_rate_lbs_per_1000sqft,
        rec.product_rate_lbs_per_1000sqft,
    ] {
        assert_eq!(amount, 0.0);
    }
    assert_eq!(rec.remaining_n_budget_lbs_per_1000sqft, 0.0);
    assert!(
        rec.explanation.contains("No fertilizer is needed now."),
        "{}",
        rec.explanation
    );
}

// @spec SOIL-NPK-007
#[test]
fn the_ratio_divides_each_amount_by_the_smallest_and_rounds() {
    assert_eq!(suggest_ratio(0.75, 0.25, 0.5).0, "3-1-2");
    assert_eq!(suggest_ratio(0.75, 0.0, 0.25).0, "3-0-1");
    assert_eq!(suggest_ratio(0.6, 0.0, 0.5).0, "1-0-1");
    assert_eq!(suggest_ratio(0.0, 0.25, 0.5).0, "0-1-2");
    assert_eq!(npk(Some(10.0), Some(80.0), &[]).recommended_ratio, "3-1-2");
}

// @spec SOIL-NPK-008
#[test]
fn the_example_analysis_follows_which_nutrients_are_advised() {
    let example = |n, p, k| suggest_ratio(n, p, k).1;
    assert_eq!(example(0.75, 0.25, 0.25), "24-4-12", "N at least twice K");
    assert_eq!(example(0.75, 0.25, 0.5), "12-4-8");
    assert_eq!(example(0.75, 0.25, 0.0), "15-5-0");
    assert_eq!(example(0.75, 0.0, 0.25), "24-0-11", "N at least twice K");
    assert_eq!(example(0.5, 0.0, 0.5), "16-0-8");
    assert_eq!(example(0.75, 0.0, 0.0), "46-0-0");
    assert_eq!(example(0.0, 0.25, 0.5), "0-5-10");
    assert_eq!(example(0.0, 0.25, 0.0), "0-20-0");
    assert_eq!(example(0.0, 0.0, 0.5), "0-0-50");
}

// @spec SOIL-NPK-009
#[test]
fn nitrogen_advice_states_pounds_of_the_example_that_supply_it() {
    let rec = npk(Some(60.0), Some(200.0), &[]);
    assert_eq!(rec.example_product_ratio, "46-0-0");
    assert!(
        close(rec.product_rate_lbs_per_1000sqft, 1.6),
        "0.75 / 0.46 = 1.63"
    );
    let rec = npk(Some(10.0), Some(80.0), &[]);
    assert_eq!(rec.example_product_ratio, "12-4-8");
    assert!(
        close(rec.product_rate_lbs_per_1000sqft, 6.3),
        "0.75 / 0.12 = 6.25"
    );
}

// @spec SOIL-NPK-011
#[test]
fn the_explanation_states_levels_budget_nitrogen_ratio_and_example() {
    let text = npk(Some(10.0), Some(80.0), &[]).explanation;
    for part in [
        "low P (10 ppm)",
        "low K (80 ppm)",
        "3.50 lbs N/1000 sqft remaining",
        "0.75 lbs N",
        "3-1-2 ratio",
        "12-4-8 product, apply 6.3 lbs/1000 sqft",
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
    assert!(!text.contains("capped"), "{text}");

    let text = npk(Some(10.0), Some(80.0), &applied(3.2)).explanation;
    assert!(
        text.contains("only 0.30 lbs N/1000 sqft remaining") && text.contains("capped"),
        "{text}"
    );
}
