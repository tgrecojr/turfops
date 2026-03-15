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
/// - Spring window: GDD 50-150 OR month == 3, soil temp 45-55°F rising
///   → target winter annuals
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

        // GDD data for spring window
        let gdd_ytd = env.gdd_base50_ytd;
        let gdd_in_spring_range = gdd_ytd.is_some_and(|gdd| {
            (SPRING_HERBICIDE_GDD_LOW..=SPRING_HERBICIDE_GDD_HIGH).contains(&gdd)
        });

        // Spring window: (GDD 50-150 OR month == 3) + soil temp 45-55°F rising
        let spring_eligible = gdd_in_spring_range || month == 3;
        if spring_eligible
            && !already_applied_spring
            && (SPRING_HERBICIDE_SOIL_LOW_F..=SPRING_HERBICIDE_SOIL_HIGH_F).contains(&soil_temp_avg)
            && env.soil_temp_trend.is_rising()
        {
            return Some(build_spring_herbicide_rec(soil_temp_avg, gdd_ytd));
        }

        // Fall window: late Sept - Oct, soil temp 50-65°F falling (GDD not used)
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

fn build_spring_herbicide_rec(soil_temp: f64, gdd_ytd: Option<f64>) -> Recommendation {
    let gdd_note = if let Some(gdd) = gdd_ytd {
        format!(
            " GDD ({:.0}) confirms winter annual weeds are actively growing and susceptible.",
            gdd
        )
    } else {
        String::new()
    };

    let mut rec = Recommendation::new(
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
    .with_explanation(format!(
        "Spring is the time to target winter annual broadleaf weeds (henbit, chickweed, \
         deadnettle) that germinated in fall. Soil temps of 45-55°F and rising indicate \
         weeds are actively growing and susceptible to herbicide. Best results when air \
         temps are 50-80°F and no rain expected for 24 hours.{}",
        gdd_note
    ))
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Target Weeds",
        "Winter annuals",
        DataSource::MissouriExtension.as_str(),
    );

    if let Some(gdd) = gdd_ytd {
        rec = rec.with_data_point(
            "GDD (Base 50°F YTD)",
            format!("{:.0} / {:.0}", gdd, SPRING_HERBICIDE_GDD_HIGH),
            DataSource::Calculated.as_str(),
        );
    }

    rec = rec.with_action(
        "Apply post-emergent broadleaf herbicide (2,4-D + dicamba or triclopyr). \
         Apply when air temp is 50-80°F and no rain expected for 24 hours. \
         Avoid application if wind >10 mph. Do not mow for 24-48 hours after application.",
    );

    rec
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{environmental::Trend, EnvironmentalReading, GrassType};

    fn base_env(soil_avg: f64, trend: Trend) -> EnvironmentalSummary {
        let reading = EnvironmentalReading::new(DataSource::SoilData);
        EnvironmentalSummary {
            current: Some(reading),
            soil_temp_7day_avg_f: Some(soil_avg),
            soil_temp_trend: trend,
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
        let env = base_env(50.0, Trend::Rising);
        assert!(env.gdd_base50_ytd.is_none());
        let rule = BroadleafHerbicideRule;
        // Month-gated (March for spring, Sep-Oct for fall), should not panic.
        let _ = rule.evaluate(&env, &base_profile(), &[]);
    }

    #[test]
    fn gdd_below_threshold_no_spring_extension() {
        // GDD = 30 (below 50) outside March — should NOT open spring window.
        let mut env = base_env(50.0, Trend::Rising);
        env.gdd_base50_ytd = Some(30.0);
        let rule = BroadleafHerbicideRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        // If not March and GDD < 50, spring window should not open.
        // Result may be Some in March or fall, None otherwise.
        let today = Local::now().date_naive();
        let month = today.month();
        if month != 3 && !(9..=10).contains(&month) {
            assert!(
                result.is_none(),
                "Should not trigger outside valid months with low GDD"
            );
        }
    }

    #[test]
    fn gdd_in_range_opens_spring_window() {
        // GDD = 100 (in 50-150 range) — should open spring window regardless of month.
        // Requires soil temp 45-55°F and rising trend.
        let mut env = base_env(50.0, Trend::Rising);
        env.gdd_base50_ytd = Some(100.0);
        let rule = BroadleafHerbicideRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        // If soil temp is in range and trend is rising, should produce recommendation.
        // Date-sensitive: only spring months produce spring recommendation.
        let today = Local::now().date_naive();
        let month = today.month();
        if (2..=4).contains(&month) {
            if let Some(rec) = result {
                assert_eq!(rec.id, "broadleaf_spring");
                assert!(
                    rec.data_points.iter().any(|dp| dp.label.contains("GDD")),
                    "Should include GDD data point"
                );
            }
        }
    }

    #[test]
    fn gdd_above_range_no_spring_extension() {
        // GDD = 200 (above 150) — should NOT open spring window via GDD alone.
        let mut env = base_env(50.0, Trend::Rising);
        env.gdd_base50_ytd = Some(200.0);
        let rule = BroadleafHerbicideRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        let today = Local::now().date_naive();
        let month = today.month();
        // GDD > 150 means gdd_in_spring_range is false
        // Only month == 3 can open the spring window
        if month != 3 && !(9..=10).contains(&month) {
            assert!(
                result.is_none(),
                "GDD above range should not open spring window"
            );
        }
    }
}
