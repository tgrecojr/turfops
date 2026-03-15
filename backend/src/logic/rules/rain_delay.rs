use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};

/// Rain delay rule - blocks fertilizer/herbicide applications when rain is imminent
///
/// Conditions:
/// - Rain forecasted (>0.1" or >50% probability) within 24-48 hours
///
/// Severity levels:
/// - Advisory: Rain in 24-48h with <50% probability
/// - Warning: Rain in 24h with >50% probability
/// - Critical: Rain in 12h with >70% probability
pub struct RainDelayRule;

impl Rule for RainDelayRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        _profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        let forecast = env.forecast.as_ref()?;

        // Check for rain in next 12 hours (critical)
        if let Some(rain_12h) = forecast.rain_expected_within(12, 0.1) {
            if rain_12h.max_probability >= 0.7 || rain_12h.expected_mm >= 2.5 {
                return Some(self.build_recommendation(
                    Severity::Critical,
                    12,
                    rain_12h.expected_mm,
                    rain_12h.max_probability,
                ));
            }
        }

        // Check for rain in next 24 hours (warning)
        if let Some(rain_24h) = forecast.rain_expected_within(24, 0.1) {
            if rain_24h.max_probability >= 0.5 || rain_24h.expected_mm >= 2.5 {
                return Some(self.build_recommendation(
                    Severity::Warning,
                    24,
                    rain_24h.expected_mm,
                    rain_24h.max_probability,
                ));
            }
        }

        // Check for rain in next 48 hours (advisory)
        if let Some(rain_48h) = forecast.rain_expected_within(48, 0.1) {
            if rain_48h.max_probability >= 0.3 || rain_48h.expected_mm >= 5.0 {
                return Some(self.build_recommendation(
                    Severity::Advisory,
                    48,
                    rain_48h.expected_mm,
                    rain_48h.max_probability,
                ));
            }
        }

        None
    }
}

impl RainDelayRule {
    fn build_recommendation(
        &self,
        severity: Severity,
        hours: u32,
        expected_mm: f64,
        probability: f64,
    ) -> Recommendation {
        let expected_inches = expected_mm / 25.4;
        let prob_percent = probability * 100.0;

        let title = match severity {
            Severity::Critical => "Rain Imminent - Delay Applications",
            Severity::Warning => "Rain Expected - Plan Applications Carefully",
            _ => "Rain in Forecast - Consider Timing",
        };

        let description = format!(
            "Rain expected within {} hours: {:.2}\" ({:.0}% probability). \
             Fertilizer and herbicide applications should be delayed.",
            hours, expected_inches, prob_percent
        );

        let action = match severity {
            Severity::Critical => {
                "Do NOT apply fertilizer, herbicide, or fungicide. \
                 Wait for dry conditions and at least 24 hours of no rain forecast."
            }
            Severity::Warning => {
                "Delay chemical applications if possible. \
                 If application is critical, ensure product has time to dry (4-6 hours)."
            }
            _ => {
                "Monitor forecast before planning applications. \
                 Consider applying in early morning if afternoon rain expected."
            }
        };

        Recommendation::new(
            "rain_delay",
            RecommendationCategory::ApplicationTiming,
            severity,
            title,
            description,
        )
        .with_explanation(
            "Chemical lawn products (fertilizers, herbicides, fungicides) need time to be \
             absorbed by plants or soil before rain. Rain within 24-48 hours of application \
             can wash products away, reducing effectiveness and potentially polluting waterways.",
        )
        .with_data_point(
            "Expected Rain",
            format!("{:.2}\"", expected_inches),
            "OpenWeatherMap",
        )
        .with_data_point(
            "Rain Probability",
            format!("{:.0}%", prob_percent),
            "OpenWeatherMap",
        )
        .with_data_point("Forecast Window", format!("{}h", hours), "OpenWeatherMap")
        .with_action(action)
    }
}
