//! A one-shot stand-in for OpenRouter that records the request it receives, for tests of
//! what the client sends and of the handlers that call it.

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// The request the stub received: header lines (lowercased names) and the JSON body.
#[derive(Debug)]
pub(crate) struct Captured {
    pub head: String,
    pub body: serde_json::Value,
}

impl Captured {
    /// A header's value, by case-insensitive name.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        let wanted = name.to_ascii_lowercase();
        self.head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim().to_ascii_lowercase() == wanted).then(|| value.trim())
        })
    }

    /// The user message the client sent.
    pub(crate) fn user_prompt(&self) -> &str {
        self.body["messages"][1]["content"].as_str().unwrap_or("")
    }
}

/// A valid care plan, as the model would return it.
pub(crate) fn sample_plan() -> serde_json::Value {
    json!({
        "identified_name": "Panicle hydrangea",
        "scientific_name": "Hydrangea paniculata",
        "identification_confidence": "High",
        "summary": "A hardy flowering shrub.",
        "tasks": [{
            "task_type": "Pruning",
            "window_start_month_day": "02-15",
            "window_end_month_day": "03-31",
            "frequency": "Once",
            "description": "Prune before new growth; it blooms on new wood.",
            "severity": "Advisory",
            "zone_note": null
        }],
        "warnings": []
    })
}

/// A chat-completion body whose message content is `content` serialized as JSON text.
pub(crate) fn completion(content: &serde_json::Value) -> String {
    json!({ "choices": [{ "message": { "content": content.to_string() } }] }).to_string()
}

/// Answer the first request with `status` and `body`; returns the base URL and the
/// request the stub received once it has answered.
pub(crate) async fn capture(status: u16, body: String) -> (String, oneshot::Receiver<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let (head, raw_body) = read_request(&mut socket).await;
        let response = format!(
            "HTTP/1.1 {status} Stub\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
        let body = serde_json::from_slice(&raw_body).unwrap_or(serde_json::Value::Null);
        let _ = tx.send(Captured { head, body });
    });
    (format!("http://{addr}/v1"), rx)
}

/// Read one HTTP request: its head and a body of `Content-Length` bytes.
async fn read_request(socket: &mut tokio::net::TcpStream) -> (String, Vec<u8>) {
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 16 * 1024];
    loop {
        let n = socket.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let Some(split) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&buf[..split]).to_string();
        let length = head
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.trim()
                    .eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        if buf.len() >= split + 4 + length {
            return (head, buf[split + 4..split + 4 + length].to_vec());
        }
    }
    (String::from_utf8_lossy(&buf).to_string(), Vec::new())
}
