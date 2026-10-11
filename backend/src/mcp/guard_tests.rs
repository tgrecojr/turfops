//! The request limits in front of `/mcp`: body size and method.

use super::guarded;
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::routing::any;
use axum::Router;
use tower::ServiceExt;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A stand-in for the MCP service that reads the whole body, as rmcp does.
fn app() -> Router {
    guarded(
        Router::new().route(
            "/mcp",
            any(|body: String| async move { body.len().to_string() }),
        ),
        TOKEN,
    )
}

async fn send(method: Method, body: Vec<u8>, token: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, body.len());
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap()
}

// @spec MCP-SRV-006
#[tokio::test]
async fn a_body_over_one_mebibyte_is_413() {
    let at_limit = send(Method::POST, vec![b' '; 1024 * 1024], Some(TOKEN)).await;
    assert_eq!(at_limit.status(), StatusCode::OK);
    let over = send(Method::POST, vec![b' '; 1024 * 1024 + 1], Some(TOKEN)).await;
    assert_eq!(over.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// @spec MCP-SRV-007
#[tokio::test]
async fn only_post_is_allowed() {
    for method in [Method::GET, Method::DELETE, Method::PUT] {
        let response = send(method.clone(), Vec::new(), Some(TOKEN)).await;
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{method}"
        );
        assert_eq!(response.headers().get(header::ALLOW).unwrap(), "POST");
    }
    assert_eq!(
        send(Method::POST, b"{}".to_vec(), Some(TOKEN))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn the_bearer_guard_still_answers_first() {
    assert_eq!(
        send(Method::GET, Vec::new(), None).await.status(),
        StatusCode::UNAUTHORIZED
    );
}
