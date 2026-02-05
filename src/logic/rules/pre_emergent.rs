use super::Rule;
use crate::models::{
    Application, ApplicationType, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local};

/// Pre-emergent herbicide timing rule
///
/// Crabgrass germinates when soil temperature at 2-4 inches depth
/// reaches 55°F for 3+ consecutive days. Pre-emergent should be
/// applied before this threshold is reached.
///
/// Window: Soil temp 50-60°F (7-day average at 10cm depth)
pub struct PreEmergentRule;

impl Rule for PreEmergentRule {
    fn id(&self) -> &'static str {
        "pre_emergent"
    }

    fn name(&self) -> &'static str {
        "Pre-Emergent Timing"
    }

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

        // Only relevant in spring (Feb-May)
        let month = Local::now().month();
        if !(2..=5).contains(&month) {
            return None;
        }

        // Check if already applied this year
        let current_year = Local::now().year();
        let already_applied = history.iter().any(|app| {
            app.application_type == ApplicationType::PreEmergent
                && app.application_date.year() == current_year
        });

        if already_applied {
            return None;
        }

        // Get 7-day soil temp average
        let soil_temp_avg = env.soil_temp_7day_avg_f?;

        // Current soil temp for display
        let current_soil_temp = env.current.as_ref()?.soil_temp_10_f?;

        if soil_temp_avg >= 50.0 && soil_temp_avg <= 60.0 {
            // Optimal window
            let severity = if soil_temp_avg >= 55.0 {
                Severity::Warning
            } else {
                Severity::Advisory
            };

            let mut rec = Recommendation::new(
                format!("pre_emergent_{}", current_year),
                RecommendationCategory::PreEmergent,
                severity,
                "Pre-Emergent Application Window",
                format!(
                    "Soil temperature is in the optimal range for pre-emergent application. \
                     7-day average: {:.1}°F",
                    soil_temp_avg
                ),
            );

            rec = rec
                .with_explanation(
                    "Crabgrass germinates when soil temperature at 2-4 inch depth reaches 55°F \
                     for 3+ consecutive days. Apply pre-emergent (prodiamine or dithiopyr) \
                     before germination begins for best results.",
                )
                .with_data_point(
                    "7-Day Avg Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    "NOAA USCRN",
                )
                .with_data_point(
                    "Current Soil Temp (10cm)",
                    format!("{:.1}°F", current_soil_temp),
                    "NOAA USCRN",
                )
                .with_data_point("Trend", env.soil_temp_trend.as_str(), "Calculated")
                .with_action(
                    "Apply pre-emergent herbicide (prodiamine, dithiopyr, or pendimethalin) \
                     at label rate. Water in within 24 hours if no rain.",
                );

            Some(rec)
        } else if soil_temp_avg > 60.0 && soil_temp_avg <= 70.0 {
            // Late window - urgent
            let rec = Recommendation::new(
                format!("pre_emergent_late_{}", current_year),
                RecommendationCategory::PreEmergent,
                Severity::Critical,
                "Pre-Emergent Window Closing",
                format!(
                    "Soil temperature is above optimal range. Crabgrass may have begun germinating. \
                     7-day average: {:.1}°F",
                    soil_temp_avg
                ),
            )
            .with_explanation(
                "If pre-emergent hasn't been applied, do so immediately. Consider a split \
                 application or use a product with post-emergent properties. After 70°F soil \
                 temp, pre-emergent efficacy drops significantly.",
            )
            .with_data_point("7-Day Avg Soil Temp", format!("{:.1}°F", soil_temp_avg), "NOAA USCRN")
            .with_action(
                "Apply pre-emergent immediately if not yet done. Consider products with \
                 post-emergent activity like quinclorac combinations.",
            );

            Some(rec)
        } else {
            None
        }
    }
}
