use crate::logic::soil_test_thresholds::*;
use crate::models::nitrogen_budget::annual_n_target;
use crate::models::{
    Application, LawnProfile, MicronutrientRecommendation, NpkRecommendation, NutrientLevel,
    PhRecommendation, SoilTest, SoilTestSummary, SoilType,
};

/// Top-level entry point: generate all soil-test-based recommendations.
pub fn generate_soil_test_recommendations(
    test: &SoilTest,
    profile: &LawnProfile,
    apps: &[Application],
) -> SoilTestSummary {
    let ph_rec = calculate_ph_recommendation(test, profile);
    let npk_rec = calculate_npk_recommendation(test, profile, apps);
    let micro_recs = evaluate_micronutrients(test);

    SoilTestSummary {
        soil_test: test.clone(),
        ph_recommendation: ph_rec,
        npk_recommendation: npk_rec,
        micronutrient_recommendations: micro_recs,
    }
}

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

/// Calculate N-P-K fertilizer recommendation integrated with nitrogen budget.
pub fn calculate_npk_recommendation(
    test: &SoilTest,
    profile: &LawnProfile,
    apps: &[Application],
) -> Option<NpkRecommendation> {
    let p_level = classify_phosphorus(test.phosphorus_ppm);
    let k_level = classify_potassium(test.potassium_ppm);

    // Calculate remaining N budget
    let n_target = annual_n_target(profile.grass_type);
    let ytd_n_applied: f64 = apps
        .iter()
        .filter_map(|a| match (a.nitrogen_pct, a.rate_per_1000sqft) {
            (Some(n_pct), Some(rate)) if n_pct > 0.0 && rate > 0.0 => Some(n_pct / 100.0 * rate),
            _ => None,
        })
        .sum();

    let remaining_n = (n_target.recommended_lbs_per_1000sqft - ytd_n_applied).max(0.0);

    // Base N rate per application
    let n_rate = if remaining_n < 0.25 {
        0.0
    } else {
        remaining_n.min(0.75)
    };

    // P2O5 and K2O rates based on soil test levels
    let p_rate = match p_level {
        NutrientLevel::Low => 0.25,
        NutrientLevel::Adequate | NutrientLevel::High => 0.0,
    };

    let k_rate = match k_level {
        NutrientLevel::Low => 0.5,
        NutrientLevel::Adequate => 0.25,
        NutrientLevel::High => 0.0,
    };

    // If nothing is needed, no recommendation
    if n_rate == 0.0 && p_rate == 0.0 && k_rate == 0.0 {
        let explanation = if remaining_n < 0.25 {
            format!(
                "Your nitrogen budget is nearly exhausted ({:.2} lbs/1000 sqft remaining). \
                 Soil P and K levels are adequate or high. No fertilizer application recommended at this time.",
                remaining_n,
            )
        } else {
            "Soil P and K levels are adequate or high and N budget allows no further application."
                .to_string()
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
            explanation,
        });
    }

    // Determine a practical ratio
    let (ratio, product_ratio) = suggest_ratio(n_rate, p_rate, k_rate);

    // Calculate product application rate based on N content of example product
    let product_n_pct = parse_first_ratio_number(&product_ratio);
    let product_rate = if product_n_pct > 0.0 && n_rate > 0.0 {
        n_rate / (product_n_pct / 100.0)
    } else if product_n_pct > 0.0 {
        // No N needed but product has N — use P or K to determine rate
        let product_p_pct = parse_second_ratio_number(&product_ratio);
        let product_k_pct = parse_third_ratio_number(&product_ratio);
        if product_p_pct > 0.0 && p_rate > 0.0 {
            p_rate / (product_p_pct / 100.0)
        } else if product_k_pct > 0.0 && k_rate > 0.0 {
            k_rate / (product_k_pct / 100.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Build explanation
    let mut parts = Vec::new();
    if let Some(p_ppm) = test.phosphorus_ppm {
        parts.push(format!(
            "{} P ({:.0} ppm)",
            p_level.as_str().to_lowercase(),
            p_ppm
        ));
    }
    if let Some(k_ppm) = test.potassium_ppm {
        parts.push(format!(
            "{} K ({:.0} ppm)",
            k_level.as_str().to_lowercase(),
            k_ppm
        ));
    }

    let soil_note = if profile.soil_type.is_none() {
        " Soil type not set — using Loam defaults."
    } else {
        ""
    };

    let budget_note = if remaining_n < 0.5 {
        format!(
            " Note: only {:.2} lbs N/1000 sqft remaining in annual budget — N rate capped accordingly.",
            remaining_n,
        )
    } else {
        String::new()
    };

    let explanation = format!(
        "Your soil test shows {}. You have {:.2} lbs N/1000 sqft remaining in your annual budget. \
         Recommend {:.2} lbs N/1000 sqft with a {} ratio fertilizer. \
         For a {} product, apply {:.1} lbs/1000 sqft.{}{}",
        parts.join(" and "),
        remaining_n,
        n_rate,
        ratio,
        product_ratio,
        product_rate,
        budget_note,
        soil_note,
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

/// Evaluate each micronutrient against thresholds.
pub fn evaluate_micronutrients(test: &SoilTest) -> Vec<MicronutrientRecommendation> {
    let mut recs = Vec::new();

    let checks: &[(&str, Option<f64>, f64, &str)] = &[
        ("Calcium", test.calcium_ppm, CALCIUM_LOW, "Apply gypsum (calcium sulfate) at 20-40 lbs/1000 sqft"),
        ("Magnesium", test.magnesium_ppm, MAGNESIUM_LOW, "Apply dolomitic lime (also raises pH) or Epsom salt at 5-10 lbs/1000 sqft"),
        ("Sulfur", test.sulfur_ppm, SULFUR_LOW, "Apply elemental sulfur at 5 lbs/1000 sqft or use ammonium sulfate fertilizer"),
        ("Iron", test.iron_ppm, IRON_LOW, "Apply chelated iron (ferrous sulfate) at 2-4 oz/1000 sqft as foliar spray"),
        ("Manganese", test.manganese_ppm, MANGANESE_LOW, "Apply manganese sulfate at 2-4 oz/1000 sqft"),
        ("Zinc", test.zinc_ppm, ZINC_LOW, "Apply zinc sulfate at 1-2 oz/1000 sqft"),
        ("Boron", test.boron_ppm, BORON_LOW, "Apply borax at 0.5-1 oz/1000 sqft — caution: boron toxicity occurs at low concentrations"),
        ("Copper", test.copper_ppm, COPPER_LOW, "Apply copper sulfate at 1-2 oz/1000 sqft"),
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
        None => NutrientLevel::Adequate,
    }
}

fn classify_potassium(ppm: Option<f64>) -> NutrientLevel {
    match ppm {
        Some(k) if k < POTASSIUM_LOW => NutrientLevel::Low,
        Some(k) if k > POTASSIUM_HIGH => NutrientLevel::High,
        Some(_) => NutrientLevel::Adequate,
        None => NutrientLevel::Adequate,
    }
}

/// Suggest a simple N-P-K ratio string and a common product ratio.
fn suggest_ratio(n: f64, p: f64, k: f64) -> (String, String) {
    // Normalize to smallest non-zero value to get a simple ratio
    let min_nonzero = [n, p, k]
        .iter()
        .copied()
        .filter(|v| *v > 0.0)
        .fold(f64::MAX, f64::min);

    let rn = if n > 0.0 {
        (n / min_nonzero).round() as u32
    } else {
        0
    };
    let rp = if p > 0.0 {
        (p / min_nonzero).round() as u32
    } else {
        0
    };
    let rk = if k > 0.0 {
        (k / min_nonzero).round() as u32
    } else {
        0
    };

    let ratio = format!("{}-{}-{}", rn, rp, rk);

    // Map to common commercial product ratios
    let product = match (rn > 0, rp > 0, rk > 0) {
        (true, true, true) => {
            if rn >= 2 * rk {
                "24-4-12"
            } else {
                "12-4-8"
            }
        }
        (true, true, false) => "15-5-0",
        (true, false, true) => {
            if rn >= 2 * rk {
                "24-0-11"
            } else {
                "16-0-8"
            }
        }
        (true, false, false) => "46-0-0",
        (false, true, true) => "0-5-10",
        (false, true, false) => "0-20-0",
        (false, false, true) => "0-0-50",
        (false, false, false) => "N/A",
    };

    (ratio, product.to_string())
}

fn parse_first_ratio_number(ratio: &str) -> f64 {
    ratio
        .split('-')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn parse_second_ratio_number(ratio: &str) -> f64 {
    ratio
        .split('-')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn parse_third_ratio_number(ratio: &str) -> f64 {
    ratio
        .split('-')
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GrassType, LawnProfile};
    use chrono::Utc;

    fn make_test(ph: f64, p: Option<f64>, k: Option<f64>) -> SoilTest {
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

    fn make_profile(grass: GrassType, soil: Option<SoilType>) -> LawnProfile {
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
        let rec = calculate_npk_recommendation(&test, &profile, &[]).unwrap();
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
        let rec = calculate_npk_recommendation(&test, &profile, &[]).unwrap();
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

    #[test]
    fn classify_phosphorus_levels() {
        assert_eq!(classify_phosphorus(Some(10.0)), NutrientLevel::Low);
        assert_eq!(classify_phosphorus(Some(30.0)), NutrientLevel::Adequate);
        assert_eq!(classify_phosphorus(Some(60.0)), NutrientLevel::High);
        assert_eq!(classify_phosphorus(None), NutrientLevel::Adequate);
    }

    #[test]
    fn classify_potassium_levels() {
        assert_eq!(classify_potassium(Some(80.0)), NutrientLevel::Low);
        assert_eq!(classify_potassium(Some(140.0)), NutrientLevel::Adequate);
        assert_eq!(classify_potassium(Some(200.0)), NutrientLevel::High);
        assert_eq!(classify_potassium(None), NutrientLevel::Adequate);
    }

    #[test]
    fn suggest_ratio_produces_valid_product() {
        let (ratio, product) = suggest_ratio(0.75, 0.25, 0.5);
        assert_eq!(ratio, "3-1-2");
        assert!(!product.is_empty());
        assert_ne!(product, "N/A");
    }
}
