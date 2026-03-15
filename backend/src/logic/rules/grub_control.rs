use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Grub control timing rule
///
/// Japanese beetle and other grub larvae are most vulnerable to
/// preventative treatments when actively feeding near the soil surface.
///
/// Window: May 15 - July 4, soil temp 60-75°F
/// Product: Chlorantraniliprole (GrubEx), Imidacloprid, or similar
pub struct GrubControlRule;

impl Rule for GrubControlRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        _profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        let today = Local::now().date_naive();
        let current_year = today.year();

        // Define the application window
        let window_start = NaiveDate::from_ymd_opt(current_year, 5, 15)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 7, 4)?;

        // Only relevant during the window
        if today < window_start || today > window_end {
            return None;
        }

        // Check if already applied this year
        let already_applied = history.iter().any(|app| {
            (app.application_type == ApplicationType::GrubControl
                || app.application_type == ApplicationType::Insecticide)
                && app.application_date.year() == current_year
                && app.application_date >= window_start
        });

        if already_applied {
            return None;
        }

        // Get soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;
        let current_soil_temp = env.current.as_ref()?.soil_temp_10_f?;

        if (GRUB_CONTROL_SOIL_LOW_F..=GRUB_CONTROL_SOIL_HIGH_F).contains(&soil_temp_avg) {
            // Calculate days remaining in window
            let days_remaining = (window_end - today).num_days();

            let severity = if days_remaining <= GRUB_URGENCY_DAYS {
                Severity::Warning
            } else {
                Severity::Advisory
            };

            let rec = Recommendation::new(
                format!("grub_control_{}", current_year),
                RecommendationCategory::GrubControl,
                severity,
                "Grub Preventative Window",
                format!(
                    "Conditions are optimal for preventative grub control application. \
                     {} days remaining in window.",
                    days_remaining
                ),
            )
            .with_explanation(
                "Japanese beetle larvae (grubs) are most vulnerable to preventative treatments \
                 when adults are laying eggs and larvae are feeding near the surface. \
                 Chlorantraniliprole (GrubEx) provides season-long control when applied now. \
                 Treatment is justified when you find 5-10+ grubs per square foot by lifting \
                 a 1-sqft section of turf (Missouri Extension g6705).",
            )
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
                "Window Closes",
                window_end.format("%B %d").to_string(),
                DataSource::Agronomic.as_str(),
            )
            .with_action(
                "Apply chlorantraniliprole (GrubEx) or imidacloprid at label rate. \
                 Water in with 0.5\" of irrigation or rain within 24 hours.",
            );

            Some(rec)
        } else if soil_temp_avg > GRUB_CONTROL_SOIL_HIGH_F {
            // Soil may be too warm - grubs may be deeper
            let rec = Recommendation::new(
                format!("grub_control_late_{}", current_year),
                RecommendationCategory::GrubControl,
                Severity::Info,
                "Grub Control - Soil Warm",
                "Soil temperature is elevated. Grub control may still be effective but optimal window is passing.",
            )
            .with_data_point(
                "7-Day Avg Soil Temp",
                format!("{:.1}°F", soil_temp_avg),
                DataSource::SoilData.as_str(),
            )
            .with_action(
                "If grub control hasn't been applied, do so soon. \
                 Effectiveness decreases as larvae move deeper into soil.",
            );

            Some(rec)
        } else {
            None
        }
    }
}
