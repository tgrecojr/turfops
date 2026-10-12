//! Which origins may call `/api/v1` from a browser.

use super::tests::logs_during;
use super::*;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use axum::routing::get;
use tower::ServiceExt;

/// The CORS headers a preflight from `origin` gets.
async fn preflight(layer: CorsLayer, origin: &str) -> HeaderMap {
    let app = Router::new()
        .route("/api/v1/applications", get(|| async { "ok" }))
        .layer(layer);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/applications")
                .header("Origin", origin)
                .header("Access-Control-Request-Method", "PATCH")
                .header("Access-Control-Request-Headers", "x-anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.headers().clone()
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).map(|v| v.to_str().unwrap())
}

/// The allowed methods and headers are the API's write methods and any header.
fn assert_api_methods_and_any_header(headers: &HeaderMap) {
    let methods = header(headers, "access-control-allow-methods").unwrap();
    for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
        assert!(methods.contains(method), "{method} in {methods}");
    }
    assert_eq!(header(headers, "access-control-allow-headers"), Some("*"));
}

// @spec UI-SRV-004
#[tokio::test]
async fn unset_allows_only_the_servers_own_localhost_origin() {
    let allowed = preflight(cors_layer(None, 3000), "http://localhost:3000").await;
    assert_eq!(
        header(&allowed, "access-control-allow-origin"),
        Some("http://localhost:3000")
    );
    assert_api_methods_and_any_header(&allowed);

    for other in ["http://localhost:5173", "https://lawn.example.com"] {
        let refused = preflight(cors_layer(None, 3000), other).await;
        // A browser lets the page read the answer only when this names its origin.
        assert_ne!(
            header(&refused, "access-control-allow-origin"),
            Some(other),
            "{other}"
        );
        assert_ne!(header(&refused, "access-control-allow-origin"), Some("*"));
    }
    // The port follows SERVER_PORT.
    let moved = preflight(cors_layer(None, 8080), "http://localhost:8080").await;
    assert_eq!(
        header(&moved, "access-control-allow-origin"),
        Some("http://localhost:8080")
    );
}

// @spec UI-SRV-005
#[tokio::test]
async fn a_star_allows_any_origin_with_a_startup_warning() {
    let (layer, logs) = logs_during(|| cors_layer(Some("*"), 3000));
    assert!(
        logs.lines()
            .any(|line| line.contains("WARN") && line.contains("all origins")),
        "{logs}"
    );
    let headers = preflight(layer, "https://anywhere.example").await;
    assert_eq!(header(&headers, "access-control-allow-origin"), Some("*"));
}

// @spec UI-SRV-006
#[tokio::test]
async fn a_named_origin_is_the_only_one_allowed() {
    let origin = "https://lawn.example.com";
    let allowed = preflight(cors_layer(Some(origin), 3000), origin).await;
    assert_eq!(
        header(&allowed, "access-control-allow-origin"),
        Some(origin)
    );
    assert_api_methods_and_any_header(&allowed);

    for other in ["http://localhost:3000", "https://evil.example.com"] {
        let refused = preflight(cors_layer(Some(origin), 3000), other).await;
        // A browser lets the page read the answer only when this names its origin.
        assert_ne!(
            header(&refused, "access-control-allow-origin"),
            Some(other),
            "{other}"
        );
        assert_ne!(header(&refused, "access-control-allow-origin"), Some("*"));
    }
}
