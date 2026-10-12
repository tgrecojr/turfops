use crate::logic::soil_test_thresholds::*;
use crate::models::nitrogen_budget::annual_n_target;
use crate::models::{
    Application, LawnProfile, MicronutrientRecommendation, NpkRecommendation, NutrientLevel,
    PhRecommendation, SoilTest, SoilTestSummary, SoilType,
};
use chrono::{Datelike, NaiveDate};

// @spec SOIL-ADV-001, SOIL-ADV-002, SOIL-ADV-003, SOIL-ADV-004, SOIL-ADV-005
/// Top-level entry point: generate all soil-test-based recommendations.
/// `apps` are this year's applications; `today` is the lawn's local date.
pub fn generate_soil_test_recommendations(
    test: &SoilTest,
    profile: &LawnProfile,
    apps: &[Application],
    today: NaiveDate,
) -> SoilTestSummary {
    let ph_rec = calculate_ph_recommendation(test, profile);
    let npk_rec = calculate_npk_recommendation(test, profile, apps, today);
    let micro_recs = evaluate_micronutrients(test);

    SoilTestSummary {
        soil_test: test.clone(),
        ph_recommendation: ph_rec,
        npk_recommendation: npk_rec,
        micronutrient_recommendations: micro_recs,
    }
}

// @spec SOIL-PH-001, SOIL-PH-002, SOIL-PH-003, SOIL-PH-004, SOIL-PH-005
/// Calculate pH amendment recommendation (lime to raise, sulfur to lower).
pub fn calculate_ph_recommendation(
    test: &SoilTest,
    profile: &LawnProfile,
) -> Option<PhRecommendation> {
    let target_ph = if profile.grass_type.is_cool_season() {
        PH_TARGET_COOL_SEASON
    } else {
        PH_TARGET_WARM_SEASON
    };

    let delta = target_ph - test.ph;
    // No recommendation if pH is within 0.2 of target
    if delta.abs() < 0.2 {
        return None;
    }

    let (soil_type, soil_note) = match profile.soil_type {
        Some(st) => (st, ""),
        None => (
            SoilType::Loam,
            " (assumed Loam — set soil type in profile for more accurate rates)",
        ),
    };

    if delta > 0.0 {
        // Need to raise pH — apply lime
        let rate_per_unit = lime_rate_per_ph_unit(soil_type);
        let total_rate = (delta * rate_per_unit).min(MAX_LIME_LBS_PER_1000SQFT);
        let explanation = format!(
            "Your soil pH is {:.1}, target is {:.1} for {}. Apply calcitic or dolomitic lime at {:.0} lbs/1000 sqft. \
             If more than {:.0} lbs is needed, split into multiple applications 3-6 months apart.{}",
            test.ph,
            target_ph,
            profile.grass_type,
            total_rate,
            MAX_LIME_LBS_PER_1000SQFT,
            soil_note,
        );
        Some(PhRecommendation {
            current_ph: test.ph,
            target_ph,
            amendment: "Lime (calcitic or dolomitic)".to_string(),
            rate_lbs_per_1000sqft: total_rate,
            explanation,
        })
    } else {
        // Need to lower pH — apply elemental sulfur
        let rate_per_unit = sulfur_rate_per_ph_unit(soil_type);
        let total_rate = (delta.abs() * rate_per_unit).min(MAX_SULFUR_LBS_PER_1000SQFT);
        let explanation = format!(
            "Your soil pH is {:.1}, target is {:.1} for {}. Apply elemental sulfur at {:.0} lbs/1000 sqft. \
             If more than {:.0} lbs is needed, split into multiple applications 3-6 months apart.{}",
            test.ph,
            target_ph,
            profile.grass_type,
            total_rate,
            MAX_SULFUR_LBS_PER_1000SQFT,
            soil_note,
        );
        Some(PhRecommendation {
            current_ph: test.ph,
            target_ph,
            amendment: "Elemental sulfur".to_string(),
            rate_lbs_per_1000sqft: total_rate,
            explanation,
        })
    }
}

