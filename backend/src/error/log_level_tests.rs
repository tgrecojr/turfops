//! Which internal failures are logged at which level, and what the owner sees instead.

use super::*;

/// The response `err` becomes, with everything logged while it was built.
fn respond(err: TurfOpsError) -> (StatusCode, String, String) {
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
    crate::test_support::keep_tracing_interest_shared();
    let response = tracing::subscriber::with_default(subscriber, || err.into_response());
    let status = response.status();
    let bytes = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(axum::body::to_bytes(response.into_body(), usize::MAX))
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let logs = String::from_utf8(out.0.lock().unwrap().clone()).unwrap();
    (status, body["error"].as_str().unwrap().to_string(), logs)
}

// @spec UI-ERR-002, DEPLOY-HTTP-007
#[test]
fn internal_failures_are_logged_at_error_and_answer_the_server_sentence() {
    let cases = [
        TurfOpsError::Config("secret-config-detail".into()),
        TurfOpsError::Internal("secret-internal-detail".into()),
        TurfOpsError::Io(std::io::Error::other("secret-io-detail")),
        TurfOpsError::Json(
            serde_json::from_str::<serde_json::Value>("{secret-json-detail").unwrap_err(),
        ),
    ];
    for err in cases {
        let detail = err.to_string();
        let (status, message, logs) = respond(err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{detail}");
        assert_eq!(
            message,
            "Something went wrong on the server. Try again; the details are in the server log."
        );
        assert!(logs.contains("ERROR") && logs.contains(&detail), "{logs}");
        assert!(!message.contains("secret"));
    }
}

// @spec UI-ERR-002, DEPLOY-HTTP-007
#[test]
fn a_lake_failure_is_logged_at_warn_and_answers_the_lakes_503() {
    let (status, message, logs) = respond(TurfOpsError::DataLake(duckdb::Error::InvalidPath(
        "/data/secret/silver_weather.parquet".into(),
    )));
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(message.contains("weather data lake"), "{message}");
    assert!(!message.contains("secret"), "{message}");
    assert!(logs.contains("WARN") && logs.contains("secret"), "{logs}");
    assert!(!logs.contains("ERROR"), "{logs}");
}
