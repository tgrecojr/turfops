use super::*;
use crate::config::test_config;
use crate::logic::data_sync::DataSyncService;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::time::Instant;
use tower::ServiceExt;

/// State whose database never answers (nothing listens on port 1) and whose lake is absent.
async fn state(mcp_enabled: bool) -> AppState {
    let config = test_config(&[("DATALAKE_ROOT", "/nonexistent/turfops-health")]);
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(60))
        .connect_lazy_with(PgConnectOptions::new().host("127.0.0.1").port(1));
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    AppState::new(pool, sync, None, mcp_enabled, config.clock)
}

async fn get_health(state: AppState) -> (StatusCode, serde_json::Value) {
    let app = Router::new()
        .route("/api/v1/health", get(health_check))
        .with_state(state);
    let request = Request::get("/api/v1/health").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// @spec DEPLOY-HEALTH-001, DEPLOY-HEALTH-002
#[tokio::test]
async fn a_database_that_does_not_answer_is_still_a_200_reading_degraded() {
    let started = Instant::now();
    let (status, body) = get_health(state(false).await).await;
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the database check gives up after 2 s: {:?}",
        started.elapsed()
    );
    assert_eq!(status, StatusCode::OK);
    let mut keys: Vec<&str> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["database", "datasources", "mcp", "status", "version"]
    );
    assert_eq!(body["database"], false);
    assert_eq!(body["status"], "degraded");
}

// @spec DEPLOY-HEALTH-003
#[tokio::test]
async fn health_reports_the_sources_mcp_and_the_crate_version() {
    let (_, body) = get_health(state(true).await).await;
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["mcp"], "enabled");
    let sources = body["datasources"].as_object().unwrap();
    for key in ["soildata", "homeassistant", "openweathermap"] {
        assert!(sources[key].is_boolean(), "{key}: {body}");
    }
    assert_eq!(sources["soildata"], false, "the lake is absent");

    let (_, body) = get_health(state(false).await).await;
    assert_eq!(body["mcp"], "disabled");
}
