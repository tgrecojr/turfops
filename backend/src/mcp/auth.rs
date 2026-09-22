//! Bearer-token guard for `/mcp`. The MCP gateway in front of TurfOps is open, so the
//! token is the only thing standing between the network and the lawn's data.

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// Reject any request without `Authorization: Bearer <MCP_TOKEN>` with an empty 401.
/// The token never appears in logs or in the response.
pub async fn require_bearer(
    State(token): State<Arc<str>>,
    request: Request,
    next: Next,
) -> Response {
    if authorized(request.headers(), &token) {
        return next.run(request).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        [(WWW_AUTHENTICATE, r#"Bearer realm="turfops-mcp""#)],
    )
        .into_response()
}

/// The auth scheme is case-insensitive (RFC 9110); the token comparison is constant-time.
fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some((scheme, presented)) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
    else {
        return false;
    };
    scheme.eq_ignore_ascii_case("bearer")
        && bool::from(presented.trim().as_bytes().ct_eq(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn app() -> Router {
        Router::new().route("/mcp", get(|| async { "ok" })).layer(
            axum::middleware::from_fn_with_state(Arc::<str>::from(TOKEN), require_bearer),
        )
    }

    async fn status_with(header: Option<&str>) -> (StatusCode, Option<String>) {
        let mut request = Request::builder().uri("/mcp");
        if let Some(value) = header {
            request = request.header(AUTHORIZATION, value);
        }
        let response = app()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let challenge = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .map(|v| v.to_str().unwrap().to_string());
        (response.status(), challenge)
    }

    #[tokio::test]
    async fn missing_header_is_401_with_challenge() {
        let (status, challenge) = status_with(None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(challenge.as_deref(), Some(r#"Bearer realm="turfops-mcp""#));
    }

    #[tokio::test]
    async fn wrong_token_is_401() {
        let wrong = format!("Bearer {}", TOKEN.replace('0', "1"));
        assert_eq!(status_with(Some(&wrong)).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn prefix_of_the_token_is_401() {
        let prefix = format!("Bearer {}", &TOKEN[..31]);
        assert_eq!(status_with(Some(&prefix)).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn other_scheme_is_401() {
        let basic = format!("Basic {TOKEN}");
        assert_eq!(status_with(Some(&basic)).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn right_token_passes() {
        let (status, challenge) = status_with(Some(&format!("Bearer {TOKEN}"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(challenge, None);
    }

    #[tokio::test]
    async fn scheme_is_case_insensitive() {
        let lower = format!("bearer {TOKEN}");
        assert_eq!(status_with(Some(&lower)).await.0, StatusCode::OK);
    }
}
