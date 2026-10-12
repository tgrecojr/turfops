//! SIGTERM stops the server gracefully.

use axum::routing::get;
use axum::Router;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// @spec DEPLOY-START-006
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_lets_the_request_in_flight_finish_then_stops() {
    // Installed first, so a SIGTERM can never take the default action and end the test run.
    let _keep = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();

    let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();
    let started_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let app = Router::new().route(
        "/slow",
        get(move || {
            let tx = started_tx.lock().unwrap().take();
            async move {
                if let Some(tx) = tx {
                    let _ = tx.send(());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                "done"
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(crate::shutdown_signal())
            .await
    });

    let client = tokio::spawn(async move {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /slow HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    });
    started_rx.await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let sent = std::process::Command::new("kill")
        .args(["-TERM", &std::process::id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());

    let response = client.await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.ends_with("done"), "finished: {response}");
    let stopped = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("the server exits after the request");
    assert!(stopped.unwrap().is_ok());
    assert!(
        TcpStream::connect(addr).await.is_err(),
        "no new connections"
    );
}
