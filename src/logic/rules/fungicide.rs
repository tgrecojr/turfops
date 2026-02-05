use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};

/// Fungicide risk assessment rule
///
/// Brown patch (Rhizoctonia) is a major disease for TTTF, especially
/// during hot, humid conditions.
///
/// Risk conditions:
/// - Humidity >80% sustained for 10+ hours
/// - Ambient temp >70°F during high humidity
/// - Night temps remaining above 65°F
pub struct FungicideRule;

impl Rule for FungicideRule {
    fn id(&self) -> &'static str {
        "fungicide_risk"
    }

    fn name(&self) -> &'static str {
        "Fungicide Disease Risk"
    }

    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        _history: &[Application],
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

        let severity = if humidity > 90.0 && ambient_temp > 80.0 && sustained_humidity {
            Severity::Critical
        } else if sustained_humidity {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        let rec = Recommendation::new(
            "fungicide_risk",
            RecommendationCategory::Fungicide,
            severity,
            "Brown Patch Risk Elevated",
            format!(
                "Current conditions favor brown patch disease. Humidity: {:.0}%, Temp: {:.1}°F",
                humidity, ambient_temp
            ),
        )
        .with_explanation(
            "Brown patch (Rhizoctonia solani) thrives in hot, humid conditions with night \
             temperatures above 65°F. Tall Fescue is particularly susceptible. Symptoms include \
             circular patches of tan/brown turf with a dark 'smoke ring' border in morning dew.",
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
            "7-Day Avg Humidity",
            format!("{:.0}%", humidity_avg),
            "Calculated",
        )
        .with_action(
            "Consider preventive fungicide application (azoxystrobin, propiconazole, or \
             thiophanate-methyl). Avoid evening irrigation - water early morning. \
             Reduce nitrogen applications during high-risk periods.",
        );

        Some(rec)
    }
}
