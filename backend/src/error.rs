use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TurfOpsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Data lake query error: {0}")]
    DataLake(#[from] duckdb::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// A data source cannot answer. The owner sees a fixed sentence for the source and
    /// the reason; any detail goes to the log only.
    #[error("{origin:?} unavailable: {outage:?}")]
    Unavailable { origin: Source, outage: Outage },

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Not found: {0}")]
    NotFound(String),

    /// The server's own data is inconsistent; the detail is logged, the owner sees the
    /// generic server-error sentence.
    #[error("Internal inconsistency: {0}")]
    Internal(String),

    /// The request clashes with what is already stored; the message says what and is shown
    /// to the owner as written.
    #[error("Conflict: {0}")]
    Conflict(String),
}

/// What a unique-index violation means to the user. The only unique indexes writable
/// through the API are the application-log duplicate guards.
const DUPLICATE_MESSAGE: &str =
    "An identical entry (same type, product and date) is already logged. \
     Edit the existing entry, or change the product name if this was a different product.";

pub type Result<T> = std::result::Result<T, TurfOpsError>;

/// The sentence the owner sees when `source` cannot answer: what is unavailable, what it
/// affects and what to do.
fn outage_message(source: Source, outage: &Outage) -> String {
    use Outage::*;
    use Source::*;
    match (source, outage) {
        (WeatherLake, NotConfigured | Disabled) => {
            "The weather data lake isn't configured, so soil, disease, timing and history \
             data are unavailable. Set DATALAKE_ROOT and restart TurfOps."
        }
        (WeatherLake, _) => {
            "The weather data lake can't be read right now, so soil, disease, timing and \
             history data are unavailable. Try again in a few minutes."
        }
        (Forecast, NotConfigured | Disabled) => {
            "The weather forecast isn't configured, so forecast-based advice is unavailable. \
             Set OWM_API_KEY and restart TurfOps."
        }
        (Forecast, _) => {
            "The weather forecast (OpenWeatherMap) is unavailable right now, so \
             forecast-based advice is limited. Try again later."
        }
        (PatioSensor, NotConfigured | Disabled) => {
            "The patio sensor (Home Assistant) isn't configured, so live air temperature and \
             humidity are unavailable."
        }
        (PatioSensor, _) => {
            "The patio sensor (Home Assistant) can't be reached right now, so live air \
             temperature and humidity are unavailable."
        }
        (PlanAssistant, NotConfigured) => {
            "The plan assistant isn't configured, so care plans and product profiles can't \
             be generated. Set OPENROUTER_API_KEY and restart TurfOps."
        }
        (PlanAssistant, Disabled) => {
            "The plan assistant is turned off. Set OPENROUTER_ENABLED=true to generate care \
             plans and product profiles."
        }
        (PlanAssistant, _) => {
            "The plan assistant couldn't answer just now. Try again in a few minutes."
        }
        (ProductAssistant, NotConfigured) => {
            "The product assistant isn't configured. Pick a category to save the product by \
             hand, or set OPENROUTER_API_KEY and restart TurfOps."
        }
        (ProductAssistant, Disabled) => {
            "The product assistant is turned off. Pick a category to save the product by \
             hand, or set OPENROUTER_ENABLED=true."
        }
        (ProductAssistant, _) => {
            "The assistant couldn't identify this product just now. Pick a category to save \
             it by hand, or try again in a few minutes."
        }
        (SoilOutlook, _) => {
            "Not enough recent station data to project soil temperature. Try again once the \
             station has reported a few more days."
        }
    }
    .to_string()
}

/// A data source as the owner knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The NOAA weather data lake (station soil, history, timing and disease inputs).
    WeatherLake,
    /// The OpenWeatherMap forecast.
    Forecast,
    /// The Home Assistant patio sensor.
    PatioSensor,
    /// OpenRouter, which writes plant care plans and product profiles.
    PlanAssistant,
    /// OpenRouter answering for a shelf product (product profiles).
    ProductAssistant,
    /// The 10 cm soil-temperature outlook.
    SoilOutlook,
}

/// Why a source cannot answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outage {
    NotConfigured,
    Disabled,
    /// It failed; the detail is for the log only.
    Down(String),
    /// It answered, but there is too little data to work with.
    NotEnoughData,
}

impl TurfOpsError {
    /// The message the owner sees for this error (the `error` field of the response):
    /// a request problem as written, a data-source sentence naming the source, the
    /// duplicate-entry explanation, or the generic server-error sentence.
    pub fn owner_message(&self) -> String {
        match self {
            TurfOpsError::NotFound(msg)
            | TurfOpsError::InvalidData(msg)
            | TurfOpsError::Conflict(msg) => msg.clone(),
            TurfOpsError::Unavailable { origin, outage } => outage_message(*origin, outage),
            TurfOpsError::Database(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                DUPLICATE_MESSAGE.to_string()
            }
            _ => SERVER_ERROR_MESSAGE.to_string(),
        }
    }

    /// `source` failed; `detail` goes to the log, never to the owner.
    pub fn unavailable(source: Source, detail: impl std::fmt::Display) -> Self {
        TurfOpsError::Unavailable {
            origin: source,
            outage: Outage::Down(detail.to_string()),
        }
    }

