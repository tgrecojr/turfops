//! `turfops-backend --healthcheck`: the container health check. The distroless runtime
//! has no shell or curl, so the binary probes its own `/api/v1/health`.

use std::net::SocketAddr;
use std::time::Duration;

/// Whether `GET /api/v1/health` at `addr` answers 200 within `timeout`.
// @spec DEPLOY-IMAGE-008
pub async fn probe(addr: SocketAddr, timeout: Duration) -> bool {
    let url = format!("http://{addr}/api/v1/health");
    let Ok(client) = reqwest::Client::builder().timeout(timeout).build() else {
        return false;
    };
    client
        .get(url)
        .send()
        .await
        .is_ok_and(|response| response.status() == reqwest::StatusCode::OK)
}

/// The address the server listens on, as the health check reaches it from inside the
/// container: loopback on the configured port.
pub fn local_addr() -> SocketAddr {
    let port = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|p| p.trim().parse().ok())
        .unwrap_or(3000);
    SocketAddr::from(([127, 0, 0, 1], port))
}

#[cfg(test)]
mod tests;
