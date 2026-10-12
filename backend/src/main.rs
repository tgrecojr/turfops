mod api;
mod clock;
mod config;
mod datasources;
mod db;
#[cfg(test)]
mod deployment_tests;
mod error;
mod health_probe;
mod logging;
mod logic;
mod mcp;
mod models;
mod state;
#[cfg(test)]
mod test_support;
mod web;

use crate::config::Config;
use crate::datasources::OpenRouterClient;
use crate::db::{pool::create_pool, queries};
use crate::logic::data_sync::DataSyncService;
use crate::state::AppState;
use axum::routing::{get, patch, post, put};
use axum::Router;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::limit::RequestBodyLimitLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "dev-env")]
    {
        let _ = dotenvy::dotenv();
    }

    // The container health check: probe our own /api/v1/health and exit.
    if std::env::args().any(|arg| arg == "--healthcheck") {
        let healthy = health_probe::probe(
            health_probe::local_addr(),
            std::time::Duration::from_secs(3),
        )
        .await;
        std::process::exit(if healthy { 0 } else { 1 });
    }

    // Initialize tracing before anything that may warn about configuration.
    let (filter, filter_warning) = logging::filter(std::env::var("RUST_LOG").ok().as_deref());
    tracing_subscriber::fmt().with_env_filter(filter).init();
    if let Some(warning) = filter_warning {
        tracing::warn!("{warning}");
    }

    // Load config from environment
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // @spec DEPLOY-TZ-003, DEPLOY-TZ-004
    tracing::info!("Clock: {}", config.clock.describe(chrono::Utc::now()));
    if config.clock.is_system() {
        tracing::warn!(
            "TZ is not set: every date gate follows the host's default zone (UTC in the \
             container image). Set TZ to the lawn's timezone, for example America/New_York"
        );
    }

    // Connect to app database and run migrations
    let pool = create_pool(
        config.database.connect_options(),
        config.database.max_connections,
    )
    .await?;

    // Create default profile if DB is empty
    ensure_default_profile(&pool, &config).await?;

    // Initialize data sync service (connects to external datasources)
    let sync_service = DataSyncService::initialize(&config, pool.clone()).await;

    // Initialize OpenRouter client (optional, used for plant maintenance plans)
    let openrouter = config
        .openrouter
        .as_ref()
        .filter(|c| c.enabled && !c.api_key.is_empty())
        .map(|c| {
            tracing::info!(model = %c.model, "OpenRouter client configured for plant plans and product profiles");
            OpenRouterClient::new(c.clone())
        });
    if openrouter.is_none() {
        tracing::info!("OpenRouter not configured — plant plans and product profiles unavailable");
    }

    // Create app state
    let mcp_token = config.mcp.token.clone();
    let state = AppState::new(
        pool,
        sync_service,
        openrouter,
        mcp_token.is_some(),
        config.clock,
    );

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
            put(api::applications::update_application)
                .delete(api::applications::delete_application),
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
        .route(
            "/api/v1/disease-risk",
            get(api::disease_risk::get_disease_risk),
        )
        .route("/api/v1/frac-classes", get(api::frac_classes::lookup))
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
            "/api/v1/timing-windows",
            get(api::timing::get_timing_windows),
        )
        .route(
            "/api/v1/plants",
            get(api::plants::list_plants).post(api::plants::create_plant),
        )
        .route(
            "/api/v1/plants/{id}",
            get(api::plants::get_plant)
                .put(api::plants::update_plant)
                .delete(api::plants::delete_plant),
        )
        .route(
            "/api/v1/plants/{id}/refresh-plan",
            post(api::plants::refresh_plant_plan),
        )
        .route(
            "/api/v1/products",
            get(api::products::list_products).post(api::products::create_product),
        )
        .route(
            "/api/v1/products/{id}",
            get(api::products::get_product)
                .put(api::products::update_product)
                .delete(api::products::delete_product),
        )
        .route(
            "/api/v1/products/{id}/refresh-profile",
            post(api::products::refresh_product_profile),
        )
        .route(
            "/api/v1/products/{id}/link-applications",
            post(api::products::link_applications),
        )
        .route(
            "/api/v1/inventory/suggestions",
            get(api::products::logged_product_suggestions),
        )
        .route(
            "/api/v1/inventory/shopping-list",
            get(api::inventory::get_shopping_list),
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
        .route("/api/v1/today", get(api::today::get_today))
        .route(
            "/api/v1/soil-tests/{id}",
            put(api::soil_tests::update_soil_test).delete(api::soil_tests::delete_soil_test),
        )
        // @spec UI-SRV-003
        .layer(RequestBodyLimitLayer::new(1024 * 1024)); // 1MB request body limit
                                                         // Every /api/v1 error is JSON with a plain message, including unknown paths and
                                                         // requests rejected before a handler runs.
    let app = api::json_errors::with_json_errors(app)
        .layer(web::cors_layer(
            config.server.cors_allowed_origin.as_deref(),
            config.server.port,
        ))
        .with_state(state.clone());

    // MCP server for LLM clients: server-to-server, so merged after the CORS layer.
    // @spec MCP-CFG-001, MCP-CFG-003, MCP-CFG-005
    let app = match mcp_token {
        Some(token) => {
            tracing::info!("MCP server mounted at /mcp (bearer token required)");
            app.merge(mcp::router(state, &token))
        }
        None => {
            tracing::info!("MCP_TOKEN not set — /mcp not mounted");
            app
        }
    };

    // Serve the React SPA, with client routes falling back to index.html.
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "./static".to_string());
    let app = web::with_frontend(app, &PathBuf::from(&static_dir));

    // Start server
    let addr = SocketAddr::new(
        config.server.host.parse().unwrap_or([0, 0, 0, 0].into()),
        config.server.port,
    );
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("Server stopped");

    Ok(())
}

/// SIGTERM (`docker stop`) or Ctrl-C: stop accepting connections and let in-flight
/// requests finish before exiting.
// @spec DEPLOY-START-006
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                tracing::warn!("Could not listen for SIGTERM: {e}");
                std::future::pending::<()>().await
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Shutdown signal received; finishing in-flight requests");
}

/// Create a default lawn profile from config if no profile exists in the DB.
// @spec APP-PROF-005
async fn ensure_default_profile(pool: &sqlx::PgPool, config: &Config) -> anyhow::Result<()> {
    if queries::get_default_lawn_profile(pool).await?.is_some() {
        return Ok(());
    }

    tracing::info!("No lawn profile found, creating default from config");

    let (profile, warnings) = config.lawn.seed_profile();
    for warning in warnings {
        tracing::warn!("{warning}");
    }

    queries::create_lawn_profile(pool, &profile).await?;
    tracing::info!("Default lawn profile created: {}", config.lawn.name);

    Ok(())
}
