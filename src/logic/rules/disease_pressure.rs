use super::Rule;
use crate::models::{
    analyze_fungicide_rotation, Application, ApplicationType, EnvironmentalSummary, LawnProfile,
    Recommendation, RecommendationCategory, Severity,
};
use chrono::Local;

/// Disease pressure forecast rule - predicts elevated fungal disease risk
///
/// Target diseases for cool-season turf:
/// - Brown patch (Rhizoctonia solani): Night >60°F + humidity >80%
/// - Dollar spot (Clarireedia jacksonii): Night >50°F + prolonged leaf wetness + N deficiency
/// - Pythium blight: Night >65°F + day >85°F + 12-14 hrs wetness
///
/// Risk factors (NC State Extension):
/// - Brown patch begins at night temps >60°F, severe at >70°F night + >90°F day
/// - Dollar spot activates at night temps >50°F with 10-12 hrs leaf wetness
/// - Dollar spot is amplified by nitrogen deficiency (no N in 30-45 days)
/// - Pythium follows thunderstorm activity
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
        history: &[Application],
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

        let disease_type = self.identify_likely_disease(env, history);

        Some(self.build_recommendation(
            severity,
            &disease_type,
            high_humidity_days,
            current_risk > 0,
            env,
            history,
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
            // A2: Dollar spot triggers at night temps >=50°F (NC State)
            let dollar_spot_nights = day.low_temp_f >= 50.0;
            // A1: Brown patch triggers at night temps >=60°F (NC State)
            let brown_patch_nights = day.low_temp_f >= 60.0;
            let warm_days = day.high_temp_f >= 75.0 && day.high_temp_f <= 90.0;
            let has_rain = day.total_precipitation_mm >= 2.5 || day.max_precipitation_prob >= 0.5;

            // Brown patch conditions: night >60°F + high humidity
            if brown_patch_nights && high_humidity {
                risk += 1;
            }

            // Dollar spot conditions: night >50°F + humidity (wider window)
            if dollar_spot_nights && high_humidity && !brown_patch_nights {
                // Only add risk if not already counted by brown patch
                risk += 1;
            }

            // Dollar spot also triggers in warm days with humidity
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

    fn identify_likely_disease(
        &self,
        env: &EnvironmentalSummary,
        _history: &[Application],
    ) -> String {
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

        // A2: Check for dollar spot conditions — night >50°F but <68°F
        let cool_nights = forecast
            .map(|f| {
                f.next_days(3)
                    .iter()
                    .any(|d| d.low_temp_f >= 50.0 && d.low_temp_f < 68.0)
            })
            .unwrap_or(false);

        if warm_nights && very_warm_days && current_humid {
            "Brown Patch".to_string()
        } else if (cool_nights && current_humid) || (warm_nights && !very_warm_days) {
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
        history: &[Application],
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

        // C2: Disease-specific nitrogen guidance
        let n_guidance = match disease_type {
            "Brown Patch" => "Limit nitrogen to ≤0.5 lb N/1000sqft (NC State). Excess N fuels brown patch.",
            "Dollar Spot" => "MAINTAIN adequate nitrogen — dollar spot is triggered by nitrogen \
                deficiency (NC State). Apply 0.5-1.0 lb N/1000sqft if not recently fertilized.",
            "Pythium" => "Limit nitrogen to ≤0.25 lb N/1000sqft (NC State). Excessive N increases susceptibility.",
            _ => "Reduce nitrogen applications during high-risk periods.",
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

        let action = match severity {
            Severity::Critical => {
                format!(
                    "Apply preventative fungicide immediately. {} \
                     Water ONLY in early morning (4-7 AM). Avoid evening irrigation. \
                     {} Monitor for symptoms.",
                    rotation_guidance, n_guidance
                )
            }
            Severity::Warning => {
                format!(
                    "Consider preventative fungicide if lawn has disease history. {} \
                     Switch to early morning watering only. {} \
                     Inspect lawn for early symptoms.",
                    rotation_guidance, n_guidance
                )
            }
            _ => {
                format!(
                    "Monitor conditions. Water early morning only. \
                     Prepare fungicide for application if conditions worsen. {}",
                    n_guidance
                )
            }
        };

        let full_action = if let Some(warning) = &advice.rotation_warning {
            format!("{} ROTATION WARNING: {}", action, warning)
        } else {
            action
        };

        let explanation = match disease_type {
            "Brown Patch" =>
                "Brown patch (Rhizoctonia solani) onset begins when nighttime temperatures \
                 exceed 60°F, with severe conditions at night >70°F and day >90°F (NC State Extension). \
                 Humidity above 80% combined with warm nights create ideal infection conditions. \
                 Symptoms include circular patches of tan/brown turf with a darker 'smoke ring' border \
                 visible in early morning dew. Preventative fungicide is more effective than curative."
                .to_string(),
            "Dollar Spot" => {
                let deficiency_note = if is_nitrogen_deficient(history, 45) {
                    " Your lawn has not received nitrogen in 45+ days — this nitrogen deficiency \
                     is a key predisposing factor for dollar spot."
                } else {
                    ""
                };
                format!(
                    "Dollar spot (Clarireedia jacksonii) activates when nighttime temps exceed \
                     50°F with 10-12 hours of leaf wetness (NC State Extension). Unlike brown patch, \
                     dollar spot is amplified by nitrogen DEFICIENCY — maintaining adequate N reduces risk. \
                     Symptoms are small, silver-dollar-sized bleached spots. Disease slows above 90°F.{}",
                    deficiency_note
                )
            }
            _ => format!(
                "{} is caused by fungal pathogens that thrive in warm, humid conditions. \
                 Night temperatures above 60°F combined with humidity above 80% create ideal \
                 infection conditions. Preventative fungicide is more effective than curative treatment.",
                disease_type
            ),
        };

        let mut rec = Recommendation::new(
            "disease_pressure_forecast",
            RecommendationCategory::DiseasePressure,
            severity,
            title,
            description,
        )
        .with_explanation(explanation)
        .with_action(full_action);

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

        // Add dollar spot N-deficiency data point
        if disease_type == "Dollar Spot" && is_nitrogen_deficient(history, 45) {
            rec = rec.with_data_point(
                "N Status",
                "No fertilizer in 45+ days (risk factor)",
                "History",
            );
        }

        // Add FRAC rotation data points
        if let Some(last) = &advice.last_class {
            rec = rec.with_data_point("Last FRAC Class", last.to_string(), "History");
        }
        if let Some(next) = &advice.recommended_next {
            rec = rec.with_data_point("Recommended Next", next.to_string(), "Rotation");
        }

        rec
    }
}

/// Check if lawn is nitrogen-deficient (no fertilizer in N days)
fn is_nitrogen_deficient(history: &[Application], days: i64) -> bool {
    let cutoff = Local::now().date_naive() - chrono::Duration::days(days);
    !history.iter().any(|app| {
        app.application_type == ApplicationType::Fertilizer && app.application_date >= cutoff
    })
}
