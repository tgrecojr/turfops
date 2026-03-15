use super::thresholds::*;
use super::Rule;
use crate::models::{
    analyze_fungicide_rotation, Application, DataSource, EnvironmentalSummary, LawnProfile,
    Recommendation, RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Pythium Blight rule (Pythium spp.)
///
/// Fast-moving disease that can destroy perennial ryegrass/bluegrass in 2-3 days.
///
/// Conditions (NC State Extension):
/// - Window: June - September
/// - Triggers: Night temps >65°F AND day temps >85°F + 12-14 hrs wetness for consecutive nights
/// - Risk amplifier: Thunderstorm activity in forecast
///
/// Severity:
/// - Advisory: Single day of favorable conditions
/// - Warning: 2+ consecutive days
/// - Critical: Conditions + thunderstorm + recent precipitation
pub struct PythiumBlightRule;

impl Rule for PythiumBlightRule {
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

        // Window: June 1 - September 30
        let window_start = NaiveDate::from_ymd_opt(current_year, 6, 1)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 9, 30)?;

        if today < window_start || today > window_end {
            return None;
        }

        let forecast = env.forecast.as_ref()?;

        // Count consecutive days with Pythium conditions:
        // Night >65°F AND day >85°F AND high humidity
        let mut consecutive_favorable = 0_usize;
        let mut has_thunderstorm = false;

        for day in forecast.next_days(5) {
            let warm_nights = day.low_temp_f >= PYTHIUM_NIGHT_MIN_F;
            let hot_days = day.high_temp_f >= PYTHIUM_DAY_MIN_F;
            let wet = day.avg_humidity >= HUMIDITY_DISEASE_RISK
                || day.total_precipitation_mm >= PRECIP_TRACE_MM
                || day.max_precipitation_prob >= PRECIP_PROB_THUNDERSTORM;

            if warm_nights && hot_days && wet {
                consecutive_favorable += 1;
            } else {
                break; // Must be consecutive
            }

            // Check for thunderstorm conditions (high precip prob + warm)
            if day.max_precipitation_prob >= PRECIP_PROB_THUNDERSTORM
                && day.high_temp_f >= PYTHIUM_DAY_MIN_F
            {
                has_thunderstorm = true;
            }
        }

        if consecutive_favorable == 0 {
            return None;
        }

        // Check recent precipitation
        let recent_heavy_rain = env
            .precipitation_7day_total_mm
            .map(|p| p > PRECIP_HEAVY_7DAY_MM) // >1 inch
            .unwrap_or(false);

        let severity = if consecutive_favorable >= 2 && (has_thunderstorm || recent_heavy_rain) {
            Severity::Critical
        } else if consecutive_favorable >= 2 {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        let storm_note = if has_thunderstorm {
            " Thunderstorm activity in forecast amplifies Pythium risk."
        } else {
            ""
        };

        let rec = Recommendation::new(
            "pythium_blight",
            RecommendationCategory::DiseasePressure,
            severity,
            match severity {
                Severity::Critical => "Pythium Blight Risk — Critical",
                Severity::Warning => "Pythium Blight Risk Elevated",
                _ => "Pythium Blight Conditions Developing",
            },
            format!(
                "{} consecutive days of Pythium-favorable conditions (night >65°F, day >85°F, \
                 high moisture).{}",
                consecutive_favorable, storm_note
            ),
        )
        .with_explanation(
            "Pythium blight is a fast-moving disease that can destroy turf in 2-3 days \
             (NC State Extension). It requires night temps >65°F AND day temps >85°F with \
             12-14 hours of leaf wetness. Summer thunderstorms create ideal conditions. \
             Perennial ryegrass and Kentucky bluegrass are most susceptible. \
             Pythium appears as greasy, water-soaked patches that collapse rapidly.",
        )
        .with_data_point(
            "Consecutive Favorable Days",
            format!("{}", consecutive_favorable),
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_data_point(
            "Thunderstorm Risk",
            if has_thunderstorm { "Yes" } else { "No" },
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_action({
            // Pythium-specific chemistry: mefenoxam (FRAC 4) and fosetyl-Al (FRAC P07)
            // are not in our general FRAC model, so keep the specific recommendation
            let mut action = format!(
                "Limit nitrogen to ≤0.25 lb N/1000sqft (NC State). \
                 Apply preventive fungicide (mefenoxam or fosetyl-Al) if not already applied. \
                 Improve drainage where possible. Water ONLY in early morning (4-7 AM). \
                 Avoid any irrigation in evening.{}",
                if severity == Severity::Critical {
                    " ACT IMMEDIATELY — Pythium can destroy turf within 48-72 hours."
                } else {
                    ""
                }
            );

            // Add rotation context if user has been applying general-purpose fungicides
            let advice = analyze_fungicide_rotation(history);
            if let Some(warning) = &advice.rotation_warning {
                action = format!("{} Note on general fungicide rotation: {}", action, warning);
            }

            action
        });

        Some(rec)
    }
}
