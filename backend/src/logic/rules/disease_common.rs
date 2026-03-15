use crate::models::{
    Application, ApplicationType, DataSource, FracClass, FungicideRotationAdvice, Recommendation,
};
use chrono::Local;

/// Build FRAC rotation guidance text from rotation analysis.
pub fn build_rotation_guidance(advice: &FungicideRotationAdvice) -> String {
    if let Some(next) = &advice.recommended_next {
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
    }
}

/// Append FRAC rotation warning to an action string if applicable.
pub fn append_rotation_warning(action: &str, advice: &FungicideRotationAdvice) -> String {
    if let Some(warning) = &advice.rotation_warning {
        format!("{} ROTATION WARNING: {}", action, warning)
    } else {
        action.to_string()
    }
}

/// Add FRAC rotation data points (last class + recommended next) to a recommendation.
pub fn add_frac_data_points(
    mut rec: Recommendation,
    advice: &FungicideRotationAdvice,
) -> Recommendation {
    if let Some(last) = &advice.last_class {
        rec = rec.with_data_point(
            "Last FRAC Class",
            last.to_string(),
            DataSource::History.as_str(),
        );
    }
    if let Some(next) = &advice.recommended_next {
        rec = rec.with_data_point(
            "Recommended Next",
            next.to_string(),
            DataSource::Rotation.as_str(),
        );
    }
    rec
}

/// Build a FRAC-aware fungicide recommendation for gray leaf spot.
/// Default is FRAC 11, but rotates if user recently used FRAC 11.
pub fn gray_leaf_spot_fungicide_rec(advice: &FungicideRotationAdvice) -> String {
    if advice.last_class == Some(FracClass::Frac11) {
        let next = advice.recommended_next.unwrap_or(FracClass::Frac3);
        let products = next.common_products();
        let example = products.first().copied().unwrap_or("(see label)");
        format!(
            "Recent FRAC 11 usage detected — rotate to {} (e.g., {})",
            next, example
        )
    } else {
        "azoxystrobin or pyraclostrobin (FRAC 11 strobilurins)".to_string()
    }
}

/// Check if lawn is nitrogen-deficient (no fertilizer in the given number of days).
pub fn is_nitrogen_deficient(history: &[Application], days: i64) -> bool {
    let cutoff = Local::now().date_naive() - chrono::Duration::days(days);
    !history.iter().any(|app| {
        app.application_type == ApplicationType::Fertilizer && app.application_date >= cutoff
    })
}
