//! Startup: the order `main` runs in, what it logs, and what it leaves untouched.

use crate::config::test_config;
use crate::logic::data_sync::DataSyncService;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAIN: &str = include_str!("../main.rs");
const CARGO_TOML: &str = include_str!("../../Cargo.toml");

/// The body of `main`, where startup happens.
fn main_body() -> &'static str {
    let start = MAIN.find("async fn main()").unwrap();
    let end = MAIN[start..].find("\n}\n").unwrap() + start;
    &MAIN[start..end]
}

fn position(needle: &str) -> usize {
    main_body()
        .find(needle)
        .unwrap_or_else(|| panic!("main does not {needle}"))
}

/// A pool that never connects (nothing listens on port 1) and does no I/O until used.
fn idle_pool() -> sqlx::PgPool {
    PgPoolOptions::new().connect_lazy_with(PgConnectOptions::new().host("127.0.0.1").port(1))
}

// @spec DEPLOY-START-001, DEPLOY-START-003
#[test]
fn startup_runs_logging_config_database_profile_clients_router_then_bind() {
    let steps = [
        "tracing_subscriber::fmt()",
        "Config::from_env()?",
        "create_pool(",
        "ensure_default_profile(&pool, &config).await?",
        "DataSyncService::initialize(",
        "OpenRouterClient::new(",
        "Router::new()",
        "TcpListener::bind(addr).await?",
    ];
    let at: Vec<usize> = steps.iter().map(|s| position(s)).collect();
    for (pair, step) in at.windows(2).zip(steps.windows(2)) {
        assert!(pair[0] < pair[1], "{} before {}", step[0], step[1]);
    }
    assert_eq!(
        main_body().matches("Config::from_env()").count(),
        1,
        "read once"
    );
    let pool = &main_body()[position("create_pool(")..position("ensure_default_profile(")];
    assert!(
        pool.contains(".await?;"),
        "a pool or migration failure ends main before the bind"
    );
}

// @spec DEPLOY-HTTP-001
#[test]
fn the_body_limit_wraps_every_api_route() {
    let body = main_body();
    let limit = position(".layer(RequestBodyLimitLayer::new(1024 * 1024))");
    let routes: Vec<usize> = body.match_indices(".route(").map(|(i, _)| i).collect();
    assert!(routes.len() >= 30, "{}", routes.len());
    assert!(
        routes.iter().all(|&r| r < limit),
        "a route added after the limit"
    );
    assert!(position("Router::new()") < routes[0]);
    assert!(limit < position("with_json_errors("));
}

// @spec DEPLOY-CONFIG-011
#[test]
fn only_the_dev_env_build_loads_a_dotenv_file() {
    assert!(CARGO_TOML.contains("dotenvy = { version = \"0.15\", optional = true }"));
    assert!(CARGO_TOML.contains("default = []\ndev-env = [\"dep:dotenvy\"]"));
    let gate = position("#[cfg(feature = \"dev-env\")]");
    let load = position("dotenvy::dotenv()");
    assert!(gate < load && load < position("Config::from_env()"));
    assert_eq!(MAIN.matches("dotenvy").count(), 1, "nowhere else");
}

// @spec DEPLOY-CONFIG-004
#[test]
fn the_openrouter_client_needs_a_key_and_the_flag() {
    let filter = position(".filter(|c| c.enabled && !c.api_key.is_empty())");
    assert!(position("config\n        .openrouter") < filter);
    assert!(filter < position("OpenRouterClient::new(c.clone())"));
}

/// Everything logged through the returned guard's subscriber.
#[derive(Clone, Default)]
struct Logs(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Logs {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Logs {
    fn capture() -> (Self, tracing::subscriber::DefaultGuard) {
        let logs = Logs::default();
        let writer = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .finish();
        let guard = tracing::subscriber::set_default(subscriber);
        // A callsite first reached on another test's thread (no subscriber) may have
        // cached "never"; recompute against this subscriber.
        tracing::callsite::rebuild_interest_cache();
        (logs, guard)
    }

    fn info_lines(&self) -> Vec<String> {
        String::from_utf8_lossy(&self.0.lock().unwrap())
            .lines()
            .filter(|l| l.contains(" INFO "))
            .map(str::to_string)
            .collect()
    }
}

// @spec DEPLOY-LOG-002
#[tokio::test]
async fn startup_logs_paths_integrations_cors_and_the_server_lines() {
    let start = || async {
        let config = test_config(&[
            ("DATALAKE_ROOT", "/lake"),
            ("HA_URL", "http://ha.example:8123"),
            ("HA_TOKEN", "ha-token"),
            ("OWM_API_KEY", "owm-key"),
            ("OWM_LATITUDE", "39.85"),
            ("OWM_LONGITUDE", "-75.78"),
            ("CORS_ALLOWED_ORIGIN", "https://lawn.example.com"),
        ]);
        let _sync = DataSyncService::initialize(&config, idle_pool()).await;
        let _cors = crate::web::cors_layer(config.server.cors_allowed_origin.as_deref(), 3000);
    };
    // Register every log callsite first: one first reached concurrently on another test's
    // thread (no subscriber) can otherwise miss this test's subscriber.
    start().await;
    let (logs, _guard) = Logs::capture();
    start().await;
    let info = logs.info_lines();
    for expected in [
        "/lake/silver/weather/silver_weather.parquet",
        "/lake/gold/weather/daily_weather.parquet",
        "Home Assistant client configured",
        "OpenWeatherMap client configured",
        "Weather data lake client configured",
        "https://lawn.example.com",
    ] {
        assert!(
            info.iter().any(|l| l.contains(expected)),
            "{expected}: {info:#?}"
        );
    }
    for line in [
        "tracing::info!(model = %c.model",
        "tracing::info!(\"MCP server mounted at /mcp",
        "tracing::info!(\"MCP_TOKEN not set — /mcp not mounted\")",
        "tracing::info!(\"Starting server on {}\", addr)",
    ] {
        position(line);
    }
}

// @spec DEPLOY-START-004
#[tokio::test]
async fn constructing_the_clients_touches_no_integration() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let lake = "/nonexistent/turfops-startup-lake";
    let config = test_config(&[
        ("DATALAKE_ROOT", lake),
        ("HA_URL", &url),
        ("HA_TOKEN", "ha-token"),
        ("OWM_API_KEY", "owm-key"),
        ("OWM_LATITUDE", "39.85"),
        ("OWM_LONGITUDE", "-75.78"),
        ("OPENROUTER_API_KEY", "sk-or-test"),
        ("OPENROUTER_BASE_URL", &url),
    ]);
    let started = std::time::Instant::now();
    let sync = DataSyncService::initialize(&config, idle_pool()).await;
    let _openrouter = crate::datasources::OpenRouterClient::new(config.openrouter.unwrap());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(sync.weather_client().is_some());
    assert!(
        tokio::time::timeout(Duration::from_millis(300), listener.accept())
            .await
            .is_err(),
        "nothing connected to Home Assistant or OpenRouter"
    );
    assert!(
        !std::path::Path::new(lake).exists(),
        "the lake was not touched"
    );
}
