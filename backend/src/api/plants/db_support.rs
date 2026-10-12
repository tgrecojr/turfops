//! An `AppState` over a test database, and an OpenRouter stand-in, for the plant handlers.

use crate::clock::Clock;
use crate::config::{Config, OpenRouterConfig};
use crate::datasources::openrouter::stub::{capture, Captured};
use crate::datasources::OpenRouterClient;
use crate::db::{plant_queries, queries};
use crate::logic::data_sync::DataSyncService;
use crate::models::plant::{IdentificationConfidence, Plant, PlantMaintenancePlan, PlantType};
use crate::models::{GrassType, LawnProfile};
use crate::state::AppState;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::oneshot;

/// The HTTP status an error answers with.
pub(super) fn status(err: crate::error::TurfOpsError) -> axum::http::StatusCode {
    use axum::response::IntoResponse;
    err.into_response().status()
}

pub(super) const MODEL: &str = "test/plan-model";

/// The app's state with no data sources reachable, and `openrouter` as the assistant.
pub(super) async fn state(pool: PgPool, openrouter: Option<OpenRouterClient>) -> AppState {
    let config: Config = serde_json::from_value(json!({
        "lawn": { "name": "Lawn", "grass_type": "TallFescue", "usda_zone": "7a",
                  "soil_type": null, "lawn_size_sqft": null, "irrigation_type": null },
        "noaa": { "station_wbanno": 3761 },
        "datalake": { "silver_weather_path": "/nonexistent/silver.parquet",
                      "gold_weather_path": "/nonexistent/gold.parquet" },
        "homeassistant": { "url": "http://127.0.0.1:9", "token": "",
                           "temperature_entity": "t", "humidity_entity": "h" },
        "openweathermap": null,
        "openrouter": null,
        "server": { "host": "127.0.0.1", "port": 3000, "cors_allowed_origin": null },
        "database": { "host": "localhost", "port": 5432, "name": "t", "user": "t",
                      "password": "t", "max_connections": 1 },
        "mcp": { "token": null }
    }))
    .unwrap();
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    AppState::new(pool, sync, openrouter, false, Clock::default())
}

/// An assistant that answers the first request with `status` and `body`, and the request
/// it received.
pub(super) async fn assistant(
    status: u16,
    body: String,
) -> (OpenRouterClient, oneshot::Receiver<Captured>) {
    let (base_url, received) = capture(status, body).await;
    let client = OpenRouterClient::new(OpenRouterConfig {
        api_key: "test_key".into(),
        model: MODEL.into(),
        enabled: true,
        base_url,
    });
    (client, received)
}

/// A lawn profile in `zone`; returns its id.
pub(super) async fn profile(pool: &PgPool, zone: &str) -> i64 {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, zone.into());
    queries::create_lawn_profile(pool, &profile).await.unwrap()
}

/// A saved plant with a one-line plan written by `old/model`.
pub(super) async fn saved_plant(pool: &PgPool, profile_id: i64, name: &str) -> i64 {
    let plant = Plant {
        id: None,
        lawn_profile_id: profile_id,
        common_name: name.into(),
        scientific_name: Some("Ilex opaca".into()),
        plant_type: PlantType::Shrub,
        location: Some("back fence".into()),
        planting_date: None,
        notes: Some("planted by the previous owner".into()),
        maintenance_plan: PlantMaintenancePlan {
            identified_name: name.into(),
            scientific_name: Some("Ilex opaca".into()),
            identification_confidence: IdentificationConfidence::Medium,
            summary: "The old plan.".into(),
            tasks: vec![],
            warnings: vec![],
        },
        plan_generated_at: Utc::now() - chrono::Duration::days(30),
        plan_model: "old/model".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    plant_queries::create_plant(pool, &plant).await.unwrap()
}
