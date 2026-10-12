//! The patio-sensor reader against a local stub of the Home Assistant states API.

use super::*;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const TEMP: &str = "sensor.patio_temperature";
const HUMIDITY: &str = "sensor.patio_humidity";

/// A local server answering `/api/states/<entity>` with `(status, state)` per entity;
/// anything else is a 404.
async fn stub_states(routes: Vec<(&'static str, u16, &'static str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0u8; 16 * 1024];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let (status, body) = routes
                .iter()
                .find(|(entity, _, _)| path == format!("/api/states/{entity}"))
                .map(|(entity, status, state)| {
                    let body = format!(r#"{{"entity_id":"{entity}","state":"{state}"}}"#);
                    (*status, body)
                })
                .unwrap_or((404, "{}".to_string()));
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    format!("http://{addr}")
}

fn client(url: String, unit: TemperatureUnit) -> HomeAssistantClient {
    HomeAssistantClient::new(HomeAssistantConfig {
        url,
        token: "token".into(),
        temperature_entity: TEMP.into(),
        humidity_entity: HUMIDITY.into(),
        temperature_unit: unit,
    })
}

/// Log lines written while the returned guard is held (current thread only).
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_logs() -> (Captured, tracing::subscriber::DefaultGuard) {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    (captured, tracing::subscriber::set_default(subscriber))
}

impl Captured {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).to_string()
    }
}

// @spec WX-SRC-HA-001
#[tokio::test]
async fn both_entities_are_read_and_celsius_is_converted() {
    let routes = vec![(TEMP, 200, "25.0"), (HUMIDITY, 200, "61.5")];
    let before = Utc::now();
    let reading = client(stub_states(routes.clone()).await, TemperatureUnit::Celsius)
        .fetch_current()
        .await
        .unwrap();
    assert_eq!(reading.ambient_temp_f, Some(77.0));
    assert_eq!(reading.humidity_percent, Some(61.5));
    assert_eq!(reading.source, DataSource::HomeAssistant);
    assert!(reading.timestamp >= before, "stamped with the server clock");

    let fahrenheit = client(stub_states(routes).await, TemperatureUnit::Fahrenheit)
        .fetch_current()
        .await
        .unwrap();
    assert_eq!(fahrenheit.ambient_temp_f, Some(25.0), "°F is taken as-is");
}

// @spec WX-SRC-HA-002
#[tokio::test]
async fn an_unavailable_entity_is_logged_and_left_empty() {
    let (logs, _guard) = capture_logs();
    let routes = vec![(TEMP, 200, "unavailable"), (HUMIDITY, 200, "48")];
    let reading = client(stub_states(routes).await, TemperatureUnit::Fahrenheit)
        .fetch_current()
        .await
        .expect("the reading is still returned");
    assert_eq!(reading.ambient_temp_f, None);
    assert_eq!(reading.humidity_percent, Some(48.0));
    let text = logs.text();
    assert!(text.contains("WARN") && text.contains(TEMP), "{text}");
}

// @spec WX-SRC-HA-002
#[tokio::test]
async fn a_failed_entity_request_is_logged_and_left_empty() {
    let (logs, _guard) = capture_logs();
    let routes = vec![(TEMP, 200, "70.2"), (HUMIDITY, 500, "")];
    let reading = client(stub_states(routes).await, TemperatureUnit::Fahrenheit)
        .fetch_current()
        .await
        .expect("the reading is still returned");
    assert_eq!(reading.ambient_temp_f, Some(70.2));
    assert_eq!(reading.humidity_percent, None);
    let text = logs.text();
    assert!(text.contains("WARN") && text.contains(HUMIDITY), "{text}");

    let unreachable = client("http://127.0.0.1:1".into(), TemperatureUnit::Fahrenheit)
        .fetch_current()
        .await
        .expect("an unreachable server still returns a reading");
    assert_eq!(
        (unreachable.ambient_temp_f, unreachable.humidity_percent),
        (None, None)
    );
}
