//! Shared helpers for the MCP tests: an `AppState` over a given pool and lake, JSON-RPC
//! over the real `/mcp` router, a small silver/gold lake, and log capture.

use crate::clock::Clock;
use crate::config::Config;
use crate::logic::data_sync::DataSyncService;
use crate::state::AppState;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

pub const TOKEN: &str = "0123456789abcdef0123456789abcdef";
pub const PROTOCOL: &str = "2025-06-18";

/// A configuration with no Home Assistant, no forecast and the lake at `silver` / `gold`.
pub fn config(silver: &str, gold: &str) -> Config {
    serde_json::from_value(json!({
        "lawn": {"name": "Test lawn", "grass_type": "TallFescue", "usda_zone": "7a",
                 "soil_type": null, "lawn_size_sqft": null, "irrigation_type": null},
        "noaa": {"station_wbanno": 3761},
        "datalake": {"silver_weather_path": silver, "gold_weather_path": gold},
        "homeassistant": {"url": "http://127.0.0.1:1", "token": "",
                          "temperature_entity": "sensor.t", "humidity_entity": "sensor.h"},
        "openweathermap": null,
        "openrouter": null,
        "server": {"host": "127.0.0.1", "port": 3000, "cors_allowed_origin": null},
        "database": {"host": "127.0.0.1", "port": 1, "name": "none", "user": "none",
                     "password": "none", "max_connections": 1},
        "mcp": {"token": TOKEN}
    }))
    .expect("test config")
}

/// A pool that never connects: every query fails quickly with a database error.
pub fn unreachable_pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(300))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .expect("lazy pool")
}

/// App state over `pool`, with the lake at `lake` (paths that need not exist).
pub async fn state(pool: PgPool, lake: &Lake) -> AppState {
    let config = config(&lake.silver, &lake.gold);
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    AppState::new(pool, sync, None, true, Clock::System)
}

/// Silver and gold parquet paths, removed on drop.
pub struct Lake {
    pub silver: String,
    pub gold: String,
}

impl Lake {
    /// Paths that do not exist: every lake read fails.
    pub fn missing(name: &str) -> Self {
        Self {
            silver: format!("/nonexistent/turfops-{name}/silver.parquet"),
            gold: format!("/nonexistent/turfops-{name}/gold.parquet"),
        }
    }

    /// A lake holding `days` complete days ending yesterday (the lawn's clock): silver
    /// hours at 20 °C, 70 % RH and 0.5 mm of rain in the 06:00 hour, soil at 18 °C; gold
    /// rows with air min 50 °F, 10 GDD, and a varying air mean and 10 cm soil mean (so
    /// the air-to-soil regression has something to fit). `gold_only` dates exist in gold
    /// alone (air min 41 °F, mean 51 °F, 1 GDD).
    pub fn with_days(name: &str, days: i64, gold_only: &[chrono::NaiveDate]) -> Self {
        let dir = std::env::temp_dir();
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tag = format!("{name}_{}_{n}", std::process::id());
        let lake = Self {
            silver: dir
                .join(format!("turfops_mcp_silver_{tag}.parquet"))
                .display()
                .to_string(),
            gold: dir
                .join(format!("turfops_mcp_gold_{tag}.parquet"))
                .display()
                .to_string(),
        };
        let first = Clock::System.today() - chrono::Duration::days(days);
        let extra: Vec<String> = gold_only
            .iter()
            .map(|d| format!("(DATE '{d}', 41.0, 51.0, 61.0, 1.0, 24, 55.0)"))
            .collect();
        let extra = if extra.is_empty() {
            String::new()
        } else {
            format!(" UNION ALL SELECT * FROM (VALUES {}) v", extra.join(", "))
        };
        duckdb::Connection::open_in_memory()
            .unwrap()
            .execute_batch(&format!(
                "COPY (
                    SELECT '03761' AS wbanno,
                           ts AS obs_ts_utc, ts AS obs_ts_local,
                           CAST(ts AS DATE) AS obs_date_local,
                           20.0 AS air_temp_c, 19.0 AS air_temp_min_c, 21.0 AS air_temp_max_c,
                           70.0 AS rh_pct,
                           CASE WHEN hour(ts) = 6 THEN 0.5 ELSE 0.0 END AS precip_mm,
                           18.0 AS soil_temp_5, 18.0 AS soil_temp_10, 18.0 AS soil_temp_20,
                           18.0 AS soil_temp_50, 18.0 AS soil_temp_100,
                           0.25 AS soil_moisture_5, 0.25 AS soil_moisture_10,
                           0.25 AS soil_moisture_20, 0.25 AS soil_moisture_50,
                           0.25 AS soil_moisture_100
                    FROM (SELECT TIMESTAMP '{first} 00:00:00' + h * INTERVAL 1 HOUR AS ts
                          FROM range({hours}) t(h))
                 ) TO '{silver}' (FORMAT PARQUET);
                 COPY (
                    SELECT * FROM (
                        SELECT DATE '{first}' + CAST(d AS INTEGER) AS day,
                               50.0 AS air_temp_min_f, 55.0 + (d * 7) % 11 AS air_temp_avg_f,
                               70.0 AS air_temp_max_f, 10.0 AS gdd50,
                               24 AS hours_observed,
                               58.0 + ((d * 7) % 11) * 0.6 + (d % 3) * 0.4
                                   AS soil_temp_10_f_mean
                        FROM range({days}) t(d)){extra}
                 ) TO '{gold}' (FORMAT PARQUET);",
                hours = days * 24,
                silver = lake.silver,
                gold = lake.gold,
            ))
            .expect("lake fixture");
        lake
    }
}

