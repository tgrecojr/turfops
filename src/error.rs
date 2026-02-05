use thiserror::Error;

#[derive(Error, Debug)]
pub enum TurfOpsError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("External database error: {0}")]
    ExternalDatabase(#[from] sqlx::Error),

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