/// Nitrogen still to apply this year: the grass type's annual target less the lawn
/// feedings dated from Jan 1 through `today` (planned feedings are not yet applied).
// @spec SOIL-NPK-003, SOIL-NPK-004
fn remaining_nitrogen(profile: &LawnProfile, apps: &[Application], today: NaiveDate) -> f64 {
    let year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
    let applied: f64 = apps
        .iter()
        .filter(|a| (year_start..=today).contains(&a.application_date))
        .filter_map(Application::turf_nitrogen_lbs)
        .sum();
    (annual_n_target(profile.grass_type).recommended_lbs_per_1000sqft - applied).max(0.0)
}

/// Calculate N-P-K fertilizer recommendation integrated with nitrogen budget.
// @spec SOIL-NPK-012, SOIL-NPK-013, SOIL-NPK-001, SOIL-NPK-002, SOIL-NPK-005, SOIL-NPK-006, SOIL-NPK-011
pub fn calculate_npk_recommendation(
    test: &SoilTest,
    profile: &LawnProfile,
    apps: &[Application],
    today: NaiveDate,
) -> Option<NpkRecommendation> {
    let p_level = classify_phosphorus(test.phosphorus_ppm);
    let k_level = classify_potassium(test.potassium_ppm);
    let remaining_n = remaining_nitrogen(profile, apps, today);

    // Base N rate per application
    let n_rate = if remaining_n < 0.25 {
        0.0
    } else {
        remaining_n.min(0.75)
    };
    // P₂O₅ only for low P; K₂O for low K, or maintenance when adequate. Nothing for an
    // untested nutrient.
    let p_rate = match p_level {
        NutrientLevel::Low => 0.25,
        _ => 0.0,
    };
    let k_rate = match k_level {
        NutrientLevel::Low => 0.5,
        NutrientLevel::Adequate => 0.25,
        _ => 0.0,
    };
    let levels = levels_text(test, p_level, k_level);

    if n_rate == 0.0 && p_rate == 0.0 && k_rate == 0.0 {
        let budget = if remaining_n < 0.25 {
            format!(
                "Your nitrogen budget is nearly used up ({remaining_n:.2} lbs N/1000 sqft remaining)."
            )
        } else {
            format!("You have {remaining_n:.2} lbs N/1000 sqft remaining in your annual budget.")
        };
        return Some(NpkRecommendation {
            phosphorus_level: p_level,
            potassium_level: k_level,
            recommended_ratio: "N/A".to_string(),
            nitrogen_rate_lbs_per_1000sqft: 0.0,
            phosphorus_rate_lbs_per_1000sqft: 0.0,
            potassium_rate_lbs_per_1000sqft: 0.0,
            product_rate_lbs_per_1000sqft: 0.0,
            example_product_ratio: "N/A".to_string(),
            remaining_n_budget_lbs_per_1000sqft: remaining_n,
            explanation: format!("{levels} {budget} No fertilizer is needed now."),
        });
    }

    let (ratio, product_ratio) = suggest_ratio(n_rate, p_rate, k_rate);
    let product_rate = example_product_rate(&product_ratio, n_rate, p_rate, k_rate);

    let soil_note = if profile.soil_type.is_none() {
        " Soil type not set — using Loam defaults."
    } else {
        ""
    };
    let budget_note = if remaining_n < 0.5 {
        format!(
            " Note: only {remaining_n:.2} lbs N/1000 sqft remaining in annual budget — N rate capped accordingly.",
        )
    } else {
        String::new()
    };

    let explanation = format!(
        "{levels} You have {remaining_n:.2} lbs N/1000 sqft remaining in your annual budget. \
         Recommend {} per 1000 sqft with a {ratio} ratio fertilizer. \
         For a {product_ratio} product, apply {product_rate:.1} lbs/1000 sqft.{budget_note}{soil_note}",
        amounts_text(n_rate, p_rate, k_rate),
    );

    Some(NpkRecommendation {
        phosphorus_level: p_level,
        potassium_level: k_level,
        recommended_ratio: ratio,
        nitrogen_rate_lbs_per_1000sqft: n_rate,
        phosphorus_rate_lbs_per_1000sqft: p_rate,
        potassium_rate_lbs_per_1000sqft: k_rate,
        product_rate_lbs_per_1000sqft: product_rate,
        example_product_ratio: product_ratio,
        remaining_n_budget_lbs_per_1000sqft: remaining_n,
        explanation,
    })
}

