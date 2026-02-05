use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};

/// Fertilizer stress avoidance rule
///
/// TTTF and other cool-season grasses go dormant in extreme heat.
/// Applying nitrogen during heat stress can burn the lawn.
///
/// Block conditions:
/// - Ambient temp >85°F
/// - Soil moisture <0.10 (drought stress) or >0.40 (saturated)
pub struct FertilizerRule;

impl Rule for FertilizerRule {
    fn id(&self) -> &'static str {
        "fertilizer_block"
    }

    fn name(&self) -> &'static str {
        "Fertilizer Stress Avoidance"
    }

    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        // Only relevant for cool-season grasses
        if !profile.grass_type.is_cool_season() {
            return None;
        }

        let current = env.current.as_ref()?;

        let ambient_temp = current.ambient_temp_f?;
        let soil_moisture = current.primary_soil_moisture();

        let mut warnings: Vec<String> = Vec::new();
        let mut data_points: Vec<(&str, String, &str)> = Vec::new();

        // Check heat stress
        if ambient_temp > 85.0 {
            warnings.push(format!(
                "Ambient temperature ({:.1}°F) exceeds 85°F heat stress threshold",
                ambient_temp
            ));
            data_points.push((
                "Ambient Temp",
                format!("{:.1}°F", ambient_temp),
                "Patio Sensor",
            ));
        }

        // Check soil moisture
        if let Some(moisture) = soil_moisture {
            if moisture < 0.10 {
                warnings.push(format!(
                    "Soil moisture ({:.2}) indicates drought stress (below 0.10)",
                    moisture
                ));
                data_points.push(("Soil Moisture", format!("{:.2}", moisture), "NOAA USCRN"));
            } else if moisture > 0.40 {
                warnings.push(format!(
                    "Soil moisture ({:.2}) indicates saturation (above 0.40) - fertilizer may leach",
                    moisture
                ));
                data_points.push(("Soil Moisture", format!("{:.2}", moisture), "NOAA USCRN"));
            }
        }

        if warnings.is_empty() {
            return None;
        }

        let severity = if ambient_temp > 90.0 || soil_moisture.map_or(false, |m| m < 0.05) {
            Severity::Critical
        } else {
            Severity::Warning
        };

        let mut rec = Recommendation::new(
            "fertilizer_block",
            RecommendationCategory::Fertilizer,
            severity,
            "Avoid Fertilizer Application",
            warnings.join(". "),
        )
        .with_explanation(
            "Cool-season grasses like Tall Fescue experience heat stress above 85°F and may \
             go partially dormant. Applying nitrogen during stress can cause fertilizer burn \
             and damage the lawn. Wait for cooler temperatures or improved soil moisture.",
        );

        for (label, value, source) in data_points {
            rec = rec.with_data_point(label, value, source);
        }

        if let Some(soil_temp) = current.soil_temp_10_f {
            rec = rec.with_data_point(
                "Soil Temp (10cm)",
                format!("{:.1}°F", soil_temp),
                "NOAA USCRN",
            );
        }

        rec = rec.with_action(
            "Delay fertilizer application until ambient temperature drops below 85°F \
             and soil moisture is between 0.10-0.40. Consider irrigation if drought-stressed.",
        );

        Some(rec)
    }
}
