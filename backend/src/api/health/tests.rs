//! The health response's shape, and a database that does not answer.

use super::health_check;
use crate::logic::data_sync::test_env::*;
use axum::extract::State;

// @spec SYNC-CONN-003
#[tokio::test]
async fn health_reports_status_version_database_datasources_and_mcp() {
    for (mcp_enabled, mcp) in [(true, "enabled"), (false, "disabled")] {
        let service = service(&config(&[]), None, unreachable_pool()).await;
        let state = app_state(unreachable_pool(), service, mcp_enabled);
        let axum::Json(health) = health_check(State(state)).await;
        let json = serde_json::to_value(&health).unwrap();
        for key in ["status", "version", "database", "datasources", "mcp"] {
            assert!(json.get(key).is_some(), "{key}: {json}");
        }
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(json["mcp"], mcp);
        for source in ["soildata", "homeassistant", "openweathermap"] {
            assert_eq!(json["datasources"][source], false, "{source}: {json}");
        }
    }
}

// @spec SYNC-CONN-004
#[tokio::test]
async fn health_is_degraded_when_the_database_does_not_answer() {
    let state = offline_state(unreachable_pool()).await;
    let started = std::time::Instant::now();
    let axum::Json(health) = health_check(State(state)).await;
    assert_eq!(health.status, "degraded");
    assert!(!health.database);
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
}
