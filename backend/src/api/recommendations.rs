use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{Recommendation, Severity};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListRecommendationsQuery {
    /// Also return recommendations the user has answered (flagged `dismissed` /
    /// `addressed`), so the UI can list them and offer Restore.
    #[serde(default)]
    pub include_inactive: bool,
}

/// GET /api/v1/recommendations
/// Evaluates all rules against current environmental data and application history.
/// Returns active recommendations; `?include_inactive=true` adds the answered ones.
pub async fn list_recommendations(
    State(state): State<AppState>,
    Query(params): Query<ListRecommendationsQuery>,
) -> Result<Json<Vec<Recommendation>>, TurfOpsError> {
    // Get current environmental data (refreshes if stale)
    let summary = {
        let mut service = state.sync_service.write().await;
        service.get_or_refresh().await?
    };

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let recommendations = if params.include_inactive {
        super::recommendation_feed::all(&state, &profile, &summary).await?
    } else {
        super::recommendation_feed::active(&state, &profile, &summary).await?
    };
    Ok(Json(recommendations))
}

#[derive(Debug, Deserialize)]
pub struct PatchRecommendationRequest {
    pub dismissed: Option<bool>,
    pub addressed: Option<bool>,
    /// Severity the recommendation showed when the user clicked; an escalation past it
    /// brings the alert back.
    pub severity: Option<Severity>,
}

/// PATCH /api/v1/recommendations/:id
/// Mark a recommendation as dismissed or addressed, or clear both to restore it.
pub async fn patch_recommendation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PatchRecommendationRequest>,
) -> Result<Json<serde_json::Value>, TurfOpsError> {
    let rec_states = queries::get_recommendation_states(&state.pool).await?;
    let existing = rec_states.get(&id);
    let dismissed = req
        .dismissed
        .unwrap_or_else(|| existing.is_some_and(|s| s.dismissed));
    let addressed = req
        .addressed
        .unwrap_or_else(|| existing.is_some_and(|s| s.addressed));

    if dismissed || addressed {
        let severity = req.severity.or_else(|| existing.and_then(|s| s.severity));
        queries::upsert_recommendation_state(&state.pool, &id, dismissed, addressed, severity)
            .await?;
    } else {
        queries::delete_recommendation_state(&state.pool, &id).await?;
    }

    Ok(Json(serde_json::json!({
        "id": id,
        "dismissed": dismissed,
        "addressed": addressed,
    })))
}
