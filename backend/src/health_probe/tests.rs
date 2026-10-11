use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A one-shot server that answers the first request with `response`.
async fn serve_once(response: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await;
        let _ = socket.write_all(response.as_bytes()).await;
    });
    addr
}

// @spec DEPLOY-IMAGE-008
#[tokio::test]
async fn a_health_answer_of_200_is_healthy() {
    let addr = serve_once("HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}").await;
    assert!(probe(addr, Duration::from_secs(2)).await);
}

// @spec DEPLOY-IMAGE-008
#[tokio::test]
async fn no_answer_or_an_error_is_unhealthy() {
    let error = serve_once("HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n").await;
    assert!(!probe(error, Duration::from_secs(2)).await);
    let closed = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap();
    assert!(!probe(closed, Duration::from_millis(500)).await);
}
