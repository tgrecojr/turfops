use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::Recommendation;
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
        let mut service = state.sync_service.lock().await;
        service.get_or_refresh().await?
    };

    // Get current profile and application history
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let apps = queries::get_applications_for_profile(&state.pool, profile.id.unwrap()).await?;

    // Evaluate rules
    let mut recommendations = state.rules_engine.evaluate(&summary, &profile, &apps);

    // Apply dismissed/addressed state from in-memory tracking
    let rec_states = state.recommendation_states.read().await;
    for rec in &mut recommendations {
        if let Some(action) = rec_states.get(&rec.id) {
            rec.dismissed = action.dismissed;
            rec.addressed = action.addressed;
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
/// Mark a recommendation as dismissed or addressed.
pub async fn patch_recommendation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PatchRecommendationRequest>,
) -> Result<Json<serde_json::Value>, TurfOpsError> {
    let mut rec_states = state.recommendation_states.write().await;
    let entry = rec_states.entry(id.clone()).or_insert(RecommendationState {
        dismissed: false,
        addressed: false,
    });

    if let Some(dismissed) = req.dismissed {
        entry.dismissed = dismissed;
    }
    if let Some(addressed) = req.addressed {
        entry.addressed = addressed;
    }

    Ok(Json(serde_json::json!({
        "id": id,
        "dismissed": entry.dismissed,
        "addressed": entry.addressed,
    })))
}

/// In-memory tracking of recommendation dismissed/addressed state.
/// Re-evaluated recommendations are ephemeral; this persists user actions across
/// re-evaluations within the same server lifetime (matches TUI behavior).
#[derive(Debug, Clone, Default)]
pub struct RecommendationState {
    pub dismissed: bool,
    pub addressed: bool,
}
