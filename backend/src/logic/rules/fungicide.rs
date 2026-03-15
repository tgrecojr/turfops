use super::Rule;
use crate::models::{
    analyze_fungicide_rotation, Application, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
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
        let high_humidity = humidity > 80.0;
        let warm_temps = ambient_temp > 70.0;
        let sustained_humidity = humidity_avg > 75.0;

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
        if night_temp < 60.0 {
            return None;
        }

        // Determine severity with tiered night temperature thresholds
        let day_temp = env
            .forecast
            .as_ref()
            .and_then(|f| f.next_days(1).first().map(|d| d.high_temp_f))
            .unwrap_or(ambient_temp);

        let severity = if night_temp >= 70.0 && day_temp >= 90.0 && sustained_humidity {
            // NC State: severe conditions at >70°F night + >90°F day
            Severity::Critical
        } else if night_temp >= 65.0
            || (humidity > 90.0 && ambient_temp > 80.0 && sustained_humidity)
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

        let rotation_guidance = if let Some(next) = &advice.recommended_next {
            let products = next.common_products();
            let example = products.first().copied().unwrap_or("(see label)");
            format!(
                "Recommended next application: {} (e.g., {}).",
                next, example
            )
        } else {
            "Rotate between FRAC classes: Strobilurins (FRAC 11), DMIs (FRAC 3), \
             Thiophanates (FRAC 1)."
                .to_string()
        };

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

        if let Some(warning) = &advice.rotation_warning {
            action = format!("{} ROTATION WARNING: {}", action, warning);
        }

        let mut rec = Recommendation::new(
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
            "Patio Sensor",
        )
        .with_data_point(
            "Ambient Temp",
            format!("{:.1}°F", ambient_temp),
            "Patio Sensor",
        )
        .with_data_point(
            "Est. Night Temp",
            format!("{:.0}°F", night_temp),
            "OpenWeatherMap",
        )
        .with_data_point(
            "7-Day Avg Humidity",
            format!("{:.0}%", humidity_avg),
            "Calculated",
        )
        .with_action(action);

        if let Some(last) = &advice.last_class {
            rec = rec.with_data_point("Last FRAC Class", last.to_string(), "History");
        }
        if let Some(next) = &advice.recommended_next {
            rec = rec.with_data_point("Recommended Next", next.to_string(), "Rotation");
        }

        Some(rec)
    }
}
