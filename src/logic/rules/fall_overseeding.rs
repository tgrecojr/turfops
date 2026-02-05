use super::Rule;
use crate::models::{
    Application, ApplicationType, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Fall overseeding timing rule
///
/// Tall Fescue doesn't spread via rhizomes or stolons - overseeding is the
/// only way to thicken a lawn. Timing is critical:
/// - Too early: Seedlings die from heat stress
/// - Too late: Not enough time to establish before winter
///
/// Optimal window: Soil temp 50-65°F, late August through October
/// Germination requires consistent moisture for 10-14 days
pub struct FallOverseedingRule;

impl Rule for FallOverseedingRule {
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

        // Define the overseeding window (Aug 15 - Oct 31 for Zone 7a)
        let window_start = NaiveDate::from_ymd_opt(current_year, 8, 15)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 10, 31)?;

        // Only evaluate during the window
        if today < window_start || today > window_end {
            return None;
        }

        // Check if already overseeded this fall
        let already_seeded = history.iter().any(|app| {
            app.application_type == ApplicationType::Overseed
                && app.application_date.year() == current_year
                && app.application_date >= window_start
        });

        if already_seeded {
            return None;
        }

        // Get soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;
        let current_soil_temp = env.current.as_ref()?.soil_temp_10_f?;

        // Check forecast for upcoming conditions (if available)
        let forecast_favorable = env
            .forecast
            .as_ref()
            .map(|f| {
                // Check if next 14 days have reasonable temps
                let days = f.next_days(14);
                let avg_high: f64 =
                    days.iter().map(|d| d.high_temp_f).sum::<f64>() / days.len().max(1) as f64;

                // Favorable if highs are moderate (rain helps but irrigation can substitute)
                avg_high < 85.0
            })
            .unwrap_or(true);

        // Calculate days remaining in window
        let days_remaining = (window_end - today).num_days();

        // Determine recommendation based on soil temp
        if (50.0..=65.0).contains(&soil_temp_avg) {
            // Optimal window
            let severity = if (55.0..=62.0).contains(&soil_temp_avg) {
                // Peak germination range
                if days_remaining < 21 {
                    Severity::Warning // Optimal but running low on time
                } else {
                    Severity::Advisory
                }
            } else if days_remaining < 14 {
                Severity::Warning
            } else {
                Severity::Advisory
            };

            let mut rec = Recommendation::new(
                format!("fall_overseeding_{}", current_year),
                RecommendationCategory::Overseeding,
                severity,
                "Fall Overseeding Window Open",
                format!(
                    "Soil temperature ({:.1}°F) is ideal for TTTF seed germination. \
                     {} days remaining in optimal window.",
                    soil_temp_avg, days_remaining
                ),
            );

            rec = rec
                .with_explanation(
                    "Tall Fescue doesn't spread on its own - overseeding is the only way to \
                     thicken your lawn and fill bare spots. Fall is THE best time because: \
                     (1) soil is warm for germination, (2) air is cool reducing seedling stress, \
                     (3) weed competition is minimal, (4) fall rains provide moisture. \
                     Seeds need 10-14 days of consistent moisture to germinate.",
                )
                .with_data_point(
                    "7-Day Avg Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    "NOAA USCRN",
                )
                .with_data_point(
                    "Current Soil Temp",
                    format!("{:.1}°F", current_soil_temp),
                    "NOAA USCRN",
                )
                .with_data_point("Days Remaining", format!("{}", days_remaining), "Calendar");

            // Add forecast note if available
            if !forecast_favorable {
                rec = rec.with_data_point(
                    "Forecast Note",
                    "Hot weather ahead - monitor seedlings",
                    "OpenWeatherMap",
                );
            }

            let seeding_rate = if profile.lawn_size_sqft.unwrap_or(5000.0) > 0.0 {
                let sqft = profile.lawn_size_sqft.unwrap_or(5000.0);
                let lbs_needed = sqft / 1000.0 * 4.0; // 4 lbs per 1000 sqft for overseeding
                format!(
                    "For your {:.0} sqft lawn: ~{:.0} lbs of TTTF seed (4 lbs/1000 sqft for overseeding). \
                     Mow low (2\"), dethatch or aerate first for seed-to-soil contact. \
                     Keep soil moist (light watering 2-3x daily) for 14 days. \
                     Avoid foot traffic for 3-4 weeks.",
                    sqft, lbs_needed
                )
            } else {
                "Seed at 4 lbs per 1000 sqft for overseeding (8 lbs for bare soil). \
                 Mow low (2\"), dethatch or aerate first for seed-to-soil contact. \
                 Keep soil moist (light watering 2-3x daily) for 14 days. \
                 Avoid foot traffic for 3-4 weeks."
                    .to_string()
            };

            rec = rec.with_action(seeding_rate);

            Some(rec)
        } else if soil_temp_avg > 65.0 && soil_temp_avg <= 75.0 {
            // Soil still warm - might be early in window
            if today < NaiveDate::from_ymd_opt(current_year, 9, 15)? {
                // Early September - wait for cooler temps
                let rec = Recommendation::new(
                    format!("fall_overseeding_wait_{}", current_year),
                    RecommendationCategory::Overseeding,
                    Severity::Info,
                    "Overseeding Window Approaching",
                    format!(
                        "Soil temperature ({:.1}°F) is still warm. \
                         Wait for temps to drop below 65°F for best germination.",
                        soil_temp_avg
                    ),
                )
                .with_explanation(
                    "TTTF germinates best when soil is 50-65°F. Seeding when soil is too warm \
                     can stress seedlings. The window typically opens mid-September in Zone 7a.",
                )
                .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp_avg), "NOAA USCRN")
                .with_action(
                    "Prepare for overseeding: order seed, plan aeration, \
                     gather supplies. Monitor soil temps weekly.",
                );

                Some(rec)
            } else {
                // Late September+ with warm soil - seed anyway, window closing
                let rec = Recommendation::new(
                    format!("fall_overseeding_late_{}", current_year),
                    RecommendationCategory::Overseeding,
                    Severity::Warning,
                    "Overseeding - Soil Warm but Window Closing",
                    format!(
                        "Soil ({:.1}°F) is warmer than ideal, but {} days remain in window. \
                         Consider seeding soon despite conditions.",
                        soil_temp_avg, days_remaining
                    ),
                )
                .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp_avg), "NOAA USCRN")
                .with_action(
                    "Seed soon if you haven't already. Water more frequently to keep \
                     seedlings cool. Soil temps will drop as nights get cooler.",
                );

                Some(rec)
            }
        } else if soil_temp_avg < 50.0 {
            // Getting cold - urgent if not seeded
            if days_remaining > 14 {
                let rec = Recommendation::new(
                    format!("fall_overseeding_cold_{}", current_year),
                    RecommendationCategory::Overseeding,
                    Severity::Warning,
                    "Overseeding Window Narrowing - Cool Soil",
                    format!(
                        "Soil temperature ({:.1}°F) is below optimal. \
                         Germination will be slow. Seed immediately if planned.",
                        soil_temp_avg
                    ),
                )
                .with_data_point("Soil Temp", format!("{:.1}°F", soil_temp_avg), "NOAA USCRN")
                .with_action(
                    "If overseeding, do it NOW. Germination slows significantly below 50°F. \
                     Seedlings need 4-6 weeks before hard frost to establish.",
                );

                Some(rec)
            } else {
                // Very late - probably too late for this year
                None
            }
        } else {
            None
        }
    }
}
