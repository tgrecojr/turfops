mod recs;

use self::recs::{build_early_fall_rec, build_late_fall_rec, build_mid_fall_rec};
use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, EnvironmentalSummary, LawnProfile, Recommendation,
};
use chrono::{Datelike, Local, NaiveDate};

/// Fall fertilization program rule
///
/// Fall is THE most important fertilization window for cool-season grass.
/// While top growth slows, roots are actively growing and storing carbohydrates
/// for winter survival and spring green-up.
///
/// Program:
/// - Early Fall (Sept): Recovery feeding after summer stress
/// - Mid Fall (Oct): Main fall feeding for root development
/// - Late Fall (Nov): "Winterizer" before dormancy
///
/// Optimal conditions: Soil temp 50-60°F, grass still green
pub struct FallFertilizationRule;

impl Rule for FallFertilizationRule {
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

        // Define fall fertilization window (Sept 1 - Nov 30)
        let window_start = NaiveDate::from_ymd_opt(current_year, 9, 1)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 11, 30)?;

        // Only evaluate during the window
        if today < window_start || today > window_end {
            return None;
        }

        // Get soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;

        // A feeding is a lawn fertilizer that carried nitrogen (or whose analysis wasn't
        // recorded) — not shrub fertilizer, and not 0-0-50 potash.
        let feedings: Vec<&Application> = history
            .iter()
            .filter(|app| {
                app.application_type == ApplicationType::Fertilizer
                    && app.is_turf()
                    && app.nitrogen_pct.is_none_or(|n| n > 0.0)
                    && app.application_date.year() == current_year
                    && app.application_date <= today
            })
            .collect();

        // Count fall feedings this year
        let app_count = feedings
            .iter()
            .filter(|a| a.application_date >= window_start)
            .count();

        // Most recent feeding of any season: an Aug 30 application still spaces Sept's.
        let last_app_date = feedings.iter().map(|a| a.application_date).max();
        let days_since_last = last_app_date.map(|d| (today - d).num_days()).unwrap_or(999);

        // Determine which phase of fall fertilization we're in
        let phase = determine_fall_phase(today, current_year);

        // Check if soil temp is appropriate
        let soil_temp_ok = (FALL_FERT_SOIL_LOW_F..=FALL_FERT_SOIL_HIGH_F).contains(&soil_temp_avg);

        // Generate recommendation based on phase and history
        match phase {
            FallPhase::Early => {
                // September - recovery feeding
                if app_count == 0 && days_since_last >= FALL_FERT_MIN_INTERVAL_DAYS && soil_temp_ok
                {
                    Some(build_early_fall_rec(soil_temp_avg, profile, env))
                } else {
                    None
                }
            }
            FallPhase::Mid => {
                // October - main fall feeding
                if app_count < 2 && days_since_last >= FALL_FERT_MIN_INTERVAL_DAYS && soil_temp_ok {
                    Some(build_mid_fall_rec(soil_temp_avg, app_count, profile, env))
                } else {
                    None
                }
            }
            FallPhase::Late => {
                // November - winterizer
                if app_count < 3
                    && days_since_last >= FALL_FERT_MIN_INTERVAL_DAYS
                    && soil_temp_avg >= WINTERIZER_MIN_SOIL_F
                {
                    Some(build_late_fall_rec(soil_temp_avg, app_count, profile))
                } else {
                    None
                }
            }
            FallPhase::TooLate => None,
        }
    }
}

#[derive(Debug)]
enum FallPhase {
    Early, // Sept 1 - Sept 30
    Mid,   // Oct 1 - Oct 31
    Late,  // Nov 1 - Nov 30
    TooLate,
}

fn determine_fall_phase(today: NaiveDate, year: i32) -> FallPhase {
    let oct_1 = NaiveDate::from_ymd_opt(year, 10, 1).unwrap();
    let nov_1 = NaiveDate::from_ymd_opt(year, 11, 1).unwrap();
    let dec_1 = NaiveDate::from_ymd_opt(year, 12, 1).unwrap();

    if today < oct_1 {
        FallPhase::Early
    } else if today < nov_1 {
        FallPhase::Mid
    } else if today < dec_1 {
        FallPhase::Late
    } else {
        FallPhase::TooLate
    }
}
