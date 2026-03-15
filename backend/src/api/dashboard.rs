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

    let apps = queries::get_applications_for_profile(&state.pool, profile.id.unwrap()).await?;

    // Get environmental data (refreshes if stale)
    let (summary, connections) = {
        let mut service = state.sync_service.lock().await;
        let summary = service.get_or_refresh().await?;
        let connections = service.check_connections().await;
        (summary, connections)
    };

    // Evaluate rules for recommendations
    let mut recommendations = state.rules_engine.evaluate(&summary, &profile, &apps);

    // Apply dismissed/addressed state
    let rec_states = state.recommendation_states.read().await;
    for rec in &mut recommendations {
        if let Some(action) = rec_states.get(&rec.id) {
            rec.dismissed = action.dismissed;
            rec.addressed = action.addressed;
        }
    }
    recommendations.retain(|r| r.is_active());

    // Top 3 recommendations by severity
    recommendations.sort_by(|a, b| b.severity.cmp(&a.severity));
    recommendations.truncate(3);

    // 5 most recent applications
    let recent_applications: Vec<Application> = apps.into_iter().take(5).collect();

    Ok(Json(DashboardResponse {
        profile,
        environmental: summary,
        recommendations,
        recent_applications,
        connections,
    }))
}
