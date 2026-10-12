//! Shared setup for the PostgreSQL-backed application and profile handler tests: an
//! `AppState` over the test pool with every outside source unconfigured, a lawn, a plant,
//! a shelf product, and a request builder.

use super::CreateApplicationRequest;
use crate::config::Config;
use crate::db::{plant_queries, product_queries, queries};
use crate::error::TurfOpsError;
use crate::logic::data_sync::DataSyncService;
use crate::models::plant::{IdentificationConfidence, Plant, PlantMaintenancePlan, PlantType};
use crate::models::product::{Product, ProductCategory, ProductFacts, ProductForm, StockStatus};
use crate::models::{Application, ApplicationType, GrassType, LawnProfile};
use crate::state::AppState;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;

/// A configuration with the given `LAWN_*` settings and nothing outside configured.
pub(crate) fn config(lawn: serde_json::Value) -> Config {
    serde_json::from_value(json!({
        "lawn": lawn,
        "noaa": { "station_wbanno": 3761 },
        "datalake": {
            "silver_weather_path": "/nonexistent/silver.parquet",
            "gold_weather_path": "/nonexistent/gold.parquet"
        },
        "homeassistant": {
            "url": "", "token": "", "temperature_entity": "", "humidity_entity": ""
        },
        "openweathermap": null,
        "openrouter": null,
        "server": { "host": "127.0.0.1", "port": 0, "cors_allowed_origin": null },
        "database": {
            "host": "", "port": 5432, "name": "", "user": "", "password": "x",
            "max_connections": 1
        },
        "mcp": { "token": null }
    }))
    .expect("test config")
}

pub(crate) async fn state(pool: &PgPool) -> AppState {
    let config = config(json!({
        "name": "Main Lawn", "grass_type": "TallFescue", "usda_zone": "7a",
        "soil_type": null, "lawn_size_sqft": null, "irrigation_type": null
    }));
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    AppState::new(pool.clone(), sync, None, false, config.clock)
}

/// Create the lawn profile; returns its id.
pub(crate) async fn lawn(pool: &PgPool) -> i64 {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(pool, &profile).await.unwrap()
}

pub(crate) async fn plant(pool: &PgPool, lawn_id: i64, name: &str) -> i64 {
    let now = Utc::now();
    let plant = Plant {
        id: None,
        lawn_profile_id: lawn_id,
        common_name: name.into(),
        scientific_name: None,
        plant_type: PlantType::Shrub,
        location: None,
        planting_date: None,
        notes: None,
        maintenance_plan: PlantMaintenancePlan {
            identified_name: name.into(),
            scientific_name: None,
            identification_confidence: IdentificationConfidence::High,
            summary: String::new(),
            tasks: Vec::new(),
            warnings: Vec::new(),
        },
        plan_generated_at: now,
        plan_model: "test".into(),
        created_at: now,
        updated_at: now,
    };
    plant_queries::create_plant(pool, &plant).await.unwrap()
}

pub(crate) async fn product(
    pool: &PgPool,
    lawn_id: i64,
    name: &str,
    category: ProductCategory,
) -> i64 {
    let now = Utc::now();
    let product = Product {
        id: None,
        lawn_profile_id: lawn_id,
        name: name.into(),
        brand: None,
        stock_status: StockStatus::InStock,
        facts: ProductFacts::manual(category, ProductForm::Granular),
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: None,
        archived: false,
        created_at: now,
        updated_at: now,
    };
    product_queries::create_product(pool, &product)
        .await
        .unwrap()
}

/// A request with only the required type and date.
pub(crate) fn request(kind: &str, date: &str) -> CreateApplicationRequest {
    CreateApplicationRequest {
        application_type: kind.into(),
        product_name: None,
        application_date: date.into(),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
    }
}

/// A turf entry of `kind` on `date` (YYYY-MM-DD) named `product`, ready to store.
pub(crate) fn entry(lawn_id: i64, kind: ApplicationType, date: &str, product: &str) -> Application {
    Application {
        id: None,
        lawn_profile_id: lawn_id,
        application_type: kind,
        product_name: Some(product.into()).filter(|p: &String| !p.is_empty()),
        application_date: date.parse().unwrap(),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

/// Store `app`; returns its id.
pub(crate) async fn store(pool: &PgPool, app: &Application) -> i64 {
    queries::create_application(pool, app).await.unwrap()
}

/// The status and owner message an error becomes.
pub(crate) async fn reply(err: TurfOpsError) -> (StatusCode, String) {
    let response = err.into_response();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (
        status,
        json["error"].as_str().unwrap_or_default().to_string(),
    )
}

/// How many application rows exist.
pub(crate) async fn count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM applications")
        .fetch_one(pool)
        .await
        .unwrap()
}
