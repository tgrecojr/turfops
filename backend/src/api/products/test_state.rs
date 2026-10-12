//! Shared setup for the products handler db-tests: an `AppState` over the test pool with no
//! external sources, a lawn profile, and a one-shot OpenRouter stand-in that records the
//! request it was sent.

use crate::clock::Clock;
use crate::config::*;
use crate::datasources::OpenRouterClient;
use crate::db::queries;
use crate::logic::data_sync::DataSyncService;
use crate::models::{GrassType, LawnProfile};
use crate::state::AppState;
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

fn config() -> Config {
    Config {
        lawn: LawnConfig {
            name: "Test".into(),
            grass_type: "TallFescue".into(),
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: None,
            irrigation_type: None,
        },
        noaa: NoaaConfig {
            station_wbanno: 3761,
        },
        datalake: DataLakeConfig {
            silver_weather_path: "/nonexistent/silver.parquet".into(),
            gold_weather_path: "/nonexistent/gold.parquet".into(),
        },
        homeassistant: HomeAssistantConfig {
            url: String::new(),
            token: String::new(),
            temperature_entity: String::new(),
            humidity_entity: String::new(),
            temperature_unit: Default::default(),
        },
        openweathermap: None,
        openrouter: None,
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            cors_allowed_origin: None,
        },
        database: DatabaseConfig {
            host: String::new(),
            port: 0,
            name: String::new(),
            user: String::new(),
            password: String::new(),
            max_connections: 1,
        },
        mcp: McpConfig { token: None },
        clock: Clock::System,
    }
}

/// An app over `pool` with no weather sources, and OpenRouter at `openrouter_url` when given.
pub(crate) async fn state(pool: PgPool, openrouter_url: Option<String>) -> AppState {
    let config = config();
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    let openrouter = openrouter_url.map(|base_url| {
        OpenRouterClient::new(OpenRouterConfig {
            api_key: "test".into(),
            model: "test/model".into(),
            enabled: true,
            base_url,
        })
    });
    AppState::new(pool, sync, openrouter, false, Clock::System)
}

/// The lawn every product belongs to (Kentucky bluegrass, zone 6b).
pub(crate) async fn lawn(pool: &PgPool) -> i64 {
    let profile = LawnProfile::new("Front".into(), GrassType::KentuckyBluegrass, "6b".into());
    queries::create_lawn_profile(pool, &profile).await.unwrap()
}

/// A one-shot OpenRouter stand-in: answers the first chat completion with `content` as the
/// model's message and hands back the JSON request body it received.
pub(crate) async fn openrouter_stub(content: String) -> (String, oneshot::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let body = read_request_body(&mut socket).await;
        let _ = tx.send(body);
        let reply =
            serde_json::json!({ "choices": [{ "message": { "content": content } }] }).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{reply}",
            reply.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
    });
    (format!("http://{addr}"), rx)
}

async fn read_request_body(socket: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = socket.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case("content-length")
                        .then(|| v.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            if buf.len() >= end + 4 + length {
                return String::from_utf8_lossy(&buf[end + 4..end + 4 + length]).into_owned();
            }
        }
    }
    String::new()
}

/// A profile reply as the model would send it.
pub(crate) fn profile_json(category: &str, manufacturer: Option<&str>) -> String {
    serde_json::json!({
        "identified_name": "Heritage G",
        "manufacturer": manufacturer,
        "category": category,
        "form": "Granular",
        "active_ingredients": [{ "name": "azoxystrobin", "pct": 0.31 }],
        "npk": { "n": 0.0, "p": 0.0, "k": 0.0 },
        "frac_classes": ["Frac3"],
        "herbicide_timing": "PostEmergent",
        "targets": ["Iron", "Broadleaf"],
        "amendment_kind": "Humic",
        "mobility": "Systemic",
        "nitrogen_release": "NotApplicable",
        "suitable_for": "Turf",
        "suggested_label_rate": { "amount": 2.0, "unit": "Lb" },
        "summary": "A strobilurin fungicide.",
        "cautions": [],
        "confidence": "High"
    })
    .to_string()
}
