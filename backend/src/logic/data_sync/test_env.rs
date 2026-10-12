//! Test-only builders: a data sync service whose sources are local stubs (or absent), an
//! `AppState` around it, and a stub HTTP server that records every request it answers.

use super::DataSyncService;
use crate::clock::Clock;
use crate::config::Config;
use crate::state::AppState;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A lake mount that does not exist: every lake read fails, as when the lake is unmounted.
pub(crate) const NO_LAKE: &str = "/nonexistent/turfops-test-lake";
/// A Home Assistant entity state that parses as a number.
pub(crate) const HA_STATE: &str = r#"{"state":"72.0","entity_id":"sensor.patio"}"#;
/// An OpenWeatherMap forecast with no points.
pub(crate) const OWM_FORECAST: &str = r#"{"list":[],"city":{"name":"Avondale","country":"US","coord":{"lat":39.8,"lon":-75.8},"timezone":-14400}}"#;
/// Nothing listens here: a connection is refused at once.
pub(crate) const UNREACHABLE: &str = "http://127.0.0.1:1";
pub(crate) const HA_TOKEN: &str = "ha-test-token";

/// A local HTTP server that answers every request alike and records each one as
/// `"<request line> | <authorization header>"`.
pub(crate) struct Stub {
    pub url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Stub {
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

/// Start a [`Stub`] answering `status` with the JSON `body`.
pub(crate) async fn stub(status: u16, body: &'static str) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = requests.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let log = log.clone();
            tokio::spawn(async move {
                let head = read_head(&mut socket).await;
                log.lock().unwrap().push(summarize(&head));
                let response = format!(
                    "HTTP/1.1 {status} Stub\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    Stub { url, requests }
}

async fn read_head(socket: &mut tokio::net::TcpStream) -> String {
    let mut head = Vec::new();
    let mut buf = [0u8; 4096];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        match socket.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    String::from_utf8_lossy(&head).into_owned()
}

fn summarize(head: &str) -> String {
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or_default().to_string();
    let auth = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default();
    format!("{request_line} | {auth}")
}

/// The configuration the variables describe, with the lake pointed at [`NO_LAKE`].
pub(crate) fn config(vars: &[(&str, &str)]) -> Config {
    let mut all: HashMap<String, String> = HashMap::from([
        ("DATABASE_PASSWORD".to_string(), "test".to_string()),
        ("DATALAKE_ROOT".to_string(), NO_LAKE.to_string()),
    ]);
    for (key, value) in vars {
        all.insert(key.to_string(), value.to_string());
    }
    crate::config::load::from_vars(&|key| all.get(key).cloned())
        .unwrap()
        .0
}

/// Home Assistant at `url` with a token.
pub(crate) fn with_home_assistant(url: &str) -> Vec<(&'static str, String)> {
    vec![
        ("HA_URL", url.to_string()),
        ("HA_TOKEN", HA_TOKEN.to_string()),
    ]
}

/// OpenWeatherMap configured (its address is set by [`service`]).
pub(crate) fn with_forecast() -> Vec<(&'static str, String)> {
    vec![
        ("OWM_API_KEY", "owm-test-key".to_string()),
        ("OWM_LATITUDE", "39.8".to_string()),
        ("OWM_LONGITUDE", "-75.8".to_string()),
    ]
}

/// The configuration from owned variable pairs.
pub(crate) fn config_of(vars: &[(&'static str, String)]) -> Config {
    let borrowed: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (*k, v.as_str())).collect();
    config(&borrowed)
}

/// A pool that never reaches a database.
pub(crate) fn unreachable_pool() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://postgres@127.0.0.1:1/none")
        .unwrap()
}

/// The service as startup builds it, with OpenWeatherMap (when configured) at `owm_url`.
pub(crate) async fn service(
    config: &Config,
    owm_url: Option<&str>,
    pool: PgPool,
) -> DataSyncService {
    let mut service = DataSyncService::initialize(config, pool).await;
    if let Some(url) = owm_url {
        service.openweathermap_client = service
            .openweathermap_client
            .map(|client| client.with_base_url(url));
    }
    service
}

/// The app state around `service` on the lawn's Eastern clock.
pub(crate) fn app_state(pool: PgPool, service: DataSyncService, mcp_enabled: bool) -> AppState {
    AppState::new(
        pool,
        service,
        None,
        mcp_enabled,
        Clock::Zone(chrono_tz::America::New_York),
    )
}

/// An app state on `pool` whose weather sources are all absent or unreadable.
pub(crate) async fn offline_state(pool: PgPool) -> AppState {
    let service = service(&config(&[]), None, pool.clone()).await;
    app_state(pool, service, false)
}