impl Drop for Lake {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.silver);
        let _ = std::fs::remove_file(&self.gold);
    }
}

/// A POST to `/mcp` carrying the bearer token and the headers an MCP client sends.
pub fn post(body: &Value) -> axum::http::request::Builder {
    post_to("turfops.lan:3000", body)
}

/// `post` addressed to `host`.
pub fn post_to(host: &str, body: &Value) -> axum::http::request::Builder {
    Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::HOST, host)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .header("mcp-protocol-version", PROTOCOL)
        .header(header::CONTENT_LENGTH, body.to_string().len())
}

/// Send `request` to `app`; the status, the headers and the body as text.
pub async fn send(
    app: Router,
    request: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let response = app.oneshot(request).await.unwrap();
    let (parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
    (
        parts.status,
        parts.headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

/// One JSON-RPC request to the real `/mcp` router; the JSON-RPC response.
pub async fn rpc(state: &AppState, method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let request = post(&body).body(Body::from(body.to_string())).unwrap();
    let (status, _, text) = send(super::router(state.clone(), TOKEN), request).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

/// The outcome of one tool call.
#[derive(Debug)]
pub struct ToolReply {
    pub is_error: bool,
    pub text: String,
}

impl ToolReply {
    /// The reply's JSON body.
    pub fn json(&self) -> Value {
        assert!(!self.is_error, "tool error: {}", self.text);
        serde_json::from_str(&self.text).unwrap_or_else(|e| panic!("{e}: {}", self.text))
    }
}

/// Call `name` with `arguments` over JSON-RPC. Panics on a protocol-level error.
pub async fn call_tool(state: &AppState, name: &str, arguments: Value) -> ToolReply {
    let response = rpc(
        state,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    )
    .await;
    assert!(
        response.get("error").is_none(),
        "{name}: JSON-RPC error {response}"
    );
    let result = &response["result"];
    let content = result["content"].as_array().expect("content");
    assert_eq!(content.len(), 1, "{name}: one content block");
    assert_eq!(content[0]["type"], "text");
    ToolReply {
        is_error: result["isError"].as_bool().unwrap_or(false),
        text: content[0]["text"].as_str().unwrap().to_string(),
    }
}

/// Everything logged at any level while `f` runs, as text.
pub fn logs_of<R>(f: impl FnOnce() -> R) -> (R, String) {
    let logs = Logs::capture();
    let result = f();
    (result, logs.text())
}

/// Log capture on the current thread (a `#[tokio::test]` runs its tasks there) until
/// dropped.
pub struct Logs {
    buffer: Arc<Mutex<Vec<u8>>>,
    _guard: tracing::subscriber::DefaultGuard,
}

impl Logs {
    pub fn capture() -> Self {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = buffer.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(move || Capture(writer.clone()))
            .finish();
        crate::test_support::keep_tracing_interest_shared();
        Self {
            buffer,
            _guard: tracing::subscriber::set_default(subscriber),
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.buffer.lock().unwrap()).into_owned()
    }
}

struct Capture(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
