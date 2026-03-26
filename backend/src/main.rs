mod api;
mod config;
mod datasources;
mod db;
mod error;
mod logic;
mod models;
mod state;

use crate::config::Config;
use crate::db::{pool::create_pool, queries};
use crate::logic::data_sync::DataSyncService;
use crate::models::{GrassType, IrrigationType, LawnProfile, SoilType};
use crate::state::AppState;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Load config from environment
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Connect to app database and run migrations
    let pool = create_pool(config.database.connect_options()).await?;

    // Create default profile if DB is empty
    ensure_default_profile(&pool, &config).await?;

    // Initialize data sync service (connects to external datasources)
    let sync_service = DataSyncService::initialize(&config, pool.clone()).await;

    // Create app state
    let state = AppState::new(pool, sync_service);

    // Build router
    let app = Router::new()
        .route("/api/v1/health", get(api::health::health_check))
        .route("/api/v1/dashboard", get(api::dashboard::get_dashboard))
        .route(
            "/api/v1/profile",
            get(api::profile::get_profile).put(api::profile::update_profile),
        )
        .route(
            "/api/v1/applications",
            get(api::applications::list_applications).post(api::applications::create_application),
        )
        .route(
            "/api/v1/applications/{id}",
            delete(api::applications::delete_application),
        )
        .route(
            "/api/v1/applications/calendar",
            get(api::calendar::get_calendar),
        )
        .route(
            "/api/v1/environmental",
            get(api::environmental::get_environmental),
        )
        .route(
            "/api/v1/environmental/refresh",
            post(api::environmental::refresh_environmental),
        )
        .route(
            "/api/v1/recommendations",
            get(api::recommendations::list_recommendations),
        )
        .route(
            "/api/v1/recommendations/{id}",
            patch(api::recommendations::patch_recommendation),
        )
        .route("/api/v1/gdd", get(api::gdd::get_gdd))
        .route("/api/v1/historical", get(api::historical::get_historical))
        .route(
            "/api/v1/nitrogen-budget",
            get(api::nitrogen_budget::get_nitrogen_budget),
        )
        .route(
            "/api/v1/seasonal-plan",
            get(api::seasonal_plan::get_seasonal_plan),
        )
        .route(
            "/api/v1/soil-temp-forecast",
            get(api::soil_temp_prediction::get_soil_temp_forecast),
        )
        .route(
            "/api/v1/soil-tests/recommendations",
            get(api::soil_tests::get_soil_test_recommendations),
        )
        .route(
            "/api/v1/soil-tests",
            get(api::soil_tests::list_soil_tests).post(api::soil_tests::create_soil_test),
        )
        .route(
            "/api/v1/soil-tests/{id}",
            put(api::soil_tests::update_soil_test).delete(api::soil_tests::delete_soil_test),
        )
        .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1MB request body limit
        .layer(build_cors_layer(&config))
        .with_state(state);

    // Serve React SPA static files with fallback to index.html
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "./static".to_string());
    let static_path = PathBuf::from(&static_dir);

    let app = if static_path.join("index.html").exists() {
        let index_file = static_path.join("index.html");
        let serve_dir = ServeDir::new(&static_path).not_found_service(ServeFile::new(&index_file));
        tracing::info!("Serving static files from {}", static_dir);
        app.fallback_service(serve_dir)
    } else {
        tracing::warn!("Static directory '{}' not found, API-only mode", static_dir);
        app
    };

    // Start server
    let addr = SocketAddr::new(
        config.server.host.parse().unwrap_or([0, 0, 0, 0].into()),
        config.server.port,
    );
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Build CORS layer from config. Defaults to same-origin (the server's own address)
/// when CORS_ALLOWED_ORIGIN is not set. Set to "*" for permissive access.
fn build_cors_layer(config: &Config) -> CorsLayer {
    use axum::http::Method;

    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
    ];

    match config.server.cors_allowed_origin.as_deref() {
        Some("*") => {
            tracing::warn!(
                "CORS configured to allow all origins — use CORS_ALLOWED_ORIGIN to restrict"
            );
            CorsLayer::permissive()
        }
        Some(origin) => {
            tracing::info!(origin = %origin, "CORS restricted to configured origin");
            CorsLayer::new()
                .allow_origin(
                    origin
                        .parse::<axum::http::HeaderValue>()
                        .expect("Invalid CORS_ALLOWED_ORIGIN"),
                )
                .allow_methods(methods)
                .allow_headers(tower_http::cors::Any)
        }
        None => {
            // Default: allow only from the server's own origin (same host/port)
            let default_origin = format!("http://localhost:{}", config.server.port);
            tracing::info!(origin = %default_origin, "CORS defaulting to localhost origin");
            CorsLayer::new()
                .allow_origin(default_origin.parse::<axum::http::HeaderValue>().unwrap())
                .allow_methods(methods)
                .allow_headers(tower_http::cors::Any)
        }
    }
}

/// Create a default lawn profile from config if no profile exists in the DB.
async fn ensure_default_profile(pool: &sqlx::PgPool, config: &Config) -> anyhow::Result<()> {
    if queries::get_default_lawn_profile(pool).await?.is_some() {
        return Ok(());
    }

    tracing::info!("No lawn profile found, creating default from config");

    let grass_type = GrassType::from_str(&config.lawn.grass_type).unwrap_or_else(|_| {
        tracing::warn!(
            grass_type = %config.lawn.grass_type,
            "Unknown LAWN_GRASS_TYPE, defaulting to TallFescue"
        );
        GrassType::TallFescue
    });
    let soil_type = config
        .lawn
        .soil_type
        .as_ref()
        .and_then(|s| SoilType::from_str(s).ok());
    let irrigation_type = config
        .lawn
        .irrigation_type
        .as_ref()
        .and_then(|i| IrrigationType::from_str(i).ok());

    let mut profile = LawnProfile::new(
        config.lawn.name.clone(),
        grass_type,
        config.lawn.usda_zone.clone(),
    );
    profile.soil_type = soil_type;
    profile.lawn_size_sqft = config.lawn.lawn_size_sqft;
    profile.irrigation_type = irrigation_type;

    queries::create_lawn_profile(pool, &profile).await?;
    tracing::info!("Default lawn profile created: {}", config.lawn.name);

    Ok(())
}
