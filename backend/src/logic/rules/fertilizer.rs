use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
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
        if ambient_temp > HEAT_STRESS_TEMP_F {
            warnings.push(format!(
                "Ambient temperature ({:.1}°F) exceeds {:.0}°F heat stress threshold",
                ambient_temp, HEAT_STRESS_TEMP_F
            ));
            data_points.push((
                "Ambient Temp",
                format!("{:.1}°F", ambient_temp),
                DataSource::HomeAssistant.as_str(),
            ));
        }

        // Check soil moisture
        if let Some(moisture) = soil_moisture {
            if moisture < SOIL_MOISTURE_DROUGHT {
                warnings.push(format!(
                    "Soil moisture ({:.2}) indicates drought stress (below {:.2})",
                    moisture, SOIL_MOISTURE_DROUGHT
                ));
                data_points.push((
                    "Soil Moisture",
                    format!("{:.2}", moisture),
                    DataSource::SoilData.as_str(),
                ));
            } else if moisture > SOIL_MOISTURE_SATURATED {
                warnings.push(format!(
                    "Soil moisture ({:.2}) indicates saturation (above {:.2}) - fertilizer may leach",
                    moisture, SOIL_MOISTURE_SATURATED
                ));
                data_points.push((
                    "Soil Moisture",
                    format!("{:.2}", moisture),
                    DataSource::SoilData.as_str(),
                ));
            }
        }

        if warnings.is_empty() {
            return None;
        }

        let severity = if ambient_temp > HEAT_STRESS_WARNING_TEMP_F
            || soil_moisture.is_some_and(|m| m < SOIL_MOISTURE_SEVERE_DROUGHT)
        {
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
        .with_explanation(format!(
            "Cool-season grasses like Tall Fescue experience heat stress above {:.0}°F and may \
             go partially dormant. Applying nitrogen during stress can cause fertilizer burn \
             and damage the lawn. Wait for cooler temperatures or improved soil moisture.",
            HEAT_STRESS_TEMP_F
        ));

        for (label, value, source) in data_points {
            rec = rec.with_data_point(label, value, source);
        }

        if let Some(soil_temp) = current.soil_temp_10_f {
            rec = rec.with_data_point(
                "Soil Temp (10cm)",
                format!("{:.1}°F", soil_temp),
                DataSource::SoilData.as_str(),
            );
        }

        rec = rec.with_action(format!(
            "Delay fertilizer application until ambient temperature drops below {:.0}°F \
             and soil moisture is between {:.2}-{:.2}. Consider irrigation if drought-stressed.",
            HEAT_STRESS_TEMP_F, SOIL_MOISTURE_DROUGHT, SOIL_MOISTURE_SATURATED
        ));

        Some(rec)
    }
}
