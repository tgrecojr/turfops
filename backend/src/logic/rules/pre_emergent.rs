use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
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

        // GDD-enhanced urgency: if GDD data is available, escalate based on crabgrass model
        let gdd_ytd = env.gdd_base50_ytd;
        let gdd_urgency = gdd_ytd.map(|gdd| {
            if gdd >= 200.0 {
                3 // Post-germination
            } else if gdd >= 150.0 {
                2 // Germination likely
            } else if gdd >= 75.0 {
                1 // Approaching
            } else {
                0 // Pre-germination
            }
        });

        if (PRE_EMERGENT_SOIL_LOW_F..=PRE_EMERGENT_SOIL_HIGH_F).contains(&soil_temp_avg) {
            // Optimal window — escalate severity if GDD indicates urgency
            let severity =
                if gdd_urgency.unwrap_or(0) >= 2 || soil_temp_avg >= PRE_EMERGENT_URGENCY_SOIL_F {
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

            let gdd_explanation = if let Some(gdd) = gdd_ytd {
                format!(
                    " Year-to-date GDD (base 50°F): {:.0}. Crabgrass germinates at ~200 GDD.",
                    gdd
                )
            } else {
                String::new()
            };

            rec = rec
                .with_explanation(format!(
                    "Crabgrass germinates when soil temperature at 2-4 inch depth reaches 55°F \
                     for 3+ consecutive days. Apply pre-emergent (prodiamine or dithiopyr) \
                     before germination begins for best results.{}",
                    gdd_explanation
                ))
                .with_data_point(
                    "7-Day Avg Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    DataSource::SoilData.as_str(),
                )
                .with_data_point(
                    "Current Soil Temp (10cm)",
                    format!("{:.1}°F", current_soil_temp),
                    DataSource::SoilData.as_str(),
                )
                .with_data_point(
                    "Trend",
                    env.soil_temp_trend.as_str(),
                    DataSource::Calculated.as_str(),
                );

            if let Some(gdd) = gdd_ytd {
                rec = rec.with_data_point(
                    "GDD (Base 50°F YTD)",
                    format!("{:.0} / 200", gdd),
                    DataSource::Calculated.as_str(),
                );
            }

            rec = rec.with_action(
                "Apply pre-emergent herbicide (prodiamine, dithiopyr, or pendimethalin) \
                     at label rate. Water in within 24 hours if no rain.",
            );

            Some(rec)
        } else if soil_temp_avg > PRE_EMERGENT_SOIL_HIGH_F
            && soil_temp_avg <= PRE_EMERGENT_LATE_SOIL_F
        {
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
            .with_explanation(format!(
                "If pre-emergent hasn't been applied, do so immediately. Consider a split \
                 application or use a product with post-emergent properties. After {:.0}°F soil \
                 temp, pre-emergent efficacy drops significantly.",
                PRE_EMERGENT_LATE_SOIL_F
            ))
            .with_data_point(
                "7-Day Avg Soil Temp",
                format!("{:.1}°F", soil_temp_avg),
                DataSource::SoilData.as_str(),
            )
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
