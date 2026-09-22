//! Tool routers, one file per area; `TurfOpsMcp::tools` sums them.

mod lawn;
mod log;

use crate::error::TurfOpsError;
use axum::Json;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::ErrorData;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;

/// A tool result carrying `value` as JSON text.
fn json_result<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::json(value)?]))
}

/// A failed handler as a tool-level error the model can read and relay. Only messages
/// written for users pass through; anything else (SQL, lake internals) stays in the log.
fn tool_error(err: TurfOpsError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(error_message(err))])
}

const MISSING_DATA_HINT: &str = "Tell the user this data is missing rather than estimating it.";

fn error_message(err: TurfOpsError) -> String {
    match err {
        TurfOpsError::NotFound(msg) | TurfOpsError::InvalidData(msg) => msg,
        TurfOpsError::DataSourceUnavailable(msg) => {
            format!("Data source unavailable: {msg}. {MISSING_DATA_HINT}")
        }
        TurfOpsError::DataLake(err) => {
            tracing::warn!(error = %err, "MCP tool: weather data lake unreadable");
            format!(
                "The NOAA weather data lake could not be read, so station soil readings, \
                 timing windows and disease risk are unavailable. {MISSING_DATA_HINT}"
            )
        }
        TurfOpsError::Http(err) => {
            tracing::warn!(error = %err, "MCP tool: upstream request failed");
            format!(
                "An external data source (forecast or Home Assistant) did not respond. \
                 {MISSING_DATA_HINT}"
            )
        }
        other => {
            tracing::error!(error = %other, "MCP tool failed");
            "TurfOps hit an internal error answering this; details are in the server log."
                .to_string()
        }
    }
}

/// Run a REST handler and return its JSON body as the tool result.
async fn call<T, F>(handler: F) -> Result<CallToolResult, ErrorData>
where
    T: Serialize,
    F: Future<Output = Result<Json<T>, TurfOpsError>>,
{
    match handler.await {
        Ok(Json(body)) => json_result(&body),
        Err(err) => Ok(tool_error(err)),
    }
}

/// Run a REST handler and reshape its body with `trim` before returning it.
async fn call_trimmed<T, F>(
    handler: F,
    trim: fn(Value) -> Value,
) -> Result<CallToolResult, ErrorData>
where
    T: Serialize,
    F: Future<Output = Result<Json<T>, TurfOpsError>>,
{
    match to_value(handler).await {
        Ok(body) => json_result(&trim(body)),
        Err(err) => Ok(tool_error(err)),
    }
}

/// A handler's body as a JSON value, for trimming or composing.
async fn to_value<T, F>(handler: F) -> Result<Value, TurfOpsError>
where
    T: Serialize,
    F: Future<Output = Result<Json<T>, TurfOpsError>>,
{
    let Json(body) = handler.await?;
    Ok(serde_json::to_value(body)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_facing_errors_pass_through() {
        assert_eq!(
            error_message(TurfOpsError::NotFound("No lawn profile found".into())),
            "No lawn profile found"
        );
        assert!(
            error_message(TurfOpsError::DataSourceUnavailable("lake".into()))
                .starts_with("Data source unavailable: lake.")
        );
    }

    #[test]
    fn lake_errors_say_what_is_missing_without_the_path() {
        let err = duckdb::Error::InvalidPath("/data/silver/weather/x.parquet".into());
        let msg = error_message(TurfOpsError::DataLake(err));
        assert!(msg.contains("weather data lake could not be read"));
        assert!(!msg.contains("/data/"));
    }

    #[test]
    fn internal_errors_are_not_leaked() {
        let msg = error_message(TurfOpsError::Config("DATABASE_PASSWORD=hunter2".into()));
        assert!(!msg.contains("hunter2"));
    }
}
