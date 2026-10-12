//! What the server answers outside the API: the bundled React SPA and the CORS policy.

use axum::response::Html;
use axum::routing::get;
use axum::Router;
use std::path::Path;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

/// `app` serving the SPA from `static_dir`: existing files as they are, every other path
/// the client router owns with `index.html` and 200, so a reload or a deep link loads the
/// app. Without a bundled frontend, `/` says so.
// @spec UI-SRV-001, UI-SRV-010, UI-SRV-002, DEPLOY-HTTP-004
pub fn with_frontend(app: Router, static_dir: &Path) -> Router {
    let index_file = static_dir.join("index.html");
    if index_file.exists() {
        // `fallback` (not `not_found_service`) keeps the 200 for client routes.
        let serve_dir = ServeDir::new(static_dir).fallback(ServeFile::new(&index_file));
        tracing::info!("Serving static files from {}", static_dir.display());
        app.fallback_service(serve_dir)
    } else {
        tracing::warn!(
            "Static directory '{}' not found, API-only mode",
            static_dir.display()
        );
        app.route("/", get(|| async { Html(NOT_BUNDLED) }))
    }
}

const NOT_BUNDLED: &str = "<!doctype html><title>TurfOps</title><p>TurfOps is running, \
but the web frontend was not bundled into this build. The API is at <code>/api/v1/</code>.\
</p>";

/// Build CORS layer from config. Defaults to same-origin (the server's own address)
/// when CORS_ALLOWED_ORIGIN is not set. Set to "*" for permissive access.
// @spec UI-SRV-004, UI-SRV-005, UI-SRV-006
pub fn cors_layer(origin: Option<&str>, port: u16) -> CorsLayer {
    use axum::http::Method;

    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
    ];

    match origin {
        Some("*") => {
            tracing::warn!(
                "CORS configured to allow all origins — use CORS_ALLOWED_ORIGIN to restrict"
            );
            CorsLayer::permissive()
        }
        Some(origin) => {
            tracing::info!(origin = %origin, "CORS restricted to configured origin");
            // Validated when the configuration loads (UI-SRV-009).
            let Ok(origin) = origin.parse::<axum::http::HeaderValue>() else {
                return CorsLayer::new();
            };
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods(methods)
                .allow_headers(tower_http::cors::Any)
        }
        None => {
            // Default: allow only from the server's own origin (same host/port)
            let default_origin = format!("http://localhost:{port}");
            tracing::info!(origin = %default_origin, "CORS defaulting to localhost origin");
            CorsLayer::new()
                .allow_origin(default_origin.parse::<axum::http::HeaderValue>().unwrap())
                .allow_methods(methods)
                .allow_headers(tower_http::cors::Any)
        }
    }
}

#[cfg(test)]
mod cors_tests;
#[cfg(test)]
mod limits_tests;
#[cfg(test)]
mod tests;
