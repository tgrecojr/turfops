//! Every `/api/v1` error is the JSON `{"error": "<message>"}` shape with a message written
//! for the owner: an unknown API path is a JSON 404 rather than the web app's page, and a
//! request axum rejects before a handler runs (malformed JSON, a path id that is not a
//! number, a body over the size limit) gets a plain sentence instead of axum's text.

use crate::error::{TurfOpsError, SERVER_ERROR_MESSAGE};
use axum::http::{header, StatusCode};
use axum::middleware::map_response;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};

/// Wrap the `/api/v1` routes so their errors are JSON with plain-language messages.
// @spec UI-SRV-008, DEPLOY-HTTP-010
pub fn with_json_errors<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/api/v1/{*path}", any(no_such_endpoint))
        .layer(map_response(json_error_body))
}

async fn no_such_endpoint() -> TurfOpsError {
    TurfOpsError::NotFound("There is no such API endpoint.".into())
}

/// An error response that is not already JSON (an axum rejection, the body-size limit)
/// becomes the JSON error shape with a sentence for its status.
async fn json_error_body(response: Response) -> Response {
    let status = response.status();
    let is_json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("application/json"));
    if !(status.is_client_error() || status.is_server_error()) || is_json {
        return response;
    }
    let body = serde_json::json!({ "error": sentence_for(status) });
    (status, Json(body)).into_response()
}

fn sentence_for(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "The request couldn't be read. Check the form and try again.",
        StatusCode::NOT_FOUND => "There is no such API endpoint.",
        StatusCode::METHOD_NOT_ALLOWED => "That action isn't supported here.",
        StatusCode::PAYLOAD_TOO_LARGE => "That request is too large to accept.",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "The request must be sent as JSON.",
        StatusCode::UNPROCESSABLE_ENTITY => {
            "Some fields are missing or not valid. Check the form and try again."
        }
        s if s.is_server_error() => SERVER_ERROR_MESSAGE,
        _ => "The request couldn't be completed. Try again.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::Path;
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use axum::Json;
    use tower::ServiceExt;
    use tower_http::limit::RequestBodyLimitLayer;

    /// A miniature app: two API routes behind the size limit, and the web app's page as
    /// the catch-all, as in `main`.
    fn app() -> Router {
        let api = Router::new()
            .route(
                "/api/v1/echo",
                post(|Json(v): Json<serde_json::Value>| async move { Json(v) }),
            )
            .route(
                "/api/v1/items/{id}",
                get(|Path(id): Path<i64>| async move { id.to_string() }),
            )
            .layer(RequestBodyLimitLayer::new(64));
        with_json_errors(api).fallback(|| async { "<!doctype html><title>TurfOps</title>" })
    }

    async fn send(req: Request<Body>) -> (StatusCode, serde_json::Value) {
        let response = app().oneshot(req).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    fn post_json(body: &str) -> Request<Body> {
        Request::post("/api/v1/echo")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    // @spec UI-SRV-008
    #[tokio::test]
    async fn an_unknown_api_path_is_a_json_404_not_the_web_page() {
        let (status, json) = send(Request::get("/api/v1/nope").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json["error"].is_string(), "JSON error body: {json}");
    }

    // @spec DEPLOY-HTTP-010
    #[tokio::test]
    async fn malformed_json_gets_a_plain_json_error() {
        let (status, json) = send(post_json("{not json")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let msg = json["error"].as_str().expect("JSON error body");
        assert!(
            !msg.contains("Failed to parse"),
            "axum's text leaked: {msg}"
        );
    }

    // @spec DEPLOY-HTTP-010
    #[tokio::test]
    async fn a_non_numeric_id_gets_a_plain_json_error() {
        let (status, json) = send(
            Request::get("/api/v1/items/abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json["error"].is_string(), "JSON error body: {json}");
    }

    // @spec DEPLOY-HTTP-010
    #[tokio::test]
    async fn an_oversized_body_gets_a_plain_json_error() {
        let (status, json) = send(post_json(&format!("[{}]", "1,".repeat(100) + "1"))).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(json["error"].is_string(), "JSON error body: {json}");
    }

    #[tokio::test]
    async fn the_web_page_still_answers_other_paths() {
        let response = app()
            .oneshot(Request::get("/timing").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
