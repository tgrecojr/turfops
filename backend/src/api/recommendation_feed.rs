//! The recommendation feed. The dashboard's "Active Alerts" and the Recommendations page
//! both come from [`active`], so the top 3 on the dashboard are always a subset of the feed.

use crate::db::{plant_queries, queries, soil_test_queries};
use crate::error::TurfOpsError;
use crate::logic::follow_up::generate_follow_up_recommendations;
use crate::logic::plant_maintenance::generate_plant_maintenance_recommendations;
use crate::logic::soil_test_recommendations::generate_soil_test_recommendations;
use crate::models::{
    Application, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity, SoilTest,
};
use crate::state::AppState;
use chrono::{Datelike, Local};

/// How much application history the rules see. Mowing is an application, so a short window
/// (the dashboard once used 10 rows) drops this season's treatments within weeks.
const RULE_HISTORY_LIMIT: i64 = 1000;

/// Every active (not dismissed/addressed) recommendation, in source order.
pub async fn active(
    state: &AppState,
    profile: &LawnProfile,
    summary: &EnvironmentalSummary,
) -> Result<Vec<Recommendation>, TurfOpsError> {
    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let apps =
        queries::get_applications_for_profile(&state.pool, profile_id, RULE_HISTORY_LIMIT, 0)
            .await?;

    let mut recommendations = state.rules_engine.evaluate(summary, profile, &apps);

    // Per-disease risk (High/Severe tiers) and the seeding / pre-emergent windows.
    recommendations.extend(super::disease_risk::recommendations(state, profile).await);
    recommendations.extend(super::timing::recommendations(state, profile).await);

    // Plant maintenance for landscape plants.
    let plants = plant_queries::list_plants_for_profile(&state.pool, profile_id).await?;
    let today = Local::now().date_naive();
    recommendations.extend(generate_plant_maintenance_recommendations(
        &plants, &apps, today,
    ));

    // Follow-up reminders for applications that scheduled one.
    let plants_by_id: std::collections::HashMap<i64, &crate::models::plant::Plant> = plants
        .iter()
        .filter_map(|p| p.id.map(|id| (id, p)))
        .collect();
    recommendations.extend(generate_follow_up_recommendations(
        &apps,
        &plants_by_id,
        today,
    ));

    // Soil-test recommendations. The nitrogen budget is annual, so only this year's
    // applications count against it (same as /soil-tests/recommendations).
    if let Ok(Some(test)) = soil_test_queries::get_latest_soil_test(&state.pool, profile_id).await {
        let this_year: Vec<Application> = apps
            .iter()
            .filter(|a| a.application_date.year() == today.year())
            .cloned()
            .collect();
        recommendations.extend(soil_test_recommendations(&test, profile, &this_year));
    }

    // Apply dismissed/addressed state from database
    let rec_states = queries::get_recommendation_states(&state.pool).await?;
    for rec in &mut recommendations {
        if let Some((dismissed, addressed)) = rec_states.get(&rec.id) {
            rec.dismissed = *dismissed;
            rec.addressed = *addressed;
        }
    }
    recommendations.retain(|r| r.is_active());

    Ok(recommendations)
}

fn soil_test_recommendations(
    test: &SoilTest,
    profile: &LawnProfile,
    apps: &[Application],
) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();
    let soil_summary = generate_soil_test_recommendations(test, profile, apps);

    if let Some(ph_rec) = &soil_summary.ph_recommendation {
        recommendations.push(
            Recommendation::new(
                "soil_test_ph",
                RecommendationCategory::SoilTest,
                if (ph_rec.current_ph - ph_rec.target_ph).abs() > 1.0 {
                    Severity::Warning
                } else {
                    Severity::Advisory
                },
                format!("pH Adjustment: Apply {}", ph_rec.amendment),
                &ph_rec.explanation,
            )
            .with_data_point(
                "Current pH",
                format!("{:.1}", ph_rec.current_ph),
                DataSource::SoilTestData.as_str(),
            )
            .with_data_point(
                "Target pH",
                format!("{:.1}", ph_rec.target_ph),
                DataSource::Agronomic.as_str(),
            )
            .with_action(format!(
                "Apply {} at {:.0} lbs/1000 sqft",
                ph_rec.amendment, ph_rec.rate_lbs_per_1000sqft
            )),
        );
    }

    if let Some(npk_rec) = &soil_summary.npk_recommendation {
        if npk_rec.nitrogen_rate_lbs_per_1000sqft > 0.0
            || npk_rec.phosphorus_rate_lbs_per_1000sqft > 0.0
            || npk_rec.potassium_rate_lbs_per_1000sqft > 0.0
        {
            recommendations.push(
                Recommendation::new(
                    "soil_test_npk",
                    RecommendationCategory::SoilTest,
                    Severity::Advisory,
                    format!("Fertilizer: Use {} ratio", npk_rec.recommended_ratio),
                    &npk_rec.explanation,
                )
                .with_data_point(
                    "N rate",
                    format!("{:.2} lbs/1000sqft", npk_rec.nitrogen_rate_lbs_per_1000sqft),
                    DataSource::SoilTestData.as_str(),
                )
                .with_data_point(
                    "P₂O₅ rate",
                    format!(
                        "{:.2} lbs/1000sqft",
                        npk_rec.phosphorus_rate_lbs_per_1000sqft
                    ),
                    DataSource::SoilTestData.as_str(),
                )
                .with_data_point(
                    "K₂O rate",
                    format!(
                        "{:.2} lbs/1000sqft",
                        npk_rec.potassium_rate_lbs_per_1000sqft
                    ),
                    DataSource::SoilTestData.as_str(),
                )
                .with_data_point(
                    "N budget remaining",
                    format!(
                        "{:.2} lbs/1000sqft",
                        npk_rec.remaining_n_budget_lbs_per_1000sqft
                    ),
                    DataSource::Calculated.as_str(),
                )
                .with_action(format!(
                    "Apply {} product at {:.1} lbs/1000 sqft",
                    npk_rec.example_product_ratio, npk_rec.product_rate_lbs_per_1000sqft
                )),
            );
        }
    }

    for micro in &soil_summary.micronutrient_recommendations {
        recommendations.push(
            Recommendation::new(
                format!("soil_test_micro_{}", micro.nutrient.to_lowercase()),
                RecommendationCategory::SoilTest,
                Severity::Info,
                format!("{} Deficiency Detected", micro.nutrient),
                &micro.suggestion,
            )
            .with_data_point(
                &format!("{} (ppm)", micro.nutrient),
                format!("{:.1}", micro.current_ppm),
                DataSource::SoilTestData.as_str(),
            )
            .with_data_point(
                "Threshold (ppm)",
                format!("{:.1}", micro.threshold_ppm),
                DataSource::Agronomic.as_str(),
            )
            .with_action(&micro.suggestion),
        );
    }

    recommendations
}
