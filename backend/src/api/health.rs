use crate::logic::data_sync::ConnectionStatus;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: bool,
    pub datasources: ConnectionStatus,
    /// `enabled` when `/mcp` is mounted (MCP_TOKEN set), else `disabled`.
    pub mcp: &'static str,
}

// @spec SYNC-CONN-003, SYNC-CONN-004, MCP-CFG-006
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_ok = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query("SELECT 1").fetch_one(&state.pool),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);

    // Check external datasource connectivity via read lock (doesn't block data sync)
    let service = state.sync_service.read().await;
    let datasources = service.check_connections().await;

    Json(HealthResponse {
        status: if db_ok { "ok" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_ok,
        datasources,
        mcp: if state.mcp_enabled {
            "enabled"
        } else {
            "disabled"
        },
    })
}

#[cfg(all(test, feature = "db-tests"))]
mod db_tests;
#[cfg(test)]
mod tests;