    /// `source` is not set up.
    pub fn not_configured(source: Source) -> Self {
        TurfOpsError::Unavailable {
            origin: source,
            outage: Outage::NotConfigured,
        }
    }

    /// `source` is set up but switched off.
    pub fn disabled(source: Source) -> Self {
        TurfOpsError::Unavailable {
            origin: source,
            outage: Outage::Disabled,
        }
    }

    /// The same outage, worded for a shelf product (OpenRouter answering for products).
    pub fn for_products(self) -> Self {
        match self {
            TurfOpsError::Unavailable {
                origin: Source::PlanAssistant,
                outage,
            } => TurfOpsError::Unavailable {
                origin: Source::ProductAssistant,
                outage,
            },
            other => other,
        }
    }

    /// `source` has too little data to answer.
    pub fn not_enough_data(source: Source) -> Self {
        TurfOpsError::Unavailable {
            origin: source,
            outage: Outage::NotEnoughData,
        }
    }
}

/// What the owner sees for any server-side failure; the details are logged.
pub const SERVER_ERROR_MESSAGE: &str =
    "Something went wrong on the server. Try again; the details are in the server log.";

// @spec DEPLOY-HTTP-008, DEPLOY-HTTP-009, UI-ERR-003
impl IntoResponse for TurfOpsError {
    fn into_response(self) -> Response {
        let status = match &self {
            TurfOpsError::NotFound(_) => StatusCode::NOT_FOUND,
            TurfOpsError::InvalidData(_) => StatusCode::BAD_REQUEST,
            TurfOpsError::Unavailable { outage, .. } => {
                if let Outage::Down(detail) = outage {
                    tracing::warn!(error = %detail, "{}", self);
                }
                StatusCode::SERVICE_UNAVAILABLE
            }
            TurfOpsError::Database(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                StatusCode::CONFLICT
            }
            TurfOpsError::Conflict(_) => StatusCode::CONFLICT,
            other => {
                tracing::error!("Internal error: {}", other);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let body = serde_json::json!({ "error": self.owner_message() });
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn body_of(err: TurfOpsError) -> (StatusCode, String) {
        let response = err.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, json["error"].as_str().unwrap().to_string())
    }

    // @spec DEPLOY-HTTP-009, UI-ERR-003, PLANT-AI-007
    #[tokio::test]
    async fn an_outage_names_the_source_and_keeps_the_detail_in_the_log() {
        let (status, msg) = body_of(TurfOpsError::unavailable(
            Source::PlanAssistant,
            "OpenRouter returned 500: {\"secret\":\"upstream body\"}",
        ))
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(msg.contains("plan assistant"), "{msg}");
        assert!(
            !msg.contains("secret") && !msg.contains("OpenRouter returned"),
            "{msg}"
        );
    }

    // @spec DEPLOY-HTTP-009, UI-ERR-007
    #[tokio::test]
    async fn an_unconfigured_lake_says_what_is_missing_and_what_it_affects() {
        let (status, msg) = body_of(TurfOpsError::not_configured(Source::WeatherLake)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(msg.contains("weather data lake isn't configured"), "{msg}");
        assert!(
            msg.contains("disease"),
            "names the affected features: {msg}"
        );
    }

    // @spec PLANT-AI-008
    #[tokio::test]
    async fn a_disabled_assistant_says_how_to_turn_it_on() {
        let (_, msg) = body_of(TurfOpsError::disabled(Source::PlanAssistant)).await;
        assert!(msg.contains("OPENROUTER_ENABLED=true"), "{msg}");
    }

    // @spec WX-OUTLOOK-API-003
    #[tokio::test]
    async fn too_little_data_for_the_outlook_is_a_plain_503() {
        let (status, msg) = body_of(TurfOpsError::not_enough_data(Source::SoilOutlook)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(msg.starts_with("Not enough recent station data"), "{msg}");
    }

    // @spec INV-API-012
    #[tokio::test]
    async fn a_failed_product_lookup_says_what_to_do_next() {
        let err = TurfOpsError::unavailable(Source::PlanAssistant, "raw model output: {…}");
        let (status, msg) = body_of(err.for_products()).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(msg.contains("couldn't identify this product"), "{msg}");
        assert!(msg.contains("Pick a category"), "{msg}");
        assert!(!msg.contains("raw model output"));
    }

    // @spec INV-API-013
    #[tokio::test]
    async fn a_conflict_is_a_409_with_its_own_message() {
        let (status, msg) = body_of(TurfOpsError::Conflict("Already on the shelf.".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(msg, "Already on the shelf.");
    }

    // @spec DEPLOY-HTTP-008
    #[tokio::test]
    async fn a_server_error_says_to_try_again_without_internals() {
        let (status, msg) = body_of(TurfOpsError::Config("DATABASE_PASSWORD=hunter2".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(msg.contains("Try again"), "{msg}");
        assert!(msg.contains("server log"), "{msg}");
        assert!(!msg.contains("hunter2"));
    }
}
