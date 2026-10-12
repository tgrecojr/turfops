//! The API error contract: one JSON shape, one status per kind of failure.

use super::*;

/// The status and the whole JSON body of `err`'s response.
async fn response_of(err: TurfOpsError) -> (StatusCode, serde_json::Value) {
    let response = err.into_response();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// @spec UI-ERR-001
#[tokio::test]
async fn every_failure_is_an_error_body_with_its_status() {
    let cases = [
        (
            TurfOpsError::NotFound("No such application.".into()),
            StatusCode::NOT_FOUND,
        ),
        (
            TurfOpsError::InvalidData("pH must be between 0 and 14.".into()),
            StatusCode::BAD_REQUEST,
        ),
        (
            TurfOpsError::Conflict("Already on the shelf.".into()),
            StatusCode::CONFLICT,
        ),
        (
            TurfOpsError::not_configured(Source::Forecast),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            TurfOpsError::Internal("rows disagree".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            TurfOpsError::Config("bad setting".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];
    for (err, expected) in cases {
        let label = format!("{err:?}");
        let (status, body) = response_of(err).await;
        assert_eq!(status, expected, "{label}");
        let object = body.as_object().expect("a JSON object");
        assert_eq!(object.len(), 1, "only the error field: {body}");
        let message = object["error"].as_str().expect("a string message");
        assert!(!message.is_empty(), "{label}");
    }
    // A duplicate application-log entry is the 409 with the duplicate explanation.
    assert_eq!(
        unique_violation(Some("applications_unique_turf")),
        (StatusCode::CONFLICT, DUPLICATE_MESSAGE)
    );
}
