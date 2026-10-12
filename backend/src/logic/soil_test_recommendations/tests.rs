//! pH, N-P-K and deficiency evaluators.

use super::*;
use crate::models::{GrassType, LawnProfile};
use chrono::{NaiveDate, Utc};

pub(super) fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 11).unwrap()
}

pub(super) fn make_test(ph: f64, p: Option<f64>, k: Option<f64>) -> SoilTest {
    SoilTest {
        id: Some(1),
        lawn_profile_id: 1,
        test_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        lab_name: None,
        ph,
        buffer_ph: None,
        phosphorus_ppm: p,
        potassium_ppm: k,
        calcium_ppm: None,
        magnesium_ppm: None,
        sulfur_ppm: None,
        iron_ppm: None,
        manganese_ppm: None,
        zinc_ppm: None,
        boron_ppm: None,
        copper_ppm: None,
        organic_matter_pct: None,
        cec: None,
        notes: None,
        created_at: Utc::now(),
    }
}

pub(super) fn make_profile(grass: GrassType, soil: Option<SoilType>) -> LawnProfile {
    LawnProfile {
        id: Some(1),
        name: "Test".to_string(),
        grass_type: grass,
        usda_zone: "7a".to_string(),
        soil_type: soil,
        lawn_size_sqft: Some(5000.0),
        irrigation_type: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn ph_within_tolerance_no_recommendation() {
    let test = make_test(6.4, None, None);
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    assert!(calculate_ph_recommendation(&test, &profile).is_none());
}

#[test]
fn low_ph_recommends_lime() {
    let test = make_test(5.5, None, None);
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let rec = calculate_ph_recommendation(&test, &profile).unwrap();
    assert!(rec.amendment.contains("Lime"));
    assert!(rec.rate_lbs_per_1000sqft > 0.0);
    assert!(rec.rate_lbs_per_1000sqft <= MAX_LIME_LBS_PER_1000SQFT);
}

#[test]
fn high_ph_recommends_sulfur() {
    let test = make_test(7.5, None, None);
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let rec = calculate_ph_recommendation(&test, &profile).unwrap();
    assert!(rec.amendment.contains("sulfur"));
    assert!(rec.rate_lbs_per_1000sqft > 0.0);
    assert!(rec.rate_lbs_per_1000sqft <= MAX_SULFUR_LBS_PER_1000SQFT);
}

#[test]
fn warm_season_uses_lower_target() {
    let test = make_test(6.3, None, None);
    let profile = make_profile(GrassType::Bermuda, Some(SoilType::Loam));
    // 6.3 is within 0.2 of 6.0 target? No, delta = -0.3 => needs sulfur
    let rec = calculate_ph_recommendation(&test, &profile);
    assert!(rec.is_some());
}

#[test]
fn no_soil_type_defaults_to_loam() {
    let test = make_test(5.5, None, None);
    let profile = make_profile(GrassType::TallFescue, None);
    let rec = calculate_ph_recommendation(&test, &profile).unwrap();
    assert!(rec.explanation.contains("assumed Loam"));
}

#[test]
fn npk_low_p_low_k() {
    let test = make_test(6.5, Some(10.0), Some(80.0));
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let rec = calculate_npk_recommendation(&test, &profile, &[], today()).unwrap();
    assert_eq!(rec.phosphorus_level, NutrientLevel::Low);
    assert_eq!(rec.potassium_level, NutrientLevel::Low);
    assert!(rec.nitrogen_rate_lbs_per_1000sqft > 0.0);
    assert!(rec.phosphorus_rate_lbs_per_1000sqft > 0.0);
    assert!(rec.potassium_rate_lbs_per_1000sqft > 0.0);
}

#[test]
fn npk_adequate_p_high_k() {
    let test = make_test(6.5, Some(30.0), Some(200.0));
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    let rec = calculate_npk_recommendation(&test, &profile, &[], today()).unwrap();
    assert_eq!(rec.phosphorus_level, NutrientLevel::Adequate);
    assert_eq!(rec.potassium_level, NutrientLevel::High);
    assert_eq!(rec.phosphorus_rate_lbs_per_1000sqft, 0.0);
    assert_eq!(rec.potassium_rate_lbs_per_1000sqft, 0.0);
}

#[test]
fn micronutrients_low_iron() {
    let mut test = make_test(6.5, None, None);
    test.iron_ppm = Some(2.0);
    let recs = evaluate_micronutrients(&test);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].nutrient, "Iron");
    assert_eq!(recs[0].level, NutrientLevel::Low);
}

#[test]
fn micronutrients_adequate_returns_empty() {
    let mut test = make_test(6.5, None, None);
    test.iron_ppm = Some(10.0);
    test.calcium_ppm = Some(1000.0);
    let recs = evaluate_micronutrients(&test);
    assert!(recs.is_empty());
}

// @spec SOIL-NPK-012
#[test]
fn classify_phosphorus_levels() {
    assert_eq!(classify_phosphorus(Some(10.0)), NutrientLevel::Low);
    assert_eq!(classify_phosphorus(Some(30.0)), NutrientLevel::Adequate);
    assert_eq!(classify_phosphorus(Some(60.0)), NutrientLevel::High);
    assert_eq!(classify_phosphorus(None), NutrientLevel::NotTested);
}

// @spec SOIL-NPK-012
#[test]
fn classify_potassium_levels() {
    assert_eq!(classify_potassium(Some(80.0)), NutrientLevel::Low);
    assert_eq!(classify_potassium(Some(140.0)), NutrientLevel::Adequate);
    assert_eq!(classify_potassium(Some(200.0)), NutrientLevel::High);
    assert_eq!(classify_potassium(None), NutrientLevel::NotTested);
}

#[test]
fn suggest_ratio_produces_valid_product() {
    let (ratio, product) = suggest_ratio(0.75, 0.25, 0.5);
    assert_eq!(ratio, "3-1-2");
    assert!(!product.is_empty());
    assert_ne!(product, "N/A");
}
