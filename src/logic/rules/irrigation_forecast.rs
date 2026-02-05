use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};

/// Irrigation forecast rule - recommends irrigation based on forecast drought
///
/// Conditions:
/// - No significant rain (<0.1") forecasted for next 5 days
/// - Current soil moisture below threshold
///
/// Severity levels:
/// - Advisory: No rain 5 days, moisture 0.15-0.20
/// - Warning: No rain 5 days, moisture 0.10-0.15
/// - Critical: No rain 5 days, moisture < 0.10
pub struct IrrigationForecastRule;

impl Rule for IrrigationForecastRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        let forecast = env.forecast.as_ref()?;
        let current = env.current.as_ref()?;
        let soil_moisture = current.primary_soil_moisture()?;

        // Skip if soil moisture is adequate
        if soil_moisture >= 0.20 {
            return None;
        }

        // Check for rain in next 5 days (120 hours)
        let rain_5day = forecast.rain_expected_within(120, 0.1);

        // If rain is expected, no irrigation recommendation
        if rain_5day.is_some() {
            return None;
        }

        // Calculate total precipitation expected in next 5 days
        let total_precip: f64 = forecast
            .next_days(5)
            .iter()
            .map(|d| d.total_precipitation_mm)
            .sum();

        // If meaningful rain expected (>2.5mm / 0.1"), no recommendation
        if total_precip > 2.5 {
            return None;
        }

        // Determine severity based on soil moisture
        let severity = if soil_moisture < 0.10 {
            Severity::Critical
        } else if soil_moisture < 0.15 {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        // Calculate days until next rain (if any)
        let dry_days = forecast
            .daily_summary
            .iter()
            .take_while(|d| d.total_precipitation_mm < 2.5 && d.max_precipitation_prob < 0.5)
            .count();

        Some(self.build_recommendation(severity, soil_moisture, dry_days, profile))
    }
}

impl IrrigationForecastRule {
    fn build_recommendation(
        &self,
        severity: Severity,
        soil_moisture: f64,
        dry_days: usize,
        _profile: &LawnProfile,
    ) -> Recommendation {
        let title = match severity {
            Severity::Critical => "Irrigation Urgently Needed",
            Severity::Warning => "Irrigation Recommended Soon",
            _ => "Consider Irrigation",
        };

        let description = format!(
            "Soil moisture is low ({:.0}%) and no significant rain is forecasted for {} days. \
             Cool-season grasses need consistent moisture.",
            soil_moisture * 100.0,
            dry_days
        );

        let action = match severity {
            Severity::Critical => {
                "Water immediately. Apply 1-1.5 inches over the next 2-3 days to prevent \
                 drought stress. Water early morning (5-9 AM) to minimize evaporation and disease."
            }
            Severity::Warning => {
                "Plan to irrigate within the next 1-2 days. Apply 0.5-1 inch of water. \
                 Deep, infrequent watering is better than shallow daily watering."
            }
            _ => {
                "Monitor soil moisture and plan irrigation if conditions don't change. \
                 Consider a deep watering session in early morning."
            }
        };

        Recommendation::new(
            "irrigation_forecast",
            RecommendationCategory::Irrigation,
            severity,
            title,
            description,
        )
        .with_explanation(
            "Tall Fescue requires 1-1.5 inches of water per week during the growing season. \
             When soil moisture drops below 10-15% and no rain is expected, supplemental \
             irrigation prevents drought stress, thinning, and weed invasion. Water deeply \
             (to 6 inches) to encourage deep root growth.",
        )
        .with_data_point(
            "Soil Moisture",
            format!("{:.0}%", soil_moisture * 100.0),
            "NOAA USCRN",
        )
        .with_data_point(
            "Dry Days Forecast",
            format!("{} days", dry_days),
            "OpenWeatherMap",
        )
        .with_action(action)
    }
}
