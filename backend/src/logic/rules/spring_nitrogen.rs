use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Spring nitrogen delay rule
///
/// A common beginner mistake is fertilizing too early in spring. This forces
/// top growth before roots wake up, weakening the plant heading into summer.
///
/// Proper spring timing:
/// - Wait until soil temp reaches 55°F (7-day average)
/// - GDD >= 50 confirms active growth even if soil temp hasn't crossed 55°F
/// - GDD >= 150 adds urgency — growth well established
/// - Wait until after first 2-3 mowings
/// - Let the lawn "wake up" naturally first
///
/// This rule warns against early nitrogen and advises patience.
pub struct SpringNitrogenRule;

impl Rule for SpringNitrogenRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        // Only relevant for cool-season grasses
        if !profile.grass_type.is_cool_season() {
            return None;
        }

        let today = Local::now().date_naive();
        let current_year = today.year();

        // Only relevant in late winter/early spring (Feb - May)
        let month = today.month();
        if !(2..=5).contains(&month) {
            return None;
        }

        // Get soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;

        // GDD data
        let gdd_ytd = env.gdd_base50_ytd;

        // Check for spring fertilizer applications
        let spring_start = NaiveDate::from_ymd_opt(current_year, 2, 1)?;
        let spring_fert_apps: Vec<&Application> = history
            .iter()
            .filter(|app| {
                app.application_type == ApplicationType::Fertilizer
                    && app.application_date.year() == current_year
                    && app.application_date >= spring_start
            })
            .collect();

        let has_spring_fert = !spring_fert_apps.is_empty();

        // A4: May 1 hard cutoff (Missouri g6705)
        if month == 5 {
            if !has_spring_fert {
                return Some(build_may_cutoff_warning(soil_temp_avg));
            }
            return None;
        }

        // Determine appropriate recommendation
        if soil_temp_avg < SPRING_N_APPROACHING_SOIL_F {
            // Too cold - definitely don't fertilize
            if has_spring_fert {
                // Already fertilized when too cold - warn about the mistake
                Some(build_too_early_warning(soil_temp_avg))
            } else {
                // Good - they're waiting. Reinforce patience.
                Some(build_patience_advisory(soil_temp_avg))
            }
        } else if (SPRING_N_APPROACHING_SOIL_F..SPRING_N_MIN_SOIL_F).contains(&soil_temp_avg) {
            // Getting close - still advise waiting UNLESS GDD confirms active growth
            if !has_spring_fert {
                let gdd_ready = gdd_ytd.is_some_and(|gdd| gdd >= SPRING_N_GDD_READY);
                if gdd_ready {
                    // GDD confirms active growth — promote to "ready"
                    Some(build_ready_to_fertilize(soil_temp_avg, profile, gdd_ytd))
                } else {
                    Some(build_almost_ready(soil_temp_avg, gdd_ytd))
                }
            } else {
                None // They already applied, no point warning now
            }
        } else if (SPRING_N_MIN_SOIL_F..=SPRING_N_READY_HIGH_F).contains(&soil_temp_avg) {
            // Good range - if they haven't fertilized, now is okay
            if !has_spring_fert {
                Some(build_ready_to_fertilize(soil_temp_avg, profile, gdd_ytd))
            } else {
                None
            }
        } else {
            // Above 65°F - late spring, different considerations
            None
        }
    }
}

fn build_too_early_warning(soil_temp: f64) -> Recommendation {
    Recommendation::new(
        "spring_n_too_early",
        RecommendationCategory::Fertilizer,
        Severity::Warning,
        "Spring Fertilizer Applied Too Early",
        format!(
            "Fertilizer was applied while soil temp ({:.1}°F) is still below {:.0}°F. \
             This can weaken the lawn heading into summer.",
            soil_temp, SPRING_N_MIN_SOIL_F
        ),
    )
    .with_explanation(
        "Applying nitrogen before the lawn is ready forces top (blade) growth while roots \
         are still dormant. This depletes the plant's carbohydrate reserves and creates \
         a shallow root system. The result: a lawn that looks good briefly in spring \
         but struggles in summer heat. The grass needs to wake up naturally first.",
    )
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Target Temp",
        format!("{:.0}°F minimum", SPRING_N_MIN_SOIL_F),
        DataSource::Agronomic.as_str(),
    )
    .with_action(format!(
        "Avoid additional nitrogen applications until soil consistently reaches {:.0}°F. \
         Focus on other spring tasks: clean up debris, sharpen mower blades, \
         check irrigation system. The pre-emergent window comes before fertilization.",
        SPRING_N_MIN_SOIL_F
    ))
}

