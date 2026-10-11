//! The three fall feedings' recommendation text.

use crate::logic::rules::nitrogen_guard::{amount_text, rate_text, RATE_POINT};
use crate::logic::rules::thresholds::*;
use crate::models::{
    DataSource, EnvironmentalSummary, LawnProfile, ProductCategory, ProductNeed, Recommendation,
    RecommendationCategory, Severity,
};

pub(super) fn build_early_fall_rec(
    soil_temp: f64,
    profile: &LawnProfile,
    env: &EnvironmentalSummary,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);

    let mut rec = Recommendation::new(
        "fall_fert_early",
        RecommendationCategory::Fertilizer,
        Severity::Warning, // Elevated — September IS the most important feeding
        "September Fertilization — Most Important Feeding",
        format!(
            "Soil temperature ({:.1}°F) is ideal for fall fertilization. \
             September is THE best time to fertilize cool-season grass.",
            soil_temp
        ),
    )
    .with_explanation(format!(
        "September is the single most important fertilization of the year for TTTF \
         (K-State Extension, Missouri Extension g6705). Apply {} \
         using quick-release or balanced nitrogen. The grass is recovering from summer \
         stress, roots are actively growing, and this feeding drives fall tillering and \
         carbohydrate storage. Recommended NPK ratios: 30-0-0, 29-5-4, 27-3-3, or \
         any 3:1:1 / 4:1:2 ratio.",
        rate_text(EARLY_FALL_N_RATE_LBS_PER_KSQFT)
    ))
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Phase",
        "September (Primary Feeding)",
        DataSource::Calendar.as_str(),
    )
    .with_data_point(
        RATE_POINT,
        rate_text(EARLY_FALL_N_RATE_LBS_PER_KSQFT),
        "K-State / Missouri Extension",
    );

    if let Some(trend) = Some(&env.soil_temp_trend) {
        rec = rec.with_data_point("Trend", trend.as_str(), DataSource::Calculated.as_str());
    }

    rec.with_need(fertilizer_need())
        .with_action(format!(
            "Apply {}. \
             Use quick-release or balanced nitrogen (K-State recommends quick-release for fall). \
             Good NPK choices: 30-0-0, 29-5-4, 27-3-3, or any 3:1:1 / 4:1:2 ratio. \
             Water in lightly if no rain expected.",
            amount_text(EARLY_FALL_N_RATE_LBS_PER_KSQFT, lawn_size)
        ))
        .with_feeding(EARLY_FALL_N_RATE_LBS_PER_KSQFT)
}

pub(super) fn build_mid_fall_rec(
    soil_temp: f64,
    app_count: usize,
    profile: &LawnProfile,
    env: &EnvironmentalSummary,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);

    let severity = if app_count == 0 {
        Severity::Warning // Missed early fall app
    } else {
        Severity::Advisory
    };

    let title = if app_count == 0 {
        "Mid-Fall Fertilization - Don't Miss Fall Feeding!"
    } else {
        "Mid-Fall Fertilization"
    };

    let mut rec = Recommendation::new(
        "fall_fert_mid",
        RecommendationCategory::Fertilizer,
        severity,
        title,
        format!(
            "Prime time for fall fertilization. Soil temp {:.1}°F is optimal \
             for root uptake and carbohydrate storage.",
            soil_temp
        ),
    )
    .with_explanation(
        "Mid-fall (October) is the MOST important fertilization of the year for TTTF. \
         Roots are actively growing while top growth slows. Nitrogen applied now is \
         stored as carbohydrates, fueling winter hardiness and explosive spring green-up. \
         This single application has more impact than any other feeding.",
    )
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point("Phase", "Mid-Fall (Primary)", DataSource::Calendar.as_str())
    .with_data_point(
        RATE_POINT,
        rate_text(MID_FALL_N_RATE_LBS_PER_KSQFT),
        DataSource::Agronomic.as_str(),
    )
    .with_data_point(
        "Fall Apps So Far",
        format!("{}", app_count),
        DataSource::History.as_str(),
    );

    if let Some(trend) = Some(&env.soil_temp_trend) {
        rec = rec.with_data_point("Trend", trend.as_str(), DataSource::Calculated.as_str());
    }

    rec.with_need(fertilizer_need())
        .with_action(format!(
            "Apply {}. \
             A slow-release or balanced fertilizer works well. \
             This is the most important feeding of the year - don't skip it!",
            amount_text(MID_FALL_N_RATE_LBS_PER_KSQFT, lawn_size)
        ))
        .with_feeding(MID_FALL_N_RATE_LBS_PER_KSQFT)
}

