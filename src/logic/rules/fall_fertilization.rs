use super::Rule;
use crate::models::{
    Application, ApplicationType, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
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
    fn id(&self) -> &'static str {
        "fall_fertilization"
    }

    fn name(&self) -> &'static str {
        "Fall Fertilization Program"
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

        // Count fall fertilizer applications this year
        let fall_apps: Vec<&Application> = history
            .iter()
            .filter(|app| {
                app.application_type == ApplicationType::Fertilizer
                    && app.application_date.year() == current_year
                    && app.application_date >= window_start
            })
            .collect();

        let app_count = fall_apps.len();

        // Find most recent fall application
        let last_app_date = fall_apps.iter().map(|a| a.application_date).max();
        let days_since_last = last_app_date.map(|d| (today - d).num_days()).unwrap_or(999);

        // Determine which phase of fall fertilization we're in
        let phase = determine_fall_phase(today, current_year);

        // Check if soil temp is appropriate
        let soil_temp_ok = soil_temp_avg >= 45.0 && soil_temp_avg <= 65.0;

        // Generate recommendation based on phase and history
        match phase {
            FallPhase::Early => {
                // September - recovery feeding
                if app_count == 0 && soil_temp_ok {
                    Some(build_early_fall_rec(soil_temp_avg, profile, env))
                } else {
                    None
                }
            }
            FallPhase::Mid => {
                // October - main fall feeding
                if app_count < 2 && days_since_last >= 21 && soil_temp_ok {
                    Some(build_mid_fall_rec(soil_temp_avg, app_count, profile, env))
                } else {
                    None
                }
            }
            FallPhase::Late => {
                // November - winterizer
                if app_count < 3 && days_since_last >= 21 && soil_temp_avg >= 40.0 {
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

fn build_early_fall_rec(
    soil_temp: f64,
    profile: &LawnProfile,
    env: &EnvironmentalSummary,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(5000.0);
    let n_needed = lawn_size / 1000.0 * 0.5; // 0.5 lb N per 1000 sqft

    let mut rec = Recommendation::new(
        "fall_fert_early",
        RecommendationCategory::Fertilizer,
        Severity::Advisory,
        "Early Fall Fertilization",
        format!(
            "Soil temperature ({:.1}°F) is ideal for fall fertilization. \
             Time to begin fall nitrogen program.",
            soil_temp
        ),
    )
    .with_explanation(
        "Early fall feeding helps TTTF recover from summer stress. Apply light nitrogen \
         (0.5 lb N per 1000 sqft) to support recovery without pushing excessive top growth. \
         This sets up the lawn for the critical mid-fall and winterizer applications.",
    )
    .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp), "NOAA USCRN")
    .with_data_point("Phase", "Early Fall (Recovery)", "Calendar");

    if let Some(trend) = Some(&env.soil_temp_trend) {
        rec = rec.with_data_point("Trend", trend.as_str(), "Calculated");
    }

    rec = rec.with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn (0.5 lb N/1000 sqft). \
         Use a balanced fertilizer or slow-release nitrogen. \
         Water in lightly if no rain expected.",
        n_needed, lawn_size
    ));

    rec
}

fn build_mid_fall_rec(
    soil_temp: f64,
    app_count: usize,
    profile: &LawnProfile,
    env: &EnvironmentalSummary,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(5000.0);
    let n_needed = lawn_size / 1000.0 * 0.75; // 0.75 lb N per 1000 sqft

    let severity = if app_count == 0 {
        Severity::Warning // Missed early fall app
    } else {
        Severity::Advisory
    };

    let title = if app_count == 0 {
        "Mid-Fall Fertilization - Don't Miss Fall Feeding!"
    } else {
        "Mid-Fall Fertilization"
    };

    let mut rec = Recommendation::new(
        "fall_fert_mid",
        RecommendationCategory::Fertilizer,
        severity,
        title,
        format!(
            "Prime time for fall fertilization. Soil temp {:.1}°F is optimal \
             for root uptake and carbohydrate storage.",
            soil_temp
        ),
    )
    .with_explanation(
        "Mid-fall (October) is the MOST important fertilization of the year for TTTF. \
         Roots are actively growing while top growth slows. Nitrogen applied now is \
         stored as carbohydrates, fueling winter hardiness and explosive spring green-up. \
         This single application has more impact than any other feeding.",
    )
    .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp), "NOAA USCRN")
    .with_data_point("Phase", "Mid-Fall (Primary)", "Calendar")
    .with_data_point("Fall Apps So Far", format!("{}", app_count), "History");

    if let Some(trend) = Some(&env.soil_temp_trend) {
        rec = rec.with_data_point("Trend", trend.as_str(), "Calculated");
    }

    rec = rec.with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn (0.75 lb N/1000 sqft). \
         A slow-release or balanced fertilizer works well. \
         This is the most important feeding of the year - don't skip it!",
        n_needed, lawn_size
    ));

    rec
}

fn build_late_fall_rec(soil_temp: f64, app_count: usize, profile: &LawnProfile) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(5000.0);
    let n_needed = lawn_size / 1000.0 * 1.0; // 1.0 lb N per 1000 sqft for winterizer

    let severity = if app_count == 0 {
        Severity::Warning // Missed all fall apps - at least get winterizer
    } else {
        Severity::Advisory
    };

    Recommendation::new(
        "fall_fert_winterizer",
        RecommendationCategory::Fertilizer,
        severity,
        "Winterizer Application",
        format!(
            "Time for final fall fertilization. Soil temp {:.1}°F - grass is slowing \
             but roots are still active.",
            soil_temp
        ),
    )
    .with_explanation(
        "The 'winterizer' application provides nitrogen that the grass stores over winter. \
         Applied when growth has slowed but before the ground freezes, this nitrogen \
         is available immediately when spring arrives, giving you the fastest, greenest \
         spring lawn. Apply even if grass appears dormant - roots are still working.",
    )
    .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp), "NOAA USCRN")
    .with_data_point("Phase", "Late Fall (Winterizer)", "Calendar")
    .with_data_point("Fall Apps So Far", format!("{}", app_count), "History")
    .with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn (1.0 lb N/1000 sqft). \
         Quick-release nitrogen is fine for winterizer since you want immediate uptake. \
         Apply before ground freezes, even if grass looks dormant.",
        n_needed, lawn_size
    ))
}
