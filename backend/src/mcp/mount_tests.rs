//! Whether and how `/mcp` is mounted: the token rules end to end, the place of the routes
//! relative to CORS and the SPA fallback, the health report, and what the token never
//! reaches.

use super::test_support::*;
use super::{parse_token, router};
use crate::web;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use serde_json::json;

const MAIN: &str = include_str!("../main.rs");

/// `main`'s MCP mounting step: from the CORS-layered API router to the merged app.
fn main_mount_step() -> &'static str {
    let start = MAIN
        .find("let app = match mcp_token")
        .expect("the mount step");
    let end = start + MAIN[start..].find("};").expect("its end");
    &MAIN[start..end]
}

/// The API as `main` builds it before `/mcp`: routes, then the CORS layer.
fn api() -> Router {
    Router::new()
        .route("/api/v1/health", get(|| async { "ok" }))
        .layer(web::cors_layer(None, 3000))
}

fn spa_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("turfops-mcp-spa-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<title>TurfOps</title>").unwrap();
    dir
}

// @spec MCP-CFG-001
#[test]
fn without_a_token_main_logs_it_and_merges_nothing() {
    let step = main_mount_step();
    let none_arm = &step[step.find("None =>").expect("a None arm")..];
    assert!(none_arm.contains(r#"tracing::info!("MCP_TOKEN not set — /mcp not mounted")"#));
    assert!(!none_arm.contains("merge"), "{none_arm}");
    assert!(MAIN.contains("let mcp_token = config.mcp.token.clone();"));
}

// @spec MCP-CFG-001
#[tokio::test]
async fn without_a_token_mcp_is_just_another_spa_path() {
    let app = web::with_frontend(api(), &spa_dir());
    for method in [Method::GET, Method::POST] {
        let request = |uri: &str| {
            Request::builder()
                .method(method.clone())
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap()
        };
        let mcp = send(app.clone(), request("/mcp")).await;
        let unknown = send(app.clone(), request("/no-such-page")).await;
        assert_eq!((mcp.0, &mcp.2), (unknown.0, &unknown.2), "{method}");
    }
    let (status, _, body) = send(
        app,
        Request::builder().uri("/mcp").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::OK, "<title>TurfOps</title>")
    );
}

// @spec MCP-CFG-003
#[tokio::test]
async fn a_long_enough_token_mounts_mcp_behind_its_trimmed_value() {
    let token = parse_token(Some(format!("  {TOKEN}\n"))).unwrap().unwrap();
    assert_eq!(token, TOKEN);
    let state = state(unreachable_pool(), &Lake::missing("mount")).await;
    let app = web::with_frontend(api().merge(router(state, &token)), &spa_dir());
    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}});
    let (status, _, text) = send(
        app.clone(),
        post(&list).body(Body::from(list.to_string())).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(text.contains("lawn_snapshot"));
    let unauthorized = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(header::AUTHORIZATION, format!("Bearer  {TOKEN}x"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(app, unauthorized).await.0, StatusCode::UNAUTHORIZED);
}

// @spec MCP-CFG-005
#[test]
fn main_merges_mcp_after_the_api_cors_layer() {
    let cors = MAIN
        .find(".layer(web::cors_layer(")
        .expect("the CORS layer");
    let merge = MAIN
        .find("app.merge(mcp::router(state, &token))")
        .expect("the merge");
    assert!(cors < merge, "/mcp must be merged after the CORS layer");
}

// @spec MCP-CFG-005
#[tokio::test]
async fn the_browser_cors_policy_does_not_apply_to_mcp() {
    let state = state(unreachable_pool(), &Lake::missing("cors")).await;
    let app = api().merge(router(state, TOKEN));
    let preflight = |uri: &str| {
        Request::builder()
            .method(Method::OPTIONS)
            .uri(uri)
            .header(header::ORIGIN, "http://localhost:3000")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .body(Body::empty())
            .unwrap()
    };
    let (_, api_headers, _) = send(app.clone(), preflight("/api/v1/health")).await;
    assert!(api_headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    let (status, mcp_headers, _) = send(app.clone(), preflight("/mcp")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "no CORS answer for /mcp");
    assert!(!mcp_headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));

    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}});
    let request = post(&list)
        .header(header::ORIGIN, "http://localhost:3000")
        .body(Body::from(list.to_string()))
        .unwrap();
    let (status, headers, _) = send(app, request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!headers.contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
}

// @spec MCP-CFG-006
#[tokio::test]
async fn health_reports_whether_mcp_is_mounted() {
    for (mounted, expected) in [(true, "enabled"), (false, "disabled")] {
        let mut state = state(unreachable_pool(), &Lake::missing("health")).await;
        state.mcp_enabled = mounted;
        let health = crate::api::health::health_check(axum::extract::State(state)).await;
        assert_eq!(serde_json::to_value(&health.0).unwrap()["mcp"], expected);
    }
    assert!(
        MAIN.contains("mcp_token.is_some(),"),
        "mcp_enabled = mounted"
    );
}

// @spec MCP-CFG-004
#[tokio::test]
async fn the_token_reaches_no_log_line_and_no_response() {
    let mount = main_mount_step();
    for line in mount.lines().filter(|l| l.contains("tracing::")) {
        assert!(!line.contains("token") || !line.contains('{'), "{line}");
    }
    let logs = Logs::capture();
    let state = state(unreachable_pool(), &Lake::missing("token")).await;
    let profile = call_tool(&state, "lawn_profile", json!({})).await;
    let snapshot = call_tool(&state, "lawn_snapshot", json!({})).await;
    let init = rpc(
        &state,
        "initialize",
        json!({"protocolVersion": PROTOCOL, "capabilities": {},
               "clientInfo": {"name": "test", "version": "1"}}),
    )
    .await;
    assert!(
        profile.is_error,
        "the database is unreachable: an error is logged"
    );
    for reply in [profile.text, snapshot.text, init.to_string()] {
        assert!(!reply.contains(TOKEN), "{reply}");
    }
    let logged = logs.text();
    assert!(!logged.is_empty(), "logs were captured");
    assert!(!logged.contains(TOKEN), "{logged}");
}
