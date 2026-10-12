//! The nitrogen basis, untested P / K, P- or K-only advice, and the iron and copper
//! suggestions.

use super::tests::{make_profile, make_test, today};
use super::*;
use crate::models::{ApplicationType, GrassType};
use chrono::{NaiveDate, Utc};

pub(super) fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// A lawn feeding of `lbs` N per 1,000 sq ft on `date` (a 20% N analysis).
pub(super) fn fed(date: NaiveDate, lbs: f64) -> Application {
    Application {
        id: None,
        lawn_profile_id: 1,
        application_type: ApplicationType::Fertilizer,
        product_name: None,
        application_date: date,
        rate_per_1000sqft: Some(lbs * 100.0 / 20.0),
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: Some(20.0),
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

fn remaining(apps: &[Application]) -> f64 {
    let test = make_test(6.5, Some(30.0), Some(140.0));
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    calculate_npk_recommendation(&test, &profile, apps, today())
        .unwrap()
        .remaining_n_budget_lbs_per_1000sqft
}

// @spec SOIL-NPK-004
#[test]
fn only_this_years_feedings_up_to_today_count_against_the_budget() {
    let none = remaining(&[]);
    let spring = fed(d(2026, 4, 10), 0.75);
    assert!((none - remaining(std::slice::from_ref(&spring)) - 0.75).abs() < 1e-6);
    let planned = fed(d(2026, 11, 20), 1.0);
    let last_year = fed(d(2025, 11, 20), 1.0);
    assert!(
        (remaining(&[spring.clone(), planned, last_year]) - remaining(&[spring])).abs() < 1e-6,
        "a planned November feeding and last year's do not count"
    );
}

fn spent_budget() -> Vec<Application> {
    vec![fed(d(2026, 4, 10), 4.0)]
}

// @spec SOIL-NPK-010
#[test]
fn p_or_k_only_advice_states_pounds_of_the_example_analysis() {
    // The page and the explanation must show the same tenth: 0.75 lb N from a 12% analysis.
    let complete = suggest_ratio(0.75, 0.25, 0.5).1;
    assert_eq!(example_product_rate(&complete, 0.75, 0.25, 0.5), 6.3);

    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let p_only = make_test(6.5, Some(10.0), Some(200.0));
    let rec = calculate_npk_recommendation(&p_only, &profile, &spent_budget(), today()).unwrap();
    assert_eq!(rec.nitrogen_rate_lbs_per_1000sqft, 0.0);
    assert_eq!(rec.example_product_ratio, "0-20-0");
    assert!(
        (rec.product_rate_lbs_per_1000sqft - 1.3).abs() < 1e-6,
        "0.25 lb P₂O₅ from a 20% analysis is 1.25 lb, stated to 0.1: {}",
        rec.product_rate_lbs_per_1000sqft
    );

    let k_only = make_test(6.5, Some(30.0), Some(80.0));
    let rec = calculate_npk_recommendation(&k_only, &profile, &spent_budget(), today()).unwrap();
    assert_eq!(rec.example_product_ratio, "0-0-50");
    assert!((rec.product_rate_lbs_per_1000sqft - 1.0).abs() < 1e-6);
}

// @spec SOIL-NPK-012
#[test]
fn unreported_p_and_k_are_not_tested_and_get_no_p_or_k() {
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let ph_only = make_test(6.0, None, None);
    let rec = calculate_npk_recommendation(&ph_only, &profile, &[], today()).unwrap();
    assert_eq!(rec.phosphorus_level, NutrientLevel::NotTested);
    assert_eq!(rec.potassium_level, NutrientLevel::NotTested);
    assert_eq!(rec.phosphorus_rate_lbs_per_1000sqft, 0.0);
    assert_eq!(rec.potassium_rate_lbs_per_1000sqft, 0.0);
    assert!(
        rec.nitrogen_rate_lbs_per_1000sqft > 0.0,
        "N still from the budget"
    );

    let no_k = make_test(6.5, Some(30.0), None);
    let rec = calculate_npk_recommendation(&no_k, &profile, &[], today()).unwrap();
    assert_eq!(rec.potassium_level, NutrientLevel::NotTested);
    assert_eq!(
        rec.potassium_rate_lbs_per_1000sqft, 0.0,
        "no maintenance potash"
    );
}

// @spec SOIL-NPK-013
#[test]
fn the_explanation_names_what_was_not_tested() {
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let explain = |p, k, apps: &[Application]| {
        calculate_npk_recommendation(&make_test(6.5, p, k), &profile, apps, today())
            .unwrap()
            .explanation
    };
    for text in [
        explain(None, None, &[]),
        explain(None, None, &spent_budget()),
    ] {
        assert!(
            text.contains("Phosphorus and potassium were not tested"),
            "{text}"
        );
        assert!(!text.contains("shows ."), "{text}");
        assert!(!text.contains("adequate or high"), "{text}");
    }
    let text = explain(Some(30.0), None, &[]);
    assert!(text.contains("Potassium was not tested"), "{text}");
    assert!(text.contains("adequate P (30 ppm)"), "{text}");
}

fn suggestion_for(nutrient: &str, set: fn(&mut SoilTest)) -> String {
    let mut test = make_test(6.5, None, None);
    set(&mut test);
    evaluate_micronutrients(&test)
        .into_iter()
        .find(|r| r.nutrient == nutrient)
        .expect("low")
        .suggestion
}

// @spec SOIL-NUTR-004
#[test]
fn low_copper_gets_a_copper_amount_only() {
    let text = suggestion_for("Copper", |t| t.copper_ppm = Some(0.1));
    assert!(
        text.contains("0.25-0.5 oz of copper per 1,000 sq ft"),
        "{text}"
    );
    assert!(!text.to_lowercase().contains("sulfate"), "{text}");
}

// @spec SOIL-NUTR-005
#[test]
fn low_iron_gets_a_foliar_iron_amount_only() {
    let text = suggestion_for("Iron", |t| t.iron_ppm = Some(2.0));
    assert!(
        text.contains("0.4-0.8 oz of iron per 1,000 sq ft"),
        "{text}"
    );
    assert!(text.contains("foliar"), "{text}");
    let lower = text.to_lowercase();
    assert!(!lower.contains("sulfate"), "{text}");
    assert!(!lower.contains("chelated"), "{text}");
}
