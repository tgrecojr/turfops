use crate::error::TurfOpsError;
use crate::models::EnvironmentalSummary;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

/// GET /api/v1/environmental
/// Returns environmental data, refreshing from datasources only if stale.
/// Sensors refresh after 5 minutes, forecast after 30 minutes.
pub async fn get_environmental(
    State(state): State<AppState>,
) -> Result<Json<EnvironmentalSummary>, TurfOpsError> {
    let mut service = state.sync_service.write().await;
    let summary = service.get_or_refresh().await?;
    Ok(Json(summary))
}

/// POST /api/v1/environmental/refresh
/// Forces an immediate refresh from all datasources regardless of cache age.
pub async fn refresh_environmental(
    State(state): State<AppState>,
) -> Result<Json<EnvironmentalSummary>, TurfOpsError> {
    let mut service = state.sync_service.write().await;
    let summary = service.force_refresh().await?;
    Ok(Json(summary))
}
