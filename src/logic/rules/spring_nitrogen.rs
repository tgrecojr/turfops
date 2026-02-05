use super::Rule;
use crate::models::{
    Application, ApplicationType, EnvironmentalSummary, LawnProfile, Recommendation,
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

        // Determine appropriate recommendation
        if soil_temp_avg < 50.0 {
            // Too cold - definitely don't fertilize
            if has_spring_fert {
                // Already fertilized when too cold - warn about the mistake
                Some(build_too_early_warning(soil_temp_avg))
            } else {
                // Good - they're waiting. Reinforce patience.
                Some(build_patience_advisory(soil_temp_avg))
            }
        } else if (50.0..55.0).contains(&soil_temp_avg) {
            // Getting close - still advise waiting
            if !has_spring_fert {
                Some(build_almost_ready(soil_temp_avg))
            } else {
                None // They already applied, no point warning now
            }
        } else if (55.0..=65.0).contains(&soil_temp_avg) {
            // Good range - if they haven't fertilized, now is okay
            if !has_spring_fert {
                Some(build_ready_to_fertilize(soil_temp_avg, profile))
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
            "Fertilizer was applied while soil temp ({:.1}°F) is still below 55°F. \
             This can weaken the lawn heading into summer.",
            soil_temp
        ),
    )
    .with_explanation(
        "Applying nitrogen before the lawn is ready forces top (blade) growth while roots \
         are still dormant. This depletes the plant's carbohydrate reserves and creates \
         a shallow root system. The result: a lawn that looks good briefly in spring \
         but struggles in summer heat. The grass needs to wake up naturally first.",
    )
    .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp), "NOAA USCRN")
    .with_data_point("Target Temp", "55°F minimum", "Agronomic")
    .with_action(
        "Avoid additional nitrogen applications until soil consistently reaches 55°F. \
         Focus on other spring tasks: clean up debris, sharpen mower blades, \
         check irrigation system. The pre-emergent window comes before fertilization.",
    )
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
    .with_explanation(
        "It's tempting to fertilize as soon as you see green, but cool-season grass \
         breaks dormancy from stored carbohydrates - not from soil nutrients. \
         Wait until soil reaches 55°F and you've mowed 2-3 times. This ensures \
         roots are active and ready to absorb nutrients. Early nitrogen forces \
         weak top growth at the expense of root development.",
    )
    .with_data_point(
        "Current Soil Temp",
        format!("{:.1}°F", soil_temp),
        "NOAA USCRN",
    )
    .with_data_point("Target Soil Temp", "55°F", "Agronomic")
    .with_action(
        "Focus on spring prep: rake leaves/debris, check for disease damage, \
         plan pre-emergent timing (that window comes first!). \
         First fertilization should wait until after 2-3 mowings.",
    )
}

fn build_almost_ready(soil_temp: f64) -> Recommendation {
    Recommendation::new(
        "spring_n_almost",
        RecommendationCategory::Fertilizer,
        Severity::Info,
        "Spring Fertilizer - Almost Time",
        format!(
            "Soil temperature ({:.1}°F) is approaching the 55°F threshold. \
             Spring fertilization window opening soon.",
            soil_temp
        ),
    )
    .with_explanation(
        "You're close to the right conditions for spring nitrogen. Wait for soil \
         to consistently reach 55°F and ensure you've completed 2-3 mowing cycles. \
         This confirms the grass is actively growing and roots are ready for nutrients. \
         Remember: pre-emergent timing (50-55°F soil) comes before fertilization.",
    )
    .with_data_point(
        "Current Soil Temp",
        format!("{:.1}°F", soil_temp),
        "NOAA USCRN",
    )
    .with_data_point("Target", "55°F + 2-3 mowings", "Agronomic")
    .with_action(
        "Continue monitoring soil temperature. Start mowing when grass needs it. \
         After your second or third mowing AND soil is 55°F+, apply light spring nitrogen. \
         Don't rush - a week or two of patience makes a stronger summer lawn.",
    )
}

fn build_ready_to_fertilize(soil_temp: f64, profile: &LawnProfile) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(5000.0);
    let n_needed = lawn_size / 1000.0 * 0.5; // Light spring app: 0.5 lb N

    Recommendation::new(
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
    .with_explanation(
        "With soil at 55°F+, roots are active and can utilize applied nitrogen. \
         Spring feeding should be LIGHT compared to fall - cool-season grass does \
         most of its feeding in autumn. A light spring application (0.5 lb N/1000 sqft) \
         supports spring growth without pushing excessive top growth that weakens the plant.",
    )
    .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp), "NOAA USCRN")
    .with_data_point("Recommended Rate", "0.5 lb N/1000 sqft", "Agronomic")
    .with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn (0.5 lb N/1000 sqft). \
         Use slow-release nitrogen to avoid surge growth. \
         This should be your ONLY spring nitrogen - save the heavy feeding for fall. \
         Verify you've mowed 2-3 times first to confirm grass is actively growing.",
        n_needed, lawn_size
    ))
}
