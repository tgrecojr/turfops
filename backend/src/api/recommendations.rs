use crate::db::{queries, soil_test_queries};
use crate::error::TurfOpsError;
use crate::logic::soil_test_recommendations::generate_soil_test_recommendations;
use crate::models::{DataSource, Recommendation, RecommendationCategory, Severity};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

/// GET /api/v1/recommendations
/// Evaluates all rules against current environmental data and application history.
/// Returns active recommendations (not dismissed/addressed).
pub async fn list_recommendations(
    State(state): State<AppState>,
) -> Result<Json<Vec<Recommendation>>, TurfOpsError> {
    // Get current environmental data (refreshes if stale)
    let summary = {
        let mut service = state.sync_service.write().await;
        service.get_or_refresh().await?
    };

    // Get current profile and application history
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let apps = queries::get_applications_for_profile(&state.pool, profile_id, 1000, 0).await?;

    // Evaluate rules
    let mut recommendations = state.rules_engine.evaluate(&summary, &profile, &apps);

    // Append soil-test-based recommendations if a test exists
    if let Ok(Some(test)) = soil_test_queries::get_latest_soil_test(&state.pool, profile_id).await {
        let soil_summary = generate_soil_test_recommendations(&test, &profile, &apps);

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
    }

    // Apply dismissed/addressed state from database
    let rec_states = queries::get_recommendation_states(&state.pool).await?;
    for rec in &mut recommendations {
        if let Some((dismissed, addressed)) = rec_states.get(&rec.id) {
            rec.dismissed = *dismissed;
            rec.addressed = *addressed;
        }
    }

    // Return only active recommendations
    recommendations.retain(|r| r.is_active());

    Ok(Json(recommendations))
}

#[derive(Debug, Deserialize)]
pub struct PatchRecommendationRequest {
    pub dismissed: Option<bool>,
    pub addressed: Option<bool>,
}

/// PATCH /api/v1/recommendations/:id
/// Mark a recommendation as dismissed or addressed. Persisted to database.
pub async fn patch_recommendation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PatchRecommendationRequest>,
) -> Result<Json<serde_json::Value>, TurfOpsError> {
    // Get existing state from DB
    let rec_states = queries::get_recommendation_states(&state.pool).await?;
    let (mut dismissed, mut addressed) = rec_states.get(&id).copied().unwrap_or((false, false));

    if let Some(d) = req.dismissed {
        dismissed = d;
    }
    if let Some(a) = req.addressed {
        addressed = a;
    }

    queries::upsert_recommendation_state(&state.pool, &id, dismissed, addressed).await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "dismissed": dismissed,
        "addressed": addressed,
    })))
}
