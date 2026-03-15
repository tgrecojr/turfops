use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local};

/// Red Thread rule (Laetisaria fuciformis)
///
/// Common spring/fall disease that signals nitrogen deficiency rather than
/// needing fungicide (NC State Extension).
///
/// Conditions:
/// - Window: March-May, September-November
/// - Triggers: Temp 40-80°F (peak at 70°F) + high humidity/rain + nitrogen deficiency
/// - Key insight: Red thread is managed by fertilizing, not by fungicide
///
/// Severity:
/// - Info: Favorable conditions but recently fertilized
/// - Advisory: Favorable + no recent N
/// - Warning: Extended favorable + no recent N
pub struct RedThreadRule;

impl Rule for RedThreadRule {
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

        // Window: March-May or September-November
        let in_window = (3..=5).contains(&month) || (9..=11).contains(&month);
        if !in_window {
            return None;
        }

        let current = env.current.as_ref()?;
        let ambient_temp = current.ambient_temp_f?;
        let humidity = current.humidity_percent?;

        // Temperature range: 40-80°F (peak at 70°F)
        if !(RED_THREAD_TEMP_LOW_F..=RED_THREAD_TEMP_HIGH_F).contains(&ambient_temp) {
            return None;
        }

        // Need elevated humidity or recent rain
        let humid = humidity >= HUMIDITY_RED_THREAD;
        let recent_rain = env
            .precipitation_7day_total_mm
            .map(|p| p > RED_THREAD_RAIN_7DAY_MM)
            .unwrap_or(false);

        if !humid && !recent_rain {
            return None;
        }

        // Count favorable forecast days
        let favorable_days = env
            .forecast
            .as_ref()
            .map(|f| {
                f.next_days(5)
                    .iter()
                    .filter(|d| {
                        d.high_temp_f >= RED_THREAD_TEMP_LOW_F
                            && d.high_temp_f <= RED_THREAD_TEMP_HIGH_F
                            && (d.avg_humidity >= HUMIDITY_RED_THREAD
                                || d.total_precipitation_mm >= PRECIP_TRACE_MM)
                    })
                    .count()
            })
            .unwrap_or(0);

        // Check nitrogen status — this is the key factor
        let cutoff_45 = today - chrono::Duration::days(N_DEFICIENCY_DAYS_45);
        let cutoff_60 = today - chrono::Duration::days(N_DEFICIENCY_DAYS_60);

        let n_deficient_45 = !history.iter().any(|app| {
            app.application_type == ApplicationType::Fertilizer && app.application_date >= cutoff_45
        });
        let n_deficient_60 = !history.iter().any(|app| {
            app.application_type == ApplicationType::Fertilizer && app.application_date >= cutoff_60
        });

        let severity = if favorable_days >= 3 && n_deficient_60 {
            Severity::Warning
        } else if n_deficient_45 {
            Severity::Advisory
        } else {
            Severity::Info
        };

        // For Info (recently fertilized), only show if conditions are very favorable
        if severity == Severity::Info && favorable_days < 3 {
            return None;
        }

        let n_status = if n_deficient_60 {
            "No nitrogen applied in 60+ days — this nitrogen deficiency is the primary \
             predisposing factor for red thread."
        } else if n_deficient_45 {
            "No nitrogen applied in 45+ days — nitrogen deficiency increases red thread risk."
        } else {
            "Recently fertilized — red thread risk is lower with adequate nitrogen."
        };

        let rec = Recommendation::new(
            "red_thread",
            RecommendationCategory::DiseasePressure,
            severity,
            if n_deficient_45 {
                "Red Thread Risk — Nitrogen Deficiency Detected"
            } else {
                "Red Thread Conditions Present"
            },
            format!(
                "Conditions favor red thread: {:.0}°F, {:.0}% humidity, {} favorable days ahead. \
                 {}",
                ambient_temp, humidity, favorable_days, n_status
            ),
        )
        .with_explanation(
            "Red thread (Laetisaria fuciformis) occurs at 40-80°F (peak 70°F) with high \
             humidity or rain (NC State Extension). Unlike most fungal diseases, red thread \
             is primarily managed through FERTILIZATION, not fungicide. It is a reliable \
             indicator of nitrogen deficiency. Symptoms are pinkish-red thread-like strands \
             on leaf tips. Applying 0.5-1.0 lb N/1000sqft usually resolves the issue.",
        )
        .with_data_point(
            "Ambient Temp",
            format!("{:.0}°F", ambient_temp),
            DataSource::HomeAssistant.as_str(),
        )
        .with_data_point(
            "Humidity",
            format!("{:.0}%", humidity),
            DataSource::HomeAssistant.as_str(),
        )
        .with_data_point(
            "Favorable Days",
            format!("{}", favorable_days),
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_action(
            "Apply nitrogen — red thread is usually a sign of nitrogen deficiency, \
             NOT a need for fungicide (NC State Extension). Apply 0.5-1.0 lb N/1000sqft. \
             Fungicide is rarely necessary for red thread; correcting the nitrogen deficit \
             is the primary treatment.",
        );

        Some(rec)
    }
}
