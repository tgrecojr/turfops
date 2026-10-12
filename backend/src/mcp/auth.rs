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
// @spec MCP-AUTH-001, MCP-AUTH-003, MCP-CFG-004
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
// @spec MCP-AUTH-001, MCP-AUTH-002
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
    use axum::http::HeaderValue;
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

    // @spec MCP-AUTH-003
    #[tokio::test]
    async fn missing_header_is_401_with_challenge() {
        let (status, challenge) = status_with(None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(challenge.as_deref(), Some(r#"Bearer realm="turfops-mcp""#));
    }

    // @spec MCP-AUTH-003
    #[tokio::test]
    async fn wrong_token_is_401() {
        let wrong = format!("Bearer {}", TOKEN.replace('0', "1"));
        assert_eq!(status_with(Some(&wrong)).await.0, StatusCode::UNAUTHORIZED);
    }

    // @spec MCP-AUTH-003
    #[tokio::test]
    async fn prefix_of_the_token_is_401() {
        let prefix = format!("Bearer {}", &TOKEN[..31]);
        assert_eq!(status_with(Some(&prefix)).await.0, StatusCode::UNAUTHORIZED);
    }

    // @spec MCP-AUTH-003
    #[tokio::test]
    async fn other_scheme_is_401() {
        let basic = format!("Basic {TOKEN}");
        assert_eq!(status_with(Some(&basic)).await.0, StatusCode::UNAUTHORIZED);
    }

    // @spec MCP-AUTH-001
    #[tokio::test]
    async fn right_token_passes() {
        let (status, challenge) = status_with(Some(&format!("Bearer {TOKEN}"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(challenge, None);
    }

    // @spec MCP-AUTH-001
    #[tokio::test]
    async fn scheme_is_case_insensitive() {
        let lower = format!("bearer {TOKEN}");
        assert_eq!(status_with(Some(&lower)).await.0, StatusCode::OK);
    }

    // @spec MCP-AUTH-001
    #[tokio::test]
    async fn the_value_after_the_first_space_is_trimmed_before_comparing() {
        let padded = format!("Bearer   {TOKEN}  ");
        assert_eq!(status_with(Some(&padded)).await.0, StatusCode::OK);
        // Only the first space splits: a scheme glued to the token is not "Bearer".
        let glued = format!("Bearer{TOKEN}");
        assert_eq!(status_with(Some(&glued)).await.0, StatusCode::UNAUTHORIZED);
        let doubled = format!("Bearer {TOKEN} {TOKEN}");
        assert_eq!(
            status_with(Some(&doubled)).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    // @spec MCP-AUTH-003, MCP-CFG-004
    #[tokio::test]
    async fn a_refusal_is_an_empty_401_whatever_was_wrong() {
        let non_ascii =
            HeaderValue::from_bytes(format!("Bearer {TOKEN}\u{e9}").as_bytes()).unwrap();
        let cases = [
            None,
            Some(HeaderValue::from_str(&format!("Bearer {}", TOKEN.replace('a', "b"))).unwrap()),
            Some(HeaderValue::from_str(&format!("Bearer {}", &TOKEN[..31])).unwrap()),
            Some(HeaderValue::from_str(&format!("Basic {TOKEN}")).unwrap()),
            Some(non_ascii),
        ];
        for header in cases {
            let mut request = Request::builder().uri("/mcp");
            if let Some(value) = header.clone() {
                request = request.header(AUTHORIZATION, value);
            }
            let response = app()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{header:?}");
            assert_eq!(
                response.headers()[WWW_AUTHENTICATE],
                r#"Bearer realm="turfops-mcp""#
            );
            let headers = format!("{:?}", response.headers());
            assert!(!headers.contains(TOKEN), "the token is echoed");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert!(body.is_empty(), "{body:?}");
        }
    }

    // @spec MCP-AUTH-002
    #[test]
    fn the_token_comparison_is_constant_time() {
        let source = include_str!("auth.rs");
        let guard = source
            .split("fn authorized")
            .nth(1)
            .and_then(|rest| rest.split("#[cfg(test)]").next())
            .unwrap();
        assert!(guard.contains(".ct_eq(token.as_bytes())"), "{guard}");
        assert!(!guard.contains("== token"), "a short-circuiting compare");
        let headers = |v: &str| {
            let mut h = HeaderMap::new();
            h.insert(AUTHORIZATION, HeaderValue::from_str(v).unwrap());
            h
        };
        assert!(authorized(&headers(&format!("Bearer {TOKEN}")), TOKEN));
        assert!(!authorized(&headers(&format!("Bearer {TOKEN}x")), TOKEN));
    }
}
