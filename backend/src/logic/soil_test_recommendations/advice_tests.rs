//! The advice as a whole (materials, amounts, no gating) and the deficiency evaluator.

use super::tests::{make_profile, make_test, today};
use super::*;
use crate::models::GrassType;

/// A test low in everything it reports.
fn depleted() -> SoilTest {
    let mut test = make_test(5.5, Some(10.0), Some(80.0));
    test.calcium_ppm = Some(300.0);
    test.magnesium_ppm = Some(30.0);
    test.sulfur_ppm = Some(5.0);
    test.iron_ppm = Some(2.0);
    test.manganese_ppm = Some(2.0);
    test.zinc_ppm = Some(0.5);
    test.boron_ppm = Some(0.2);
    test.copper_ppm = Some(0.2);
    test
}

fn advice(test: &SoilTest, on: NaiveDate) -> SoilTestSummary {
    let profile = make_profile(GrassType::TallFescue, Some(SoilType::Loam));
    generate_soil_test_recommendations(test, &profile, &[], on)
}

fn suggestion(summary: &SoilTestSummary, nutrient: &str) -> String {
    summary
        .micronutrient_recommendations
        .iter()
        .find(|r| r.nutrient == nutrient)
        .unwrap_or_else(|| panic!("{nutrient} not low"))
        .suggestion
        .clone()
}

// @spec SOIL-ADV-002
#[test]
fn advice_names_materials_and_analyses_never_a_product() {
    let summary = advice(&depleted(), today());
    assert_eq!(
        summary.ph_recommendation.as_ref().unwrap().amendment,
        "Lime (calcitic or dolomitic)"
    );
    let analysis = &summary
        .npk_recommendation
        .as_ref()
        .unwrap()
        .example_product_ratio;
    let parts: Vec<&str> = analysis.split('-').collect();
    assert_eq!(parts.len(), 3, "{analysis}");
    assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()), "{analysis}");

    let materials = [
        ("Calcium", "gypsum"),
        ("Magnesium", "dolomitic lime"),
        ("Sulfur", "elemental sulfur"),
        ("Iron", "iron"),
        ("Manganese", "manganese sulfate"),
        ("Zinc", "zinc sulfate"),
        ("Boron", "borax"),
        ("Copper", "copper"),
    ];
    for (nutrient, material) in materials {
        let text = suggestion(&summary, nutrient);
        assert!(text.starts_with("Apply "), "{text}");
        assert!(text.contains(material), "{nutrient}: {text}");
        assert!(!text.contains('®') && !text.contains('™'), "{text}");
    }
}

// @spec SOIL-ADV-003
#[test]
fn advice_states_agronomic_amounts_per_1000_sq_ft_and_no_label_rate() {
    let summary = advice(&depleted(), today());
    let ph = summary.ph_recommendation.as_ref().unwrap();
    assert!(
        ph.explanation.contains("at 50 lbs/1000 sqft"),
        "{}",
        ph.explanation
    );
    let npk = summary.npk_recommendation.as_ref().unwrap();
    assert!(
        npk.explanation.contains("lbs P₂O₅") && npk.explanation.contains("lbs K₂O"),
        "{}",
        npk.explanation
    );
    assert!(npk.explanation.contains("apply 6.3 lbs/1000 sqft"));
    for rec in &summary.micronutrient_recommendations {
        let per_1000 =
            rec.suggestion.contains("/1000 sqft") || rec.suggestion.contains("per 1,000 sq ft");
        assert!(per_1000, "{}: {}", rec.nutrient, rec.suggestion);
    }
    let json = serde_json::to_string(&summary).unwrap();
    assert!(!json.contains("label_rate"), "{json}");
}

// @spec SOIL-ADV-004
#[test]
fn advice_is_the_same_in_any_season() {
    let year = |m, d| NaiveDate::from_ymd_opt(2026, m, d).unwrap();
    let test = depleted();
    let winter = serde_json::to_value(advice(&test, year(1, 15))).unwrap();
    for day in [year(4, 1), year(7, 20), year(12, 31)] {
        let other = serde_json::to_value(advice(&test, day)).unwrap();
        assert_eq!(other, winter, "{day}");
    }
}

// @spec SOIL-NUTR-001
#[test]
fn a_nutrient_below_its_threshold_is_low_with_value_threshold_and_suggestion() {
    let summary = advice(&depleted(), today());
    let expected = [
        ("Calcium", 300.0, 500.0),
        ("Magnesium", 30.0, 50.0),
        ("Sulfur", 5.0, 10.0),
        ("Iron", 2.0, 5.0),
        ("Manganese", 2.0, 5.0),
        ("Zinc", 0.5, 1.0),
        ("Boron", 0.2, 0.5),
        ("Copper", 0.2, 0.5),
    ];
    let recs = &summary.micronutrient_recommendations;
    assert_eq!(recs.len(), expected.len());
    for (nutrient, value, threshold) in expected {
        let rec = recs.iter().find(|r| r.nutrient == nutrient).unwrap();
        assert_eq!(rec.level, NutrientLevel::Low, "{nutrient}");
        assert_eq!(rec.current_ppm, value, "{nutrient}");
        assert_eq!(rec.threshold_ppm, threshold, "{nutrient}");
        assert!(!rec.suggestion.is_empty(), "{nutrient}");
    }
}

// @spec SOIL-NUTR-002
#[test]
fn a_nutrient_at_its_threshold_unreported_or_plentiful_is_not_listed() {
    let mut at = make_test(6.5, None, None);
    at.calcium_ppm = Some(500.0);
    at.magnesium_ppm = Some(50.0);
    at.sulfur_ppm = Some(10.0);
    at.iron_ppm = Some(5.0);
    at.manganese_ppm = Some(5.0);
    at.zinc_ppm = Some(1.0);
    at.boron_ppm = Some(0.5);
    at.copper_ppm = Some(0.5);
    assert!(evaluate_micronutrients(&at).is_empty());
    assert!(evaluate_micronutrients(&make_test(6.5, None, None)).is_empty());

    let mut plenty = at.clone();
    plenty.calcium_ppm = Some(5000.0);
    plenty.iron_ppm = Some(500.0);
    assert!(evaluate_micronutrients(&plenty).is_empty(), "never High");
}

// @spec SOIL-NUTR-003
#[test]
fn each_low_nutrient_gets_its_material_with_an_amount() {
    let summary = advice(&depleted(), today());
    let expected: [(&str, &[&str]); 6] = [
        ("Calcium", &["gypsum", "lbs/1000 sqft"]),
        (
            "Magnesium",
            &["dolomitic lime", "raises pH", "Epsom salt", "lbs/1000 sqft"],
        ),
        (
            "Sulfur",
            &[
                "elemental sulfur",
                "lbs/1000 sqft",
                "ammonium sulfate fertilizer",
            ],
        ),
        ("Manganese", &["manganese sulfate", "oz/1000 sqft"]),
        ("Zinc", &["zinc sulfate", "oz/1000 sqft"]),
        ("Boron", &["borax", "oz/1000 sqft", "boron toxicity"]),
    ];
    for (nutrient, parts) in expected {
        let text = suggestion(&summary, nutrient);
        for part in parts {
            assert!(text.contains(part), "{nutrient} {part}: {text}");
        }
    }
}