fn build_patience_advisory(soil_temp: f64) -> Recommendation {
    Recommendation::new(
        "spring_n_wait",
        RecommendationCategory::Fertilizer,
        Severity::Info,
        "Spring Fertilizer - Wait for Warmer Soil",
        format!(
            "Soil temperature ({:.1}°F) is still too cold for spring fertilization. \
             Patience now pays off with a stronger lawn later.",
            soil_temp
        ),
    )
    .with_explanation(format!(
        "It's tempting to fertilize as soon as you see green, but cool-season grass \
         breaks dormancy from stored carbohydrates - not from soil nutrients. \
         Wait until soil reaches {:.0}°F and you've mowed 2-3 times. This ensures \
         roots are active and ready to absorb nutrients. Early nitrogen forces \
         weak top growth at the expense of root development.",
        SPRING_N_MIN_SOIL_F
    ))
    .with_data_point(
        "Current Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Target Soil Temp",
        format!("{:.0}°F", SPRING_N_MIN_SOIL_F),
        DataSource::Agronomic.as_str(),
    )
    .with_action(
        "Focus on spring prep: rake leaves/debris, check for disease damage, \
         plan pre-emergent timing (that window comes first!). \
         First fertilization should wait until after 2-3 mowings.",
    )
}

fn build_almost_ready(soil_temp: f64, gdd_ytd: Option<f64>) -> Recommendation {
    let mut rec = Recommendation::new(
        "spring_n_almost",
        RecommendationCategory::Fertilizer,
        Severity::Info,
        "Spring Fertilizer - Almost Time",
        format!(
            "Soil temperature ({:.1}°F) is approaching the {:.0}°F threshold. \
             Spring fertilization window opening soon.",
            soil_temp, SPRING_N_MIN_SOIL_F
        ),
    )
    .with_explanation(format!(
        "You're close to the right conditions for spring nitrogen. Wait for soil \
         to consistently reach {:.0}°F and ensure you've completed 2-3 mowing cycles. \
         This confirms the grass is actively growing and roots are ready for nutrients. \
         Remember: pre-emergent timing ({:.0}-{:.0}°F soil) comes before fertilization.",
        SPRING_N_MIN_SOIL_F, PRE_EMERGENT_SOIL_LOW_F, PRE_EMERGENT_URGENCY_SOIL_F
    ))
    .with_data_point(
        "Current Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Target",
        format!("{:.0}°F + 2-3 mowings", SPRING_N_MIN_SOIL_F),
        DataSource::Agronomic.as_str(),
    );

    if let Some(gdd) = gdd_ytd {
        rec = rec.with_data_point(
            "GDD (Base 50°F YTD)",
            format!("{:.0}", gdd),
            DataSource::Calculated.as_str(),
        );
    }

    rec = rec.with_action(
        "Continue monitoring soil temperature. Start mowing when grass needs it. \
         After your second or third mowing AND soil is 55°F+, apply light spring nitrogen. \
         Don't rush - a week or two of patience makes a stronger summer lawn.",
    );

    rec
}

fn build_may_cutoff_warning(soil_temp: f64) -> Recommendation {
    Recommendation::new(
        "spring_n_may_cutoff",
        RecommendationCategory::Fertilizer,
        Severity::Info,
        "Spring Nitrogen — Last Chance (May Cutoff)",
        format!(
            "Soil temperature ({:.1}°F). This is the absolute last chance for spring nitrogen. \
             Many extension programs recommend skipping spring N entirely.",
            soil_temp
        ),
    )
    .with_explanation(
        "Missouri Extension g6705 states: 'Do not apply nitrogen, particularly quickly \
         available soluble forms, past May 1.' Nitrogen applied this late pushes top growth \
         heading into summer heat stress, weakening the plant. Many extension programs now \
         recommend skipping spring N entirely and focusing all feeding in fall (September-November). \
         If you must apply, use slow-release only at ≤0.5 lb N/1000sqft.",
    )
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Deadline",
        "May 1 (Missouri Extension)",
        DataSource::Agronomic.as_str(),
    )
    .with_action(
        "If you haven't fertilized yet this spring, consider SKIPPING spring nitrogen \
         entirely. Save your feeding budget for September (the most important fertilization). \
         If you insist on applying, use only slow-release nitrogen at 0.5 lb N/1000sqft MAX. \
         Do not apply soluble/quick-release nitrogen past May 1.",
    )
}

