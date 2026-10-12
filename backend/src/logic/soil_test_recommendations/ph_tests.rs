//! The pH evaluator: target, dead band, lime and sulfur amounts, and the explanation.

use super::tests::{make_profile, make_test};
use super::*;
use crate::models::GrassType;

const SOILS: [SoilType; 6] = [
    SoilType::Sandy,
    SoilType::SandyLoam,
    SoilType::Loam,
    SoilType::SiltLoam,
    SoilType::ClayLoam,
    SoilType::Clay,
];

fn ph_advice(ph: f64, grass: GrassType, soil: Option<SoilType>) -> Option<PhRecommendation> {
    calculate_ph_recommendation(&make_test(ph, None, None), &make_profile(grass, soil))
}

// @spec SOIL-PH-001
#[test]
fn cool_season_lawns_aim_for_ph_six_and_a_half() {
    for grass in [
        GrassType::TallFescue,
        GrassType::KentuckyBluegrass,
        GrassType::PerennialRyegrass,
        GrassType::FineFescue,
    ] {
        let rec = ph_advice(5.5, grass, Some(SoilType::Loam)).unwrap();
        assert_eq!(rec.target_ph, 6.5, "{grass:?}");
    }
}

// @spec SOIL-ADV-005
#[test]
fn a_mixed_lawn_is_advised_as_cool_season() {
    let rec = ph_advice(5.5, GrassType::Mixed, Some(SoilType::Loam)).unwrap();
    assert_eq!(rec.target_ph, 6.5);
    assert!(ph_advice(6.4, GrassType::Mixed, Some(SoilType::Loam)).is_none());
}

// @spec SOIL-PH-002
#[test]
fn a_ph_within_two_tenths_of_the_target_gets_no_advice() {
    for ph in [6.31, 6.4, 6.5, 6.6, 6.69] {
        assert!(
            ph_advice(ph, GrassType::TallFescue, Some(SoilType::Loam)).is_none(),
            "{ph}"
        );
    }
    for ph in [6.3, 6.7] {
        assert!(
            ph_advice(ph, GrassType::TallFescue, Some(SoilType::Loam)).is_some(),
            "{ph}: 0.2 away is advised"
        );
    }
}

// @spec SOIL-PH-003
#[test]
fn acid_soil_gets_lime_by_the_gap_times_the_soil_factor_capped_at_fifty() {
    let factors = [25.0, 35.0, 50.0, 60.0, 70.0, 80.0];
    for (soil, factor) in SOILS.into_iter().zip(factors) {
        let rec = ph_advice(6.0, GrassType::TallFescue, Some(soil)).unwrap();
        assert_eq!(rec.amendment, "Lime (calcitic or dolomitic)");
        assert!(
            (rec.rate_lbs_per_1000sqft - 0.5 * factor).abs() < 1e-9,
            "{soil:?}: {}",
            rec.rate_lbs_per_1000sqft
        );
    }
    let capped = ph_advice(4.5, GrassType::TallFescue, Some(SoilType::Clay)).unwrap();
    assert_eq!(capped.rate_lbs_per_1000sqft, 50.0, "2.0 × 80 = 160, capped");
}

// @spec SOIL-PH-004
#[test]
fn alkaline_soil_gets_sulfur_by_the_gap_times_the_soil_factor_capped_at_ten() {
    let factors = [5.0, 7.0, 10.0, 12.0, 15.0, 18.0];
    for (soil, factor) in SOILS.into_iter().zip(factors) {
        let rec = ph_advice(7.0, GrassType::TallFescue, Some(soil)).unwrap();
        assert_eq!(rec.amendment, "Elemental sulfur");
        assert!(
            (rec.rate_lbs_per_1000sqft - 0.5 * factor).abs() < 1e-9,
            "{soil:?}: {}",
            rec.rate_lbs_per_1000sqft
        );
    }
    let capped = ph_advice(8.0, GrassType::TallFescue, Some(SoilType::Clay)).unwrap();
    assert_eq!(capped.rate_lbs_per_1000sqft, 10.0, "1.5 × 18 = 27, capped");
}

// @spec SOIL-PH-005
#[test]
fn the_ph_explanation_states_both_phs_the_material_its_amount_and_the_split() {
    let split = "split into multiple applications 3-6 months apart";
    let lime = ph_advice(6.0, GrassType::TallFescue, Some(SoilType::Loam)).unwrap();
    let text = &lime.explanation;
    for part in [
        "6.0",
        "6.5",
        "lime at 25 lbs/1000 sqft",
        "more than 50 lbs",
        split,
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }

    let sulfur = ph_advice(7.0, GrassType::TallFescue, Some(SoilType::Loam)).unwrap();
    let text = &sulfur.explanation;
    for part in [
        "7.0",
        "6.5",
        "elemental sulfur at 5 lbs/1000 sqft",
        "more than 10 lbs",
        split,
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
}
