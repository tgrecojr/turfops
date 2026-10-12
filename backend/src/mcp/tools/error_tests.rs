//! What a failing tool tells the model, and what stays in the log.

use super::{error_message, INTERNAL_ERROR, MISSING_DATA_HINT};
use crate::error::TurfOpsError;
use crate::mcp::test_support::*;
use serde_json::json;

const LAKE_TEXT: &str = "The NOAA weather data lake could not be read, so station soil \
                         readings, timing windows and disease risk are unavailable.";

// @spec MCP-ERR-001, MCP-ERR-005
#[tokio::test]
async fn a_failing_handler_is_a_tool_error_not_a_protocol_error() {
    let state = state(unreachable_pool(), &Lake::missing("err")).await;
    for tool in ["lawn_profile", "applications", "products", "soil_tests"] {
        let reply = call_tool(&state, tool, json!({})).await;
        assert!(reply.is_error, "{tool}");
        assert_eq!(reply.text, INTERNAL_ERROR, "{tool}");
    }
}

// @spec MCP-ERR-002
#[test]
fn not_found_and_invalid_data_messages_reach_the_model_verbatim() {
    let (messages, logged) = logs_of(|| {
        [
            error_message(TurfOpsError::NotFound("No lawn profile found".into())),
            error_message(TurfOpsError::InvalidData(
                "`since` must not be after `until`".into(),
            )),
        ]
    });
    assert_eq!(
        messages,
        [
            "No lawn profile found".to_string(),
            "`since` must not be after `until`".to_string()
        ]
    );
    assert!(logged.is_empty(), "{logged}");
}

// @spec MCP-ERR-003
#[test]
fn an_unreadable_lake_names_what_is_missing_without_path_or_sql() {
    let err = duckdb::Connection::open_in_memory()
        .unwrap()
        .execute_batch("SELECT * FROM read_parquet('/data/silver/weather/x.parquet')")
        .unwrap_err();
    let (msg, logged) = logs_of(|| error_message(TurfOpsError::DataLake(err)));
    assert_eq!(msg, format!("{LAKE_TEXT} {MISSING_DATA_HINT}"));
    assert!(!msg.contains("/data/") && !msg.contains("SELECT"), "{msg}");
    assert!(logged.contains("WARN"), "{logged}");
    assert!(
        logged.contains("/data/silver/weather/x.parquet"),
        "{logged}"
    );
}

// @spec MCP-ERR-003
#[tokio::test]
async fn a_tool_over_a_missing_lake_says_the_lake_could_not_be_read() {
    let state = state(unreachable_pool(), &Lake::missing("lake-err")).await;
    let reply = call_tool(&state, "weather_history", json!({})).await;
    assert!(reply.is_error);
    assert_eq!(reply.text, format!("{LAKE_TEXT} {MISSING_DATA_HINT}"));
    assert!(!reply.text.contains("nonexistent"), "{}", reply.text);
}

// @spec MCP-ERR-004
#[tokio::test]
async fn a_failed_upstream_request_says_a_source_did_not_respond() {
    let err = reqwest::Client::new()
        .get("http://127.0.0.1:1/forecast?appid=secret-key")
        .send()
        .await
        .unwrap_err();
    let (msg, logged) = logs_of(|| error_message(TurfOpsError::Http(err)));
    assert_eq!(
        msg,
        format!(
            "An external data source (forecast or Home Assistant) did not respond. \
             {MISSING_DATA_HINT}"
        )
    );
    assert!(!msg.contains("127.0.0.1"), "{msg}");
    assert!(logged.contains("WARN"), "{logged}");
    assert!(
        logged.contains("127.0.0.1"),
        "the detail is logged: {logged}"
    );
}

// @spec MCP-ERR-005
#[test]
fn any_other_failure_is_the_internal_error_sentence_logged_as_an_error() {
    let failures = [
        TurfOpsError::Database(sqlx::Error::RowNotFound),
        TurfOpsError::Config("DATABASE_PASSWORD=hunter2".into()),
        TurfOpsError::Io(std::io::Error::other("disk on fire")),
        TurfOpsError::Json(serde_json::from_str::<serde_json::Value>("{").unwrap_err()),
        TurfOpsError::Internal("profile row without id".into()),
    ];
    for err in failures {
        let detail = err.to_string();
        let (msg, logged) = logs_of(|| error_message(err));
        assert_eq!(
            msg,
            "TurfOps hit an internal error answering this; details are in the server log."
        );
        assert!(logged.contains("ERROR"), "{logged}");
        assert!(logged.contains(&detail), "{detail} not logged: {logged}");
    }
}