/// Evaluate each micronutrient against thresholds. Iron and copper salts are also
/// pesticide actives, so those two suggestions state only the nutrient amount.
// @spec SOIL-NUTR-001, SOIL-NUTR-002, SOIL-NUTR-003, SOIL-NUTR-004, SOIL-NUTR-005
pub fn evaluate_micronutrients(test: &SoilTest) -> Vec<MicronutrientRecommendation> {
    let mut recs = Vec::new();

    let checks: &[(&str, Option<f64>, f64, &str)] = &[
        ("Calcium", test.calcium_ppm, CALCIUM_LOW, "Apply gypsum (calcium sulfate) at 20-40 lbs/1000 sqft"),
        ("Magnesium", test.magnesium_ppm, MAGNESIUM_LOW, "Apply dolomitic lime (also raises pH) or Epsom salt at 5-10 lbs/1000 sqft"),
        ("Sulfur", test.sulfur_ppm, SULFUR_LOW, "Apply elemental sulfur at 5 lbs/1000 sqft or use ammonium sulfate fertilizer"),
        ("Iron", test.iron_ppm, IRON_LOW, "Apply 0.4-0.8 oz of iron per 1,000 sq ft as a foliar application; follow the product label for how much product supplies it"),
        ("Manganese", test.manganese_ppm, MANGANESE_LOW, "Apply manganese sulfate at 2-4 oz/1000 sqft"),
        ("Zinc", test.zinc_ppm, ZINC_LOW, "Apply zinc sulfate at 1-2 oz/1000 sqft"),
        ("Boron", test.boron_ppm, BORON_LOW, "Apply borax at 0.5-1 oz/1000 sqft — caution: boron toxicity occurs at low concentrations"),
        ("Copper", test.copper_ppm, COPPER_LOW, "Apply 0.25-0.5 oz of copper per 1,000 sq ft; follow the product label for how much product supplies it"),
    ];

    for (nutrient, value, threshold, suggestion) in checks {
        if let Some(ppm) = value {
            let level = if *ppm < *threshold {
                NutrientLevel::Low
            } else {
                NutrientLevel::Adequate
            };
            if level == NutrientLevel::Low {
                recs.push(MicronutrientRecommendation {
                    nutrient: nutrient.to_string(),
                    current_ppm: *ppm,
                    threshold_ppm: *threshold,
                    level,
                    suggestion: suggestion.to_string(),
                });
            }
        }
    }

    recs
}

fn classify_phosphorus(ppm: Option<f64>) -> NutrientLevel {
    match ppm {
        Some(p) if p < PHOSPHORUS_LOW => NutrientLevel::Low,
        Some(p) if p > PHOSPHORUS_HIGH => NutrientLevel::High,
        Some(_) => NutrientLevel::Adequate,
        None => NutrientLevel::NotTested,
    }
}

fn classify_potassium(ppm: Option<f64>) -> NutrientLevel {
    match ppm {
        Some(k) if k < POTASSIUM_LOW => NutrientLevel::Low,
        Some(k) if k > POTASSIUM_HIGH => NutrientLevel::High,
        Some(_) => NutrientLevel::Adequate,
        None => NutrientLevel::NotTested,
    }
}

mod ratio;
use ratio::{amounts_text, example_product_rate, levels_text, suggest_ratio};

#[cfg(test)]
mod advice_tests;
#[cfg(test)]
mod gap_tests;
#[cfg(test)]
mod npk_tests;
#[cfg(test)]
mod ph_tests;
#[cfg(test)]
mod tests;
