use super::*;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use tower::ServiceExt;

/// A throwaway static directory holding `files`.
fn static_dir(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("turfops-web-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (file, body) in files {
        std::fs::write(dir.join(file), body).unwrap();
    }
    dir
}

async fn send(app: Router, method: Method, uri: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn api() -> Router {
    Router::new().route("/api/v1/health", get(|| async { "ok" }))
}

// @spec UI-SRV-001
#[tokio::test]
async fn a_client_route_loads_index_html_with_200() {
    let dir = static_dir(
        "spa",
        &[("index.html", "<title>TurfOps</title>"), ("app.js", "js")],
    );
    let app = with_frontend(api(), &dir);
    let (status, body) = send(app.clone(), Method::GET, "/timing").await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::OK, "<title>TurfOps</title>")
    );
    let (status, _) = send(app.clone(), Method::HEAD, "/disease-risk/brown-patch").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        send(app.clone(), Method::GET, "/app.js").await,
        (StatusCode::OK, "js".into())
    );
    assert_eq!(send(app, Method::GET, "/api/v1/health").await.1, "ok");
}

// @spec UI-SRV-010
#[tokio::test]
async fn without_a_bundled_frontend_the_root_says_so() {
    let dir = static_dir("empty", &[]);
    let (status, body) = send(with_frontend(api(), &dir), Method::GET, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("not bundled"), "{body}");
}
