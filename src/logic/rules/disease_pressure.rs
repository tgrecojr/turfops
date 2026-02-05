use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};

/// Disease pressure forecast rule - predicts elevated fungal disease risk
///
/// Target diseases for cool-season turf:
/// - Brown patch (Rhizoctonia solani)
/// - Dollar spot (Clarireedia jacksonii)
/// - Pythium blight
///
/// Risk factors:
/// - Sustained humidity >80% for 2+ consecutive days
/// - Night temps staying >65°F (brown patch trigger)
/// - Day temps 75-90°F with high humidity
/// - Rain followed by warm humid conditions
///
/// Severity levels:
/// - Advisory: 1-2 days of disease-favorable conditions ahead
/// - Warning: 3-4 days of sustained risk conditions
/// - Critical: 5+ days of high-risk forecast OR current + forecast both high-risk
pub struct DiseasePressureRule;

impl Rule for DiseasePressureRule {
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

        // Assess current conditions
        let current_risk = self.assess_current_risk(env);

        // Count consecutive high-risk days in forecast
        let high_humidity_days = forecast.consecutive_high_humidity_days(80.0);

        // Assess forecast disease risk
        let forecast_risk = self.assess_forecast_risk(env);

        // Determine if we should alert
        let combined_risk = current_risk.saturating_add(forecast_risk);

        if combined_risk < 2 {
            return None;
        }

        let severity = if combined_risk >= 5 || (current_risk >= 2 && forecast_risk >= 3) {
            Severity::Critical
        } else if combined_risk >= 3 {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        let disease_type = self.identify_likely_disease(env);

        Some(self.build_recommendation(
            severity,
            &disease_type,
            high_humidity_days,
            current_risk > 0,
            env,
        ))
    }
}

impl DiseasePressureRule {
    fn assess_current_risk(&self, env: &EnvironmentalSummary) -> u32 {
        let mut risk = 0;

        if let Some(current) = &env.current {
            // High current humidity
            if let Some(humidity) = current.humidity_percent {
                if humidity >= 90.0 {
                    risk += 2;
                } else if humidity >= 80.0 {
                    risk += 1;
                }
            }

            // Warm temps with humidity
            if let Some(temp) = current.ambient_temp_f {
                if (75.0..=90.0).contains(&temp) {
                    risk += 1;
                }
            }
        }

        // Recent precipitation + current humidity
        if let Some(precip) = env.precipitation_7day_total_mm {
            if precip > 25.0 {
                // >1 inch in 7 days
                risk += 1;
            }
        }

        // Sustained high humidity
        if let Some(avg_humidity) = env.humidity_7day_avg {
            if avg_humidity >= 80.0 {
                risk += 1;
            }
        }

        risk
    }

    fn assess_forecast_risk(&self, env: &EnvironmentalSummary) -> u32 {
        let forecast = match &env.forecast {
            Some(f) => f,
            None => return 0,
        };

        let mut risk = 0;

        // Check each day for disease-favorable conditions
        for day in forecast.next_days(5) {
            let high_humidity = day.avg_humidity >= 80.0;
            let warm_nights = day.low_temp_f >= 65.0;
            let warm_days = day.high_temp_f >= 75.0 && day.high_temp_f <= 90.0;
            let has_rain = day.total_precipitation_mm >= 2.5 || day.max_precipitation_prob >= 0.5;

            // Brown patch conditions: warm nights + high humidity
            if warm_nights && high_humidity {
                risk += 1;
            }

            // Dollar spot conditions: warm days + humidity + dew
            if warm_days && high_humidity {
                risk += 1;
            }

            // Rain followed by warmth increases risk
            if has_rain && warm_days {
                risk += 1;
            }
        }

        // Cap at reasonable level
        risk.min(6)
    }

    fn identify_likely_disease(&self, env: &EnvironmentalSummary) -> String {
        let current = env.current.as_ref();
        let forecast = env.forecast.as_ref();

        let warm_nights = forecast
            .map(|f| f.next_days(3).iter().any(|d| d.low_temp_f >= 68.0))
            .unwrap_or(false);

        let very_warm_days = forecast
            .map(|f| f.next_days(3).iter().any(|d| d.high_temp_f >= 85.0))
            .unwrap_or(false);

        let current_humid = current
            .and_then(|c| c.humidity_percent)
            .map(|h| h >= 85.0)
            .unwrap_or(false);

        if warm_nights && very_warm_days && current_humid {
            "Brown Patch".to_string()
        } else if warm_nights && !very_warm_days {
            "Dollar Spot".to_string()
        } else {
            "Fungal Disease".to_string()
        }
    }

    fn build_recommendation(
        &self,
        severity: Severity,
        disease_type: &str,
        humid_days: u32,
        current_conditions_bad: bool,
        env: &EnvironmentalSummary,
    ) -> Recommendation {
        let title = match severity {
            Severity::Critical => format!("High {} Risk - Act Now", disease_type),
            Severity::Warning => format!("{} Risk Elevated", disease_type),
            _ => format!("{} Conditions Developing", disease_type),
        };

        let current_note = if current_conditions_bad {
            "Current conditions already favor disease. "
        } else {
            ""
        };

        let description = format!(
            "{}Forecast shows {} days of disease-favorable conditions (high humidity, warm temps). \
             {} thrives in these conditions.",
            current_note, humid_days, disease_type
        );

        let action = match severity {
            Severity::Critical => {
                "Apply preventative fungicide immediately (azoxystrobin, propiconazole). \
                 Water ONLY in early morning (5-7 AM). Avoid evening irrigation. \
                 Reduce nitrogen. Monitor for circular brown patches."
            }
            Severity::Warning => {
                "Consider preventative fungicide if lawn is valuable or has history of disease. \
                 Switch to early morning watering only. Avoid high-nitrogen fertilizer. \
                 Inspect lawn for early symptoms."
            }
            _ => {
                "Monitor conditions. Water early morning only. \
                 Prepare fungicide for application if conditions worsen. \
                 Avoid fertilizer during high-risk period."
            }
        };

        let mut rec = Recommendation::new(
            "disease_pressure_forecast",
            RecommendationCategory::DiseasePressure,
            severity,
            title,
            description,
        )
        .with_explanation(format!(
            "{} is caused by fungal pathogens that thrive in warm, humid conditions. \
             Night temperatures above 65°F combined with humidity above 80% create ideal \
             infection conditions. Symptoms include circular patches of tan/brown turf, \
             often with a darker 'smoke ring' border visible in early morning dew. \
             Preventative fungicide is more effective than curative treatment.",
            disease_type
        ))
        .with_action(action);

        // Add relevant data points
        if let Some(humidity) = env.current.as_ref().and_then(|c| c.humidity_percent) {
            rec = rec.with_data_point(
                "Current Humidity",
                format!("{:.0}%", humidity),
                "Patio Sensor",
            );
        }

        rec = rec.with_data_point(
            "High-Risk Days",
            format!("{}", humid_days),
            "OpenWeatherMap",
        );

        if let Some(avg_humidity) = env.humidity_7day_avg {
            rec = rec.with_data_point(
                "7-Day Avg Humidity",
                format!("{:.0}%", avg_humidity),
                "Calculated",
            );
        }

        rec
    }
}
