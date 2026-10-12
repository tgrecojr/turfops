//! Helpers shared by tests.

use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A one-shot HTTP server on localhost that answers the first request with `status` and a
/// JSON `body`, then closes. Returns its base URL (`http://127.0.0.1:<port>`).
pub async fn stub_http(status: u16, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0u8; 64 * 1024];
            let _ = socket.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 {status} Upstream\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    format!("http://{addr}")
}

/// The production module tree: every file reachable from `main.rs` through `mod` items
/// that are not gated on `test`. Test modules and test helpers are left out by how they are
/// declared, whatever their file names.
fn walk(file: &Path, out: &mut Vec<PathBuf>) {
    out.push(file.to_path_buf());
    let text = std::fs::read_to_string(file).unwrap();
    let stem = file.file_stem().unwrap().to_string_lossy();
    let dir = if matches!(stem.as_ref(), "main" | "lib" | "mod") {
        file.parent().unwrap().to_path_buf()
    } else {
        file.with_extension("")
    };
    let mut test_gated = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("#[cfg(") {
            test_gated = line.contains("test");
            continue;
        }
        let decl = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub(super) "))
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
            if !test_gated {
                let flat = dir.join(format!("{name}.rs"));
                let nested = dir.join(name).join("mod.rs");
                walk(if flat.exists() { &flat } else { &nested }, out);
            }
        }
        if !line.starts_with("#[") {
            test_gated = false;
        }
    }
}

/// Every production source file, from `src/main.rs` down.
pub fn production_sources() -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"),
        &mut files,
    );
    assert!(files.len() > 50, "walked the module tree: {}", files.len());
    files
}

/// A production file's code without its inline `#[cfg(test)] mod … { … }` block (which
/// sits at the end of the file by convention).
pub fn production_code(file: &Path) -> String {
    let text = std::fs::read_to_string(file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let cut = lines
        .windows(2)
        .position(|w| {
            w[0].trim().starts_with("#[cfg(")
                && w[0].contains("test")
                && w[1].trim_end().ends_with('{')
                && w[1].trim_start().starts_with("mod ")
        })
        .unwrap_or(lines.len());
    lines[..cut].join("\n")
}

/// Keep a second tracing dispatcher alive for the whole test process. With exactly one
/// dispatcher registered, tracing-core asks only the *registering thread's* default
/// subscriber whether a log site is interesting; a test thread with no subscriber that
/// reaches a site first then caches it as "never", and another thread's log capture
/// silently sees nothing. With two, every live dispatcher is asked.
pub fn keep_tracing_interest_shared() {
    static SECOND: std::sync::OnceLock<tracing::Dispatch> = std::sync::OnceLock::new();
    SECOND.get_or_init(|| tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default()));
}
