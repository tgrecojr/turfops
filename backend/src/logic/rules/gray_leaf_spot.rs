use super::disease_common::gray_leaf_spot_fungicide_rec;
use super::thresholds::*;
use super::Rule;
use crate::models::{
    analyze_fungicide_rotation, Application, ApplicationType, DataSource, EnvironmentalSummary,
    LawnProfile, Recommendation, RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Gray Leaf Spot rule (Pyricularia grisea / Magnaporthe oryzae)
///
/// Devastating TTTF disease, especially on newly overseeded turf.
///
/// Conditions (NC State Extension):
/// - Window: July 1 - September 30
/// - Triggers: Ambient 70-95°F + humidity >85% sustained (14+ hrs leaf wetness proxy)
/// - Risk amplifier: Recent overseeding (within 60 days) — newly established turf is
///   extremely susceptible
/// - Excessive nitrogen increases susceptibility
///
/// Severity:
/// - Advisory: 1-2 favorable days
/// - Warning: 3+ days or recent overseed
/// - Critical: Sustained conditions + recent overseed
pub struct GrayLeafSpotRule;

impl Rule for GrayLeafSpotRule {
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

        // Window: July 1 - September 30
        let window_start = NaiveDate::from_ymd_opt(current_year, 7, 1)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 9, 30)?;

        if today < window_start || today > window_end {
            return None;
        }

        let forecast = env.forecast.as_ref()?;
        let current = env.current.as_ref()?;

        let ambient_temp = current.ambient_temp_f?;
        let humidity = current.humidity_percent?;

        // Check temperature range (70-95°F)
        if !(GRAY_LEAF_SPOT_TEMP_LOW_F..=GRAY_LEAF_SPOT_TEMP_HIGH_F).contains(&ambient_temp) {
            return None;
        }

        // Check humidity threshold (>85%)
        if humidity <= HUMIDITY_HIGH_DISEASE {
            return None;
        }

        // Count favorable forecast days
        let favorable_days: usize = forecast
            .next_days(5)
            .iter()
            .filter(|d| {
                d.high_temp_f >= GRAY_LEAF_SPOT_TEMP_LOW_F
                    && d.high_temp_f <= GRAY_LEAF_SPOT_TEMP_HIGH_F
                    && d.avg_humidity >= HUMIDITY_HIGH_DISEASE
            })
            .count();

        if favorable_days == 0 {
            return None;
        }

        // Check for recent overseeding (within 60 days) — major risk amplifier
        let recent_overseed = history.iter().any(|app| {
            app.application_type == ApplicationType::Overseed
                && (today - app.application_date).num_days() <= N_DEFICIENCY_DAYS_60
        });

        let severity = if favorable_days >= 3 && recent_overseed {
            Severity::Critical
        } else if favorable_days >= 3 || recent_overseed {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        let overseed_note = if recent_overseed {
            " CRITICAL: Recently overseeded turf is extremely susceptible to gray leaf spot. \
             New seedlings can be destroyed within days."
        } else {
            ""
        };

        let rec = Recommendation::new(
            "gray_leaf_spot",
            RecommendationCategory::DiseasePressure,
            severity,
            if recent_overseed {
                "Gray Leaf Spot Risk — New Seedlings At Risk"
            } else {
                "Gray Leaf Spot Risk Elevated"
            },
            format!(
                "Conditions favor gray leaf spot: {:.0}°F, {:.0}% humidity, {} favorable days \
                 in forecast.{}",
                ambient_temp, humidity, favorable_days, overseed_note
            ),
        )
        .with_explanation(
            "Gray leaf spot (Pyricularia grisea) is one of the most destructive diseases \
             of tall fescue, especially on newly established turf (NC State Extension). \
             It thrives at 70-95°F with 14+ hours of leaf wetness. Excessive nitrogen \
             increases susceptibility. Newly overseeded lawns are extremely vulnerable — \
             the disease can destroy new seedlings within days of onset.",
        )
        .with_data_point(
            "Ambient Temp",
            format!("{:.0}°F", ambient_temp),
            DataSource::HomeAssistant.as_str(),
        )
        .with_data_point(
            "Current Humidity",
            format!("{:.0}%", humidity),
            DataSource::HomeAssistant.as_str(),
        )
        .with_data_point(
            "Favorable Days",
            format!("{}", favorable_days),
            DataSource::OpenWeatherMap.as_str(),
        );

        // FRAC-aware product recommendation for gray leaf spot
        let advice = analyze_fungicide_rotation(history);
        let fungicide_rec = gray_leaf_spot_fungicide_rec(&advice);

        let rec = rec.with_action(format!(
            "Limit nitrogen to ≤0.25 lb N/1000sqft. Apply preventive fungicide: {}. \
             Avoid evening irrigation — water only in early morning (4-7 AM). \
             Minimize leaf wetness duration.{}",
            fungicide_rec,
            if recent_overseed {
                " Monitor new seedlings closely — consider fungicide application \
                 before symptoms appear."
            } else {
                ""
            }
        ));

        // Add FRAC data point if applicable
        let rec = if let Some(last) = &advice.last_class {
            rec.with_data_point(
                "Last FRAC Class",
                last.to_string(),
                DataSource::History.as_str(),
            )
        } else {
            rec
        };

        Some(rec)
    }
}
