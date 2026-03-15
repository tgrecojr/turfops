use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity, SoilType,
};
use chrono::{Datelike, Local, NaiveDate};

/// Core aeration timing rule
///
/// Best timing (Missouri Extension g6705):
/// - Window: August 15 - October 15
/// - Soil temp 50-65°F
/// - At least annually where compaction exists
/// - Best paired with fall overseeding
///
/// Clay/ClayLoam soils get elevated severity (more prone to compaction).
pub struct AerationRule;

impl Rule for AerationRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        if !profile.grass_type.is_cool_season() {
            return None;
        }

        let today = Local::now().date_naive();
        let current_year = today.year();

        // Window: August 15 - October 15
        let window_start = NaiveDate::from_ymd_opt(current_year, 8, 15)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 10, 15)?;

        if today < window_start || today > window_end {
            return None;
        }

        // Check if already aerated this year
        let already_aerated = history.iter().any(|app| {
            app.application_type == ApplicationType::Aeration
                && app.application_date.year() == current_year
        });

        if already_aerated {
            return None;
        }

        // Check soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;
        if !(AERATION_SOIL_LOW_F..=AERATION_SOIL_HIGH_F).contains(&soil_temp_avg) {
            return None;
        }

        // Check if aerated in past 12 months
        let one_year_ago = today - chrono::Duration::days(365);
        let aerated_recently = history.iter().any(|app| {
            app.application_type == ApplicationType::Aeration
                && app.application_date >= one_year_ago
        });

        // Clay/ClayLoam soils get elevated severity
        let is_heavy_soil = profile
            .soil_type
            .map(|s| matches!(s, SoilType::Clay | SoilType::ClayLoam))
            .unwrap_or(false);

        let severity = if is_heavy_soil && !aerated_recently {
            Severity::Warning
        } else if !aerated_recently {
            Severity::Advisory
        } else {
            Severity::Info
        };

        // Check if overseeding is also due (pair recommendation)
        let needs_overseed = !history.iter().any(|app| {
            app.application_type == ApplicationType::Overseed
                && app.application_date.year() == current_year
                && app.application_date >= window_start
        });

        let overseed_note = if needs_overseed {
            " Pair aeration with overseeding for best results — aeration creates ideal \
             seed-to-soil contact."
        } else {
            ""
        };

        let soil_note = if is_heavy_soil {
            format!(
                " Your {} soil is prone to compaction — annual aeration is especially important.",
                profile.soil_type.map(|s| s.as_str()).unwrap_or("heavy")
            )
        } else {
            String::new()
        };

        let rec = Recommendation::new(
            "aeration",
            RecommendationCategory::Aeration,
            severity,
            "Core Aeration Window",
            format!(
                "Soil temperature ({:.1}°F) is ideal for core aeration.{}{}",
                soil_temp_avg, soil_note, overseed_note
            ),
        )
        .with_explanation(
            "Core aeration relieves soil compaction, improves water/nutrient penetration, \
             and promotes root growth (Missouri Extension g6705). Early fall (August 15 - \
             October 15) is the best time for cool-season grasses because the grass can \
             recover quickly during its peak growth period. Aerate at least annually where \
             compaction exists. Clay and clay loam soils benefit the most.",
        )
        .with_data_point(
            "Soil Temp",
            format!("{:.1}°F", soil_temp_avg),
            DataSource::SoilData.as_str(),
        )
        .with_data_point(
            "Last Aeration",
            if aerated_recently {
                "Within 12 months"
            } else {
                "Over 12 months ago (or never)"
            },
            DataSource::History.as_str(),
        )
        .with_action(format!(
            "Core aerate when soil is moist (not wet or dry). Make 2-3 passes with a core \
             aerator, pulling 2-3 inch plugs. Leave plugs on the surface to break down. \
             Water lightly after aeration.{}",
            overseed_note
        ));

        Some(rec)
    }
}
