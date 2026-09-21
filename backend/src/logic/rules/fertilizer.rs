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

        // Ambient comes from Home Assistant only; without it the soil checks still apply.
        let ambient_temp = current.ambient_temp_f;
        let soil_moisture = current.primary_soil_moisture();

        let mut warnings: Vec<String> = Vec::new();
        let mut data_points: Vec<(&str, String, &str)> = Vec::new();

        // Check heat stress
        if let Some(ambient_temp) = ambient_temp.filter(|t| *t > HEAT_STRESS_TEMP_F) {
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

        let severity = if ambient_temp.is_some_and(|t| t > HEAT_STRESS_WARNING_TEMP_F)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvironmentalReading, GrassType};

    fn profile() -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type: GrassType::TallFescue,
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn env(ambient: Option<f64>, moisture: f64) -> EnvironmentalSummary {
        let mut reading = EnvironmentalReading::new(DataSource::SoilData);
        reading.ambient_temp_f = ambient;
        reading.soil_moisture_10 = Some(moisture);
        EnvironmentalSummary {
            current: Some(reading),
            ..Default::default()
        }
    }

    #[test]
    fn saturated_soil_blocks_without_an_ambient_reading() {
        // Home Assistant down: the soil checks must still fire.
        let rec = FertilizerRule
            .evaluate(&env(None, 0.45), &profile(), &[])
            .expect("saturated soil blocks fertilizer");
        assert_eq!(rec.severity, Severity::Warning);
    }

    #[test]
    fn nothing_to_report_in_good_conditions() {
        assert!(FertilizerRule
            .evaluate(&env(Some(72.0), 0.25), &profile(), &[])
            .is_none());
        assert!(FertilizerRule
            .evaluate(&env(None, 0.25), &profile(), &[])
            .is_none());
    }
}
