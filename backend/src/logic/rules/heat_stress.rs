use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};

/// Heat stress warning rule - warns about upcoming heat stress conditions
///
/// Cool-season grasses (TTTF, KBG, PRG) struggle when temps exceed 85°F.
///
/// Conditions:
/// - Max temp >85°F forecasted in next 3 days
///
/// Severity levels:
/// - Advisory: 85-90°F expected
/// - Warning: 90-95°F expected
/// - Critical: >95°F expected
pub struct HeatStressRule;

impl Rule for HeatStressRule {
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

        let forecast = env.forecast.as_ref()?;

        // Find max temp in next 3 days
        let max_temp = forecast.max_temp_next_days(3)?;

        // No warning if temps are mild
        if max_temp < HEAT_STRESS_TEMP_F {
            return None;
        }

        // Count consecutive hot days
        let hot_days: usize = forecast
            .next_days(5)
            .iter()
            .take_while(|d| d.high_temp_f >= HEAT_STRESS_TEMP_F)
            .count();

        let severity = if max_temp >= HEAT_STRESS_CRITICAL_TEMP_F {
            Severity::Critical
        } else if max_temp >= HEAT_STRESS_WARNING_TEMP_F {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        Some(self.build_recommendation(severity, max_temp, hot_days))
    }
}

impl HeatStressRule {
    fn build_recommendation(
        &self,
        severity: Severity,
        max_temp: f64,
        hot_days: usize,
    ) -> Recommendation {
        let title = match severity {
            Severity::Critical => "Extreme Heat Stress Expected",
            Severity::Warning => "Heat Stress Warning",
            _ => "Warm Weather Ahead",
        };

        let description = format!(
            "Temperatures up to {:.0}°F expected over the next {} days. \
             Cool-season grasses experience stress above {:.0}°F.",
            max_temp,
            hot_days.max(1),
            HEAT_STRESS_TEMP_F
        );

        let action = match severity {
            Severity::Critical => {
                "Avoid ALL fertilizer applications. Raise mowing height to 4 inches (TTTF max summer height). \
                 Never cut more than 1/3 of the blade at once — if raising height, do so gradually \
                 over 1-2 mowings (Missouri Extension g6705). \
                 Water early morning (before 7 AM) only. Do not mow during peak heat. \
                 Accept some dormancy as natural protection."
            }
            Severity::Warning => {
                "Avoid fertilizer applications, especially high-nitrogen. Raise mowing \
                 height to 3.5-4 inches. Never remove more than 1/3 of the blade at once — \
                 if raising height, do so gradually over 1-2 mowings. \
                 Water deeply in early morning. Consider skipping mowing to reduce stress."
            }
            _ => {
                "Consider raising mowing height toward 3.5-4 inches. Never cut more than \
                 1/3 of the blade at once. Water early morning if needed. \
                 Avoid fertilizer applications until temps moderate."
            }
        };

        Recommendation::new(
            "heat_stress_forecast",
            RecommendationCategory::HeatStress,
            severity,
            title,
            description,
        )
        .with_explanation(format!(
            "Tall Fescue and other cool-season grasses evolved for temperatures between \
             {:.0}-{:.0}°F. Above {:.0}°F, photosynthesis slows and root growth stops. Above {:.0}°F, \
             the grass may enter summer dormancy. Fertilizing during heat stress forces \
             top growth at the expense of roots, weakening the plant. Taller grass shades \
             the crown and soil, reducing heat stress.",
            COOL_SEASON_IDEAL_LOW_F,
            COOL_SEASON_IDEAL_HIGH_F,
            HEAT_STRESS_TEMP_F,
            HEAT_STRESS_WARNING_TEMP_F
        ))
        .with_data_point(
            "Max Forecast Temp",
            format!("{:.0}°F", max_temp),
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_data_point(
            "Hot Days",
            format!("{}", hot_days),
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_action(action)
    }
}
