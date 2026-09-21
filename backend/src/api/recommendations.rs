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
        let mut service = state.sync_service.write().await;
        service.get_or_refresh().await?
    };

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    Ok(Json(
        super::recommendation_feed::active(&state, &profile, &summary).await?,
    ))
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
