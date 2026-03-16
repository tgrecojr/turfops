use super::Rule;
use crate::models::soil_temp_prediction::CrossingDirection;
use crate::models::{
    Application, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};

/// Proactive soil temperature forecast rule.
/// Emits recommendations when predicted threshold crossings are approaching.
pub struct SoilTempForecastRule;

impl Rule for SoilTempForecastRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        _profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        let crossings = env.predicted_threshold_crossings.as_ref()?;

        // Find the most actionable upcoming crossing
        let crossing = crossings
            .iter()
            .filter(|c| c.days_until_crossing > 0 && c.days_until_crossing <= 7)
            .min_by_key(|c| c.days_until_crossing)?;

        let days = crossing.days_until_crossing;
        let direction_str = match crossing.direction {
            CrossingDirection::Rising => "rising toward",
            CrossingDirection::Falling => "dropping below",
        };

        let (severity, urgency) = if days <= 2 {
            (Severity::Advisory, "imminent")
        } else if days <= 5 {
            (Severity::Info, "approaching")
        } else {
            (Severity::Info, "on the horizon")
        };

        let title = format!("{} — {} days away", crossing.threshold_name, days);

        let description = format!(
            "Soil temperature is {} {:.0}°F. Predicted to cross this threshold around {}.",
            direction_str,
            crossing.threshold_temp_f,
            crossing.estimated_crossing_date.format("%B %-d"),
        );

        let explanation = format!(
            "Based on a rolling 30-day regression model correlating air and soil temperatures. \
             The {:.0}°F threshold is significant for: {}. \
             Confidence: {} (model R²: {:.2}).",
            crossing.threshold_temp_f,
            threshold_significance(&crossing.threshold_name),
            crossing.confidence,
            env.soil_temp_predictions
                .as_ref()
                .and_then(|p| p.first())
                .map(|_| "see model info")
                .unwrap_or("N/A"),
        );

        let action = threshold_action(&crossing.threshold_name, &crossing.direction);

        let current_soil = env
            .current
            .as_ref()
            .and_then(|c| c.soil_temp_10_f)
            .map(|t| format!("{:.1}°F", t))
            .unwrap_or_else(|| "--".to_string());

        let rec = Recommendation::new(
            format!(
                "soil_forecast_{}_{}",
                crossing.threshold_temp_f as i32, days
            ),
            RecommendationCategory::SoilTempForecast,
            severity,
            title,
            description,
        )
        .with_explanation(explanation)
        .with_data_point(
            "Current Soil Temp (10cm)",
            &current_soil,
            DataSource::SoilData.as_str(),
        )
        .with_data_point(
            "Predicted Crossing",
            format!(
                "{:.0}°F in ~{} days ({})",
                crossing.threshold_temp_f, days, urgency
            ),
            DataSource::Calculated.as_str(),
        )
        .with_data_point(
            "Confidence",
            crossing.confidence.as_str(),
            DataSource::Calculated.as_str(),
        );

        let rec = if let Some(action) = action {
            rec.with_action(action)
        } else {
            rec
        };

        Some(rec)
    }
}

fn threshold_significance(name: &str) -> &'static str {
    match name {
        "Pre-Emergent Window" => "crabgrass prevention timing",
        "Crabgrass Germination" => "crabgrass germination begins at 55°F sustained",
        "Grub Control Window" => "grub egg-laying and optimal control timing",
        "Active Growth Peak" => "peak cool-season grass growth and overseeding",
        "Heat Stress Risk" => "cool-season grass heat stress and dormancy",
        "Winterizer / Dormancy" => "final fertilization window before dormancy",
        _ => "agronomic activity timing",
    }
}

fn threshold_action(name: &str, direction: &CrossingDirection) -> Option<String> {
    match (name, direction) {
        ("Pre-Emergent Window", CrossingDirection::Rising) => Some(
            "Plan pre-emergent herbicide application. Apply before soil reaches 55°F sustained."
                .to_string(),
        ),
        ("Crabgrass Germination", CrossingDirection::Rising) => Some(
            "If pre-emergent not yet applied, do so immediately. Crabgrass germination is imminent."
                .to_string(),
        ),
        ("Grub Control Window", CrossingDirection::Rising) => Some(
            "Monitor for grub activity. Plan preventive grub control application."
                .to_string(),
        ),
        ("Active Growth Peak", CrossingDirection::Falling) => Some(
            "Consider fall overseeding. Soil temps 60-65°F are ideal for seed germination."
                .to_string(),
        ),
        ("Heat Stress Risk", CrossingDirection::Rising) => Some(
            "Raise mowing height and reduce nitrogen. Avoid heavy fertilization during heat stress."
                .to_string(),
        ),
        ("Winterizer / Dormancy", CrossingDirection::Falling) => Some(
            "Apply winterizer fertilizer before soil drops below 45°F and grass enters dormancy."
                .to_string(),
        ),
        _ => None,
    }
}
