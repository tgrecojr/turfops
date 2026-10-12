//! The 1 MB request-body limit on `/api/v1`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use tower::ServiceExt;
use tower_http::limit::RequestBodyLimitLayer;

const MAIN: &str = include_str!("../main.rs");
const LIMIT: &str = ".layer(RequestBodyLimitLayer::new(1024 * 1024))";

/// The API router as `main` builds it: the routes, the body limit, the JSON errors.
fn api() -> Router {
    let routes = Router::new()
        .route(
            "/api/v1/soil-tests",
            post(|body: String| async move { body.len().to_string() }),
        )
        .layer(RequestBodyLimitLayer::new(1024 * 1024));
    crate::api::json_errors::with_json_errors(routes)
}

async fn post_bytes(len: usize) -> StatusCode {
    api()
        .oneshot(
            Request::post("/api/v1/soil-tests")
                .header("Content-Type", "application/json")
                .body(Body::from(vec![b' '; len]))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

// @spec UI-SRV-003
#[test]
fn main_limits_every_api_route_to_one_megabyte() {
    let limit = MAIN
        .find(LIMIT)
        .expect("the API router has the 1 MB body limit");
    let routes = MAIN.find("let app = Router::new()").unwrap();
    let json_errors = MAIN
        .find("api::json_errors::with_json_errors(app)")
        .unwrap();
    // Applied to the whole API router (after its last route, before anything else).
    assert!(routes < limit && limit < json_errors);
    assert!(!MAIN[limit + LIMIT.len()..json_errors].contains(".route("));
}

// @spec UI-SRV-003
#[tokio::test]
async fn a_body_over_one_megabyte_is_rejected() {
    assert_eq!(post_bytes(1024 * 1024).await, StatusCode::OK);
    assert_eq!(
        post_bytes(1024 * 1024 + 1).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}