pub(super) fn build_late_fall_rec(
    soil_temp: f64,
    app_count: usize,
    profile: &LawnProfile,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);

    let severity = if app_count == 0 {
        Severity::Warning // Missed all fall apps - at least get winterizer
    } else {
        Severity::Advisory
    };

    Recommendation::new(
        "fall_fert_winterizer",
        RecommendationCategory::Fertilizer,
        severity,
        "Winterizer Application",
        format!(
            "Time for final fall fertilization. Soil temp {:.1}°F - grass is slowing \
             but roots are still active.",
            soil_temp
        ),
    )
    .with_explanation(
        "The 'winterizer' application provides nitrogen that the grass stores over winter. \
         Applied when growth has slowed but before the ground freezes, this nitrogen \
         is available immediately when spring arrives, giving you the fastest, greenest \
         spring lawn. Apply even if grass appears dormant - roots are still working.",
    )
    .with_data_point(
        "Soil Temp",
        format!("{:.1}°F", soil_temp),
        DataSource::SoilData.as_str(),
    )
    .with_data_point(
        "Phase",
        "Late Fall (Winterizer)",
        DataSource::Calendar.as_str(),
    )
    .with_data_point(
        RATE_POINT,
        rate_text(WINTERIZER_N_RATE_LBS_PER_KSQFT),
        DataSource::Agronomic.as_str(),
    )
    .with_data_point(
        "Fall Apps So Far",
        format!("{}", app_count),
        DataSource::History.as_str(),
    )
    .with_need(fertilizer_need())
    .with_action(format!(
        "Apply {}. \
         Quick-release nitrogen is fine for winterizer since you want immediate uptake. \
         Apply before ground freezes, even if grass looks dormant.",
        amount_text(WINTERIZER_N_RATE_LBS_PER_KSQFT, lawn_size)
    ))
    .with_feeding(WINTERIZER_N_RATE_LBS_PER_KSQFT)
}

// @spec RULE-GUARD-002, RULE-FALL-007
/// The need of a feeding: a fertilizer that carries nitrogen.
fn fertilizer_need() -> ProductNeed {
    ProductNeed::new("nitrogen fertilizer", ProductCategory::Fertilizer).requiring_nitrogen()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GrassType;

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

    // @spec RULE-GUARD-002, RULE-FALL-007
    #[test]
    fn every_fall_feeding_is_marked_and_needs_nitrogen() {
        let env = EnvironmentalSummary::default();
        for (rec, rate) in [
            (
                build_early_fall_rec(60.0, &profile(), &env),
                EARLY_FALL_N_RATE_LBS_PER_KSQFT,
            ),
            (
                build_mid_fall_rec(55.0, 0, &profile(), &env),
                MID_FALL_N_RATE_LBS_PER_KSQFT,
            ),
            (
                build_late_fall_rec(45.0, 0, &profile()),
                WINTERIZER_N_RATE_LBS_PER_KSQFT,
            ),
        ] {
            assert_eq!(
                rec.feeding.map(|f| f.n_lbs_per_ksqft),
                Some(rate),
                "{}",
                rec.id
            );
            assert!(!rec.needs.is_empty() && rec.needs.iter().all(|n| n.requires_nitrogen));
        }
    }
}
