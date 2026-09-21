use crate::db::queries;
use crate::error::TurfOpsError;
use crate::logic::data_sync::ConnectionStatus;
use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub profile: LawnProfile,
    pub environmental: EnvironmentalSummary,
    pub recommendations: Vec<Recommendation>,
    pub recent_applications: Vec<Application>,
    pub connections: ConnectionStatus,
}

/// GET /api/v1/dashboard
/// Composite endpoint returning profile, environmental summary, top recommendations,
/// recent applications, and datasource connection status.
pub async fn get_dashboard(
    State(state): State<AppState>,
) -> Result<Json<DashboardResponse>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let recent_applications: Vec<Application> =
        queries::get_applications_for_profile(&state.pool, profile_id, 5, 0).await?;

    // Get environmental data (refreshes if stale)
    let summary = {
        let mut service = state.sync_service.write().await;
        service.get_or_refresh().await?
    };
    // Check connections with read lock (doesn't block other readers)
    let connections = {
        let service = state.sync_service.read().await;
        service.check_connections().await
    };

    // The same feed as the Recommendations page, so the two can never disagree.
    let mut recommendations =
        super::recommendation_feed::active(&state, &profile, &summary).await?;

    // Top 3 recommendations by severity
    recommendations.sort_by_key(|r| std::cmp::Reverse(r.severity));
    recommendations.truncate(3);

    Ok(Json(DashboardResponse {
        profile,
        environmental: summary,
        recommendations,
        recent_applications,
        connections,
    }))
}