fn build_ready_to_fertilize(
    soil_temp: f64,
    profile: &LawnProfile,
    gdd_ytd: Option<f64>,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);
    let n_needed = lawn_size / 1000.0 * SPRING_N_RATE_LBS_PER_KSQFT;

    let gdd_note = if let Some(gdd) = gdd_ytd {
        if gdd >= SPRING_N_GDD_ESTABLISHED {
            format!(
                " GDD ({:.0}) confirms growth is well established — good time to apply.",
                gdd
            )
        } else if gdd >= SPRING_N_GDD_READY {
            format!(" GDD ({:.0}) confirms active growth has begun.", gdd)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut rec = Recommendation::new(
        "spring_n_ready",
        RecommendationCategory::Fertilizer,
        Severity::Advisory,
        "Spring Fertilization Window Open",
        format!(
            "Soil temperature ({:.1}°F) indicates spring fertilization is appropriate. \
             If you've mowed 2-3 times, light nitrogen is now beneficial.",
            soil_temp
        ),
    )
    .with_explanation(format!(
        "With soil at {:.0}°F+, roots are active and can utilize applied nitrogen. \
         Spring feeding should be LIGHT compared to fall - cool-season grass does \
         most of its feeding in autumn. A light spring application ({:.1} lb N/1000 sqft) \
         supports spring growth without pushing excessive top growth that weakens the plant.{}",
        SPRING_N_MIN_SOIL_F, SPRING_N_RATE_LBS_PER_KSQFT, gdd_note
    ))
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Recommended Rate",
        format!("{:.1} lb N/1000 sqft", SPRING_N_RATE_LBS_PER_KSQFT),
        DataSource::Agronomic.as_str(),
    );

    if let Some(gdd) = gdd_ytd {
        rec = rec.with_data_point(
            "GDD (Base 50°F YTD)",
            format!("{:.0}", gdd),
            DataSource::Calculated.as_str(),
        );
    }

    rec = rec.with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn ({:.1} lb N/1000 sqft). \
         Use slow-release nitrogen to avoid surge growth. \
         This should be your ONLY spring nitrogen - save the heavy feeding for fall. \
         Verify you've mowed 2-3 times first to confirm grass is actively growing.",
        n_needed, lawn_size, SPRING_N_RATE_LBS_PER_KSQFT
    ));

    rec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvironmentalReading, GrassType};

    fn base_env(soil_avg: f64) -> EnvironmentalSummary {
        let reading = EnvironmentalReading::new(DataSource::SoilData);
        EnvironmentalSummary {
            current: Some(reading),
            soil_temp_7day_avg_f: Some(soil_avg),
            ..Default::default()
        }
    }

    fn base_profile() -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type: GrassType::TallFescue,
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn gdd_none_degrades_gracefully() {
        // GDD = None should not change behavior vs pre-GDD code.
        let env = base_env(52.0);
        assert!(env.gdd_base50_ytd.is_none());
        let rule = SpringNitrogenRule;
        // Month-gated: only produces recommendations Feb-May.
        let result = rule.evaluate(&env, &base_profile(), &[]);
        // Should not panic regardless of date.
        if let Some(rec) = result {
            // In the 50-55 range with no GDD, should be "almost ready" (Info)
            assert_eq!(rec.severity, Severity::Info);
        }
    }

    #[test]
    fn gdd_below_threshold_no_promotion() {
        // GDD = 30 (below 50) in the 50-55°F range — should NOT promote to "ready".
        let mut env = base_env(52.0);
        env.gdd_base50_ytd = Some(30.0);
        let rule = SpringNitrogenRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        if let Some(rec) = result {
            // Should still be "almost ready" (Info), not promoted
            assert_eq!(rec.id, "spring_n_almost");
        }
    }

    #[test]
    fn gdd_at_ready_promotes_almost_to_ready() {
        // GDD = 50 in the 50-55°F range — should promote "almost ready" to "ready".
        // Note: This is month-gated (Feb-May), so result depends on current date.
        let mut env = base_env(52.0);
        env.gdd_base50_ytd = Some(50.0);
        let rule = SpringNitrogenRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        if let Some(rec) = result {
            // With GDD >= 50, should be promoted to spring_n_ready
            assert_eq!(rec.id, "spring_n_ready");
            assert_eq!(rec.severity, Severity::Advisory);
        }
    }

    #[test]
    fn gdd_established_adds_note_in_ready_range() {
        // GDD = 150 in the 55-65°F range — should include GDD note.
        let mut env = base_env(58.0);
        env.gdd_base50_ytd = Some(150.0);
        let rule = SpringNitrogenRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        if let Some(rec) = result {
            assert_eq!(rec.id, "spring_n_ready");
            // Should include GDD data point
            assert!(
                rec.data_points.iter().any(|dp| dp.label.contains("GDD")),
                "Should include GDD data point"
            );
        }
    }
}
