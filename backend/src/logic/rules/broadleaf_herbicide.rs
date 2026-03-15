use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Broadleaf herbicide timing rule
///
/// Post-emergent weed control windows (Missouri Extension g6705):
/// - Spring window (March): Soil temp 45-55°F rising → target winter annuals
/// - Fall window (late Sept - Oct): Soil temp 50-65°F falling → best time for
///   perennial broadleaf control (dandelion, clover, plantain)
///
/// Blocked if overseeded within 60 days (herbicide kills seedlings).
pub struct BroadleafHerbicideRule;

impl Rule for BroadleafHerbicideRule {
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
        let month = today.month();

        // Check if overseeded within 60 days — suppress recommendation
        let recent_overseed = history.iter().any(|app| {
            app.application_type == ApplicationType::Overseed
                && (today - app.application_date).num_days() <= HERBICIDE_OVERSEED_BUFFER_DAYS
        });

        if recent_overseed {
            return None;
        }

        // Check if already applied post-emergent this season
        let current_year = today.year();
        let already_applied_spring = history.iter().any(|app| {
            app.application_type == ApplicationType::PostEmergent
                && app.application_date.year() == current_year
                && app.application_date.month() <= 4
        });

        let already_applied_fall = history.iter().any(|app| {
            app.application_type == ApplicationType::PostEmergent
                && app.application_date.year() == current_year
                && app.application_date.month() >= 9
        });

        let soil_temp_avg = env.soil_temp_7day_avg_f?;

        // Spring window: March, soil temp 45-55°F rising
        if month == 3
            && !already_applied_spring
            && (SPRING_HERBICIDE_SOIL_LOW_F..=SPRING_HERBICIDE_SOIL_HIGH_F).contains(&soil_temp_avg)
            && env.soil_temp_trend.is_rising()
        {
            return Some(build_spring_herbicide_rec(soil_temp_avg));
        }

        // Fall window: late Sept - Oct, soil temp 50-65°F falling
        let fall_start = NaiveDate::from_ymd_opt(current_year, 9, 20)?;
        let fall_end = NaiveDate::from_ymd_opt(current_year, 10, 31)?;

        if today >= fall_start
            && today <= fall_end
            && !already_applied_fall
            && (FALL_HERBICIDE_SOIL_LOW_F..=FALL_HERBICIDE_SOIL_HIGH_F).contains(&soil_temp_avg)
        {
            return Some(build_fall_herbicide_rec(soil_temp_avg));
        }

        None
    }
}

fn build_spring_herbicide_rec(soil_temp: f64) -> Recommendation {
    Recommendation::new(
        "broadleaf_spring",
        RecommendationCategory::Herbicide,
        Severity::Advisory,
        "Spring Broadleaf Herbicide Window",
        format!(
            "Soil temperature ({:.1}°F) is in the spring post-emergent window. \
             Good time to target winter annual weeds.",
            soil_temp
        ),
    )
    .with_explanation(
        "Spring is the time to target winter annual broadleaf weeds (henbit, chickweed, \
         deadnettle) that germinated in fall. Soil temps of 45-55°F and rising indicate \
         weeds are actively growing and susceptible to herbicide. Best results when air \
         temps are 50-80°F and no rain expected for 24 hours.",
    )
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Target Weeds",
        "Winter annuals",
        DataSource::MissouriExtension.as_str(),
    )
    .with_action(
        "Apply post-emergent broadleaf herbicide (2,4-D + dicamba or triclopyr). \
         Apply when air temp is 50-80°F and no rain expected for 24 hours. \
         Avoid application if wind >10 mph. Do not mow for 24-48 hours after application.",
    )
}

fn build_fall_herbicide_rec(soil_temp: f64) -> Recommendation {
    Recommendation::new(
        "broadleaf_fall",
        RecommendationCategory::Herbicide,
        Severity::Warning, // Fall is the BEST time
        "Fall Broadleaf Herbicide — Best Window",
        format!(
            "Soil temperature ({:.1}°F) is ideal for fall broadleaf weed control. \
             This is the most effective time to treat perennial broadleaf weeds.",
            soil_temp
        ),
    )
    .with_explanation(
        "Late September through October is THE best time for perennial broadleaf weed \
         control (Missouri Extension g6705). Weeds like dandelion, clover, and plantain \
         are actively translocating nutrients to roots for winter — herbicide applied now \
         follows the same path, killing the entire plant including roots. Spring applications \
         are less effective because weeds are pushing energy outward to leaves.",
    )
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Target Weeds",
        "Perennial broadleaf (dandelion, clover, plantain)",
        DataSource::MissouriExtension.as_str(),
    )
    .with_action(
        "Apply post-emergent broadleaf herbicide (2,4-D + dicamba or triclopyr). \
         Fall application is more effective than spring for perennial weeds. \
         Apply when air temp is 50-80°F, no rain for 24 hours, wind <10 mph. \
         Do not mow for 24-48 hours after application.",
    )
}
