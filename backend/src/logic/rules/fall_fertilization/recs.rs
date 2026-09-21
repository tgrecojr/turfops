//! The three fall feedings' recommendation text.

use crate::logic::rules::thresholds::*;
use crate::models::{
    DataSource, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory, Severity,
};

pub(super) fn build_early_fall_rec(
    soil_temp: f64,
    profile: &LawnProfile,
    env: &EnvironmentalSummary,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);
    let n_needed = lawn_size / 1000.0 * EARLY_FALL_N_RATE_LBS_PER_KSQFT;

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
         (K-State Extension, Missouri Extension g6705). Apply {:.1} lb N per 1000 sqft \
         using quick-release or balanced nitrogen. The grass is recovering from summer \
         stress, roots are actively growing, and this feeding drives fall tillering and \
         carbohydrate storage. Recommended NPK ratios: 30-0-0, 29-5-4, 27-3-3, or \
         any 3:1:1 / 4:1:2 ratio.",
        EARLY_FALL_N_RATE_LBS_PER_KSQFT
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
        "Rate",
        format!("{:.1} lb N/1000sqft", EARLY_FALL_N_RATE_LBS_PER_KSQFT),
        "K-State / Missouri Extension",
    );

    if let Some(trend) = Some(&env.soil_temp_trend) {
        rec = rec.with_data_point("Trend", trend.as_str(), DataSource::Calculated.as_str());
    }

    rec = rec.with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn ({:.1} lb N/1000 sqft). \
         Use quick-release or balanced nitrogen (K-State recommends quick-release for fall). \
         Good NPK choices: 30-0-0, 29-5-4, 27-3-3, or any 3:1:1 / 4:1:2 ratio. \
         Water in lightly if no rain expected.",
        n_needed, lawn_size, EARLY_FALL_N_RATE_LBS_PER_KSQFT
    ));

    rec
}

pub(super) fn build_mid_fall_rec(
    soil_temp: f64,
    app_count: usize,
    profile: &LawnProfile,
    env: &EnvironmentalSummary,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);
    let n_needed = lawn_size / 1000.0 * MID_FALL_N_RATE_LBS_PER_KSQFT;

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
        "Fall Apps So Far",
        format!("{}", app_count),
        DataSource::History.as_str(),
    );

    if let Some(trend) = Some(&env.soil_temp_trend) {
        rec = rec.with_data_point("Trend", trend.as_str(), DataSource::Calculated.as_str());
    }

    rec = rec.with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn ({:.2} lb N/1000 sqft). \
         A slow-release or balanced fertilizer works well. \
         This is the most important feeding of the year - don't skip it!",
        n_needed, lawn_size, MID_FALL_N_RATE_LBS_PER_KSQFT
    ));

    rec
}

pub(super) fn build_late_fall_rec(
    soil_temp: f64,
    app_count: usize,
    profile: &LawnProfile,
) -> Recommendation {
    let lawn_size = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);
    let n_needed = lawn_size / 1000.0 * WINTERIZER_N_RATE_LBS_PER_KSQFT;

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
        "Fall Apps So Far",
        format!("{}", app_count),
        DataSource::History.as_str(),
    )
    .with_action(format!(
        "Apply ~{:.1} lbs of nitrogen for your {:.0} sqft lawn ({:.1} lb N/1000 sqft). \
         Quick-release nitrogen is fine for winterizer since you want immediate uptake. \
         Apply before ground freezes, even if grass looks dormant.",
        n_needed, lawn_size, WINTERIZER_N_RATE_LBS_PER_KSQFT
    ))
}
