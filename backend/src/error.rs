use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TurfOpsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Data source unavailable: {0}")]
    DataSourceUnavailable(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, TurfOpsError>;

impl IntoResponse for TurfOpsError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            TurfOpsError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            TurfOpsError::InvalidData(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            TurfOpsError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            TurfOpsError::DataSourceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, msg.clone())
            }
            other => {
                tracing::error!("Internal error: {}", other);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
