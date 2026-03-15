use super::disease_common::{
    add_frac_data_points, append_rotation_warning, build_rotation_guidance,
};
use super::thresholds::*;
use super::Rule;
use crate::models::{
    analyze_fungicide_rotation, Application, DataSource, EnvironmentalSummary, LawnProfile,
    Recommendation, RecommendationCategory, Severity,
};

/// Fungicide risk assessment rule
///
/// Brown patch (Rhizoctonia) is a major disease for TTTF, especially
/// during hot, humid conditions.
///
/// Risk conditions (NC State Extension):
/// - Night temps >60°F initiate onset
/// - Night temps >65°F increase risk
/// - Night temps >70°F with day temps >90°F = severe conditions
/// - Humidity >80% sustained for 10+ hours
///
/// Fungicide rotation:
/// - Strobilurins (FRAC 11): azoxystrobin, pyraclostrobin
/// - DMIs (FRAC 3): propiconazole, myclobutanil
/// - Thiophanates (FRAC 1): thiophanate-methyl
/// - Resistance develops after as few as 5 consecutive applications of same class
pub struct FungicideRule;

impl Rule for FungicideRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        // Most relevant for cool-season grasses
        if !profile.grass_type.is_cool_season() {
            return None;
        }

        let current = env.current.as_ref()?;

        let humidity = current.humidity_percent?;
        let ambient_temp = current.ambient_temp_f?;
        let humidity_avg = env.humidity_7day_avg?;

        // Check for disease-favorable conditions
        let high_humidity = humidity > HUMIDITY_DISEASE_RISK;
        let warm_temps = ambient_temp > GRAY_LEAF_SPOT_TEMP_LOW_F;
        let sustained_humidity = humidity_avg > HUMIDITY_RED_THREAD;

        if !high_humidity || !warm_temps {
            return None;
        }

        // Estimate night temp from forecast lows or current conditions
        let night_temp = env
            .forecast
            .as_ref()
            .and_then(|f| f.next_days(1).first().map(|d| d.low_temp_f))
            .unwrap_or(ambient_temp - 15.0); // rough estimate if no forecast

        // NC State: onset begins at night temps >60°F
        if night_temp < BROWN_PATCH_NIGHT_ONSET_F {
            return None;
        }

        // Determine severity with tiered night temperature thresholds
        let day_temp = env
            .forecast
            .as_ref()
            .and_then(|f| f.next_days(1).first().map(|d| d.high_temp_f))
            .unwrap_or(ambient_temp);

        let severity = if night_temp >= BROWN_PATCH_NIGHT_SEVERE_F
            && day_temp >= BROWN_PATCH_DAY_SEVERE_F
            && sustained_humidity
        {
            // NC State: severe conditions at >70°F night + >90°F day
            Severity::Critical
        } else if night_temp >= BROWN_PATCH_NIGHT_ELEVATED_F
            || (humidity > HUMIDITY_SEVERE_DISEASE
                && ambient_temp > RED_THREAD_TEMP_HIGH_F
                && sustained_humidity)
        {
            Severity::Warning
        } else {
            // 60-65°F night temps = Advisory
            Severity::Advisory
        };

        let severity_label = match severity {
            Severity::Critical => "Critical",
            Severity::Warning => "Warning",
            _ => "Advisory",
        };

        // Analyze FRAC rotation history
        let advice = analyze_fungicide_rotation(history);

        let rotation_guidance = build_rotation_guidance(&advice);

        let mut action = match severity {
            Severity::Critical => {
                format!(
                    "Apply preventative fungicide immediately. {} \
                     Limit nitrogen to 0.5 lb N/1000sqft or less. \
                     Water ONLY in early morning (4-7 AM). Avoid evening irrigation. \
                     Monitor for circular brown patches with 'smoke ring' border.",
                    rotation_guidance
                )
            }
            Severity::Warning => {
                format!(
                    "Consider preventative fungicide if lawn has history of brown patch. {} \
                     Switch to early morning watering only. Limit nitrogen to 0.5 lb N/1000sqft. \
                     Inspect lawn for early symptoms.",
                    rotation_guidance
                )
            }
            _ => "Monitor conditions — brown patch onset begins at night temps above 60°F. \
                 Water early morning only. Prepare fungicide for application if conditions worsen. \
                 Avoid high-nitrogen fertilizer during risk periods."
                .to_string(),
        };

        action = append_rotation_warning(&action, &advice);

        let rec = Recommendation::new(
            "fungicide_risk",
            RecommendationCategory::Fungicide,
            severity,
            format!("Brown Patch Risk — {}", severity_label),
            format!(
                "Current conditions favor brown patch disease. Humidity: {:.0}%, Temp: {:.1}°F, \
                 Est. night temp: {:.0}°F.",
                humidity, ambient_temp, night_temp
            ),
        )
        .with_explanation(
            "Brown patch (Rhizoctonia solani) onset begins when nighttime temperatures \
             exceed 60°F (NC State Extension). Severity increases at night temps >65°F, \
             and becomes critical when nights exceed 70°F with day temps >90°F. \
             Tall Fescue is particularly susceptible. Symptoms include circular patches of \
             tan/brown turf with a dark 'smoke ring' border in morning dew. \
             Limit nitrogen to ≤0.5 lb N/1000sqft during active risk — excess N fuels the pathogen.",
        )
        .with_data_point(
            "Current Humidity",
            format!("{:.0}%", humidity),
            DataSource::HomeAssistant.as_str(),
        )
        .with_data_point(
            "Ambient Temp",
            format!("{:.1}°F", ambient_temp),
            DataSource::HomeAssistant.as_str(),
        )
        .with_data_point(
            "Est. Night Temp",
            format!("{:.0}°F", night_temp),
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_data_point(
            "7-Day Avg Humidity",
            format!("{:.0}%", humidity_avg),
            DataSource::Calculated.as_str(),
        )
        .with_action(action);

        Some(add_frac_data_points(rec, &advice))
    }
}
