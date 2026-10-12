//! Connection status on the dashboard and in the MCP snapshot (`--features db-tests`).

use super::test_env::*;
use crate::db::queries;
use crate::models::{GrassType, LawnProfile};
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Request};
use sqlx::PgPool;
use tower::ServiceExt;

const MCP_TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A lawn profile, and a state whose Home Assistant is the returned stub.
async fn lawn_with_patio_sensor(pool: PgPool) -> (Stub, crate::state::AppState) {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(&pool, &profile).await.unwrap();
    let ha = stub(200, HA_STATE).await;
    let service = service(
        &config_of(&with_home_assistant(&ha.url)),
        None,
        pool.clone(),
    )
    .await;
    (ha, app_state(pool, service, true))
}

/// Whether every Home Assistant entity read (the refresh) came before its probe.
fn probed_after_refresh(ha: &Stub) -> bool {
    let requests = ha.requests();
    let probe = requests.iter().position(|r| r.starts_with("GET /api/ "));
    let last_read = requests
        .iter()
        .rposition(|r| r.starts_with("GET /api/states/"));
    matches!((last_read, probe), (Some(read), Some(probe)) if read < probe)
}

// @spec SYNC-CONN-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_dashboard_carries_connections_checked_after_its_refresh(pool: PgPool) {
    let (ha, state) = lawn_with_patio_sensor(pool).await;
    let axum::Json(dashboard) = crate::api::dashboard::get_dashboard(State(state))
        .await
        .unwrap();
    assert!(dashboard.connections.homeassistant);
    assert!(!dashboard.connections.soildata && !dashboard.connections.openweathermap);
    assert!(probed_after_refresh(&ha), "{:?}", ha.requests());
}

// @spec SYNC-CONN-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_mcp_snapshot_carries_data_sources_checked_after_its_refresh(pool: PgPool) {
    let (ha, state) = lawn_with_patio_sensor(pool).await;
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "lawn_snapshot", "arguments": {}}
    });
    let request = Request::post("/mcp")
        .header(header::HOST, "turfops.lan")
        .header(header::AUTHORIZATION, format!("Bearer {MCP_TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-06-18")
        .body(Body::from(call.to_string()))
        .unwrap();
    let response = crate::mcp::router(state, MCP_TOKEN)
        .oneshot(request)
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let reply: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&body)));
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{reply}"));
    let snapshot: serde_json::Value = serde_json::from_str(text).unwrap();
    let sources = &snapshot["data_sources"];
    assert_eq!(sources["homeassistant"], true, "{snapshot}");
    assert_eq!(sources["soildata"], false);
    assert_eq!(sources["openweathermap"], false);
    assert!(probed_after_refresh(&ha), "{:?}", ha.requests());
}
