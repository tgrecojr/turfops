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

pub(super) async fn send(app: Router, method: Method, uri: &str) -> (StatusCode, String) {
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

pub(super) fn api() -> Router {
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

/// What `f` logs (plain text, one event per line), with what it returns.
pub(super) fn logs_during<T>(f: impl FnOnce() -> T) -> (T, String) {
    #[derive(Clone, Default)]
    struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for Captured {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let out = Captured::default();
    let writer = out.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let value = tracing::subscriber::with_default(subscriber, f);
    let text = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
    (value, text)
}

// @spec UI-SRV-002
#[tokio::test]
async fn without_a_bundled_frontend_the_api_is_served_and_a_warning_logged() {
    let dir = static_dir("missing", &[]);
    let (app, logs) = logs_during(|| with_frontend(api(), &dir));
    let warning = logs
        .lines()
        .find(|line| line.contains("WARN"))
        .unwrap_or_else(|| panic!("no warning in: {logs}"));
    assert!(warning.contains(&dir.display().to_string()), "{warning}");
    assert!(warning.contains("API-only"), "{warning}");
    assert_eq!(
        send(app.clone(), Method::GET, "/api/v1/health").await,
        (StatusCode::OK, "ok".into())
    );
    // No SPA: a client route is not answered with a page.
    let (status, _) = send(app, Method::GET, "/timing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
