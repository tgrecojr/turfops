use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};

/// Mowing height rule — seasonal mowing guidance for TTTF
///
/// Recommended heights (Missouri Extension g6705):
/// - Spring (50-75°F avg): 2-3.5"
/// - Summer (>75°F avg): 3-4" (taller to shade crown/soil)
/// - Fall (50-75°F avg): 2.5-3.5"
///
/// Key principle: Never remove more than 1/3 of the green leaf area at once.
pub struct MowingHeightRule;

impl Rule for MowingHeightRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        if !profile.grass_type.is_cool_season() {
            return None;
        }

        let ambient_avg = env.ambient_temp_7day_avg_f?;

        // Determine season based on temperature
        let (height_range, season, severity) = if ambient_avg > 85.0 {
            ("3.5-4 inches", "Summer (heat stress)", Severity::Warning)
        } else if ambient_avg > 75.0 {
            ("3-4 inches", "Summer", Severity::Advisory)
        } else if ambient_avg >= 50.0 {
            ("2.5-3.5 inches", "Spring/Fall", Severity::Info)
        } else {
            // Below 50°F — grass isn't actively growing
            return None;
        };

        let rec = Recommendation::new(
            "mowing_height",
            RecommendationCategory::Mowing,
            severity,
            format!("Mowing Height: {} ({})", height_range, season),
            format!(
                "With a 7-day average temp of {:.0}°F, recommended TTTF mowing height \
                 is {}.",
                ambient_avg, height_range
            ),
        )
        .with_explanation(
            "Tall Fescue mowing height should follow seasonal temperatures \
             (Missouri Extension g6705). Taller grass in summer shades the crown and soil, \
             reducing heat stress and water loss. Never remove more than 1/3 of the blade \
             at once. If raising height, do so gradually over 1-2 mowings. Maximum TTTF \
             summer height is 4 inches.",
        )
        .with_data_point(
            "7-Day Avg Temp",
            format!("{:.0}°F", ambient_avg),
            "Calculated",
        )
        .with_data_point(
            "Recommended Height",
            height_range,
            "Missouri Extension g6705",
        )
        .with_action(format!(
            "Set mowing height to {}. Never cut more than 1/3 of the blade at once. \
             If changing height, adjust gradually over 1-2 mowings.{}",
            height_range,
            if ambient_avg > 85.0 {
                " Consider skipping mowing during extreme heat — reduced mowing \
                 frequency reduces stress on the plant."
            } else {
                ""
            }
        ));

        Some(rec)
    }
}
