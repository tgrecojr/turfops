//! The two method documents as MCP resources, over the real `/mcp` router.

use super::test_support::*;
use serde_json::json;

const AGRONOMY: &str = include_str!("../../../docs/agronomy-methods.md");
const TIMING: &str = include_str!("../../../docs/timing-windows.md");

async fn lazy_state() -> crate::state::AppState {
    state(unreachable_pool(), &Lake::missing("resources")).await
}

// @spec MCP-RES-001
#[tokio::test]
async fn exactly_the_two_method_documents_are_listed_with_their_sizes() {
    let response = rpc(&lazy_state().await, "resources/list", json!({})).await;
    let listed = response["result"]["resources"].as_array().unwrap();
    assert_eq!(listed.len(), 2);
    for (uri, name, text) in [
        ("turfops://docs/agronomy", "agronomy-methods", AGRONOMY),
        ("turfops://docs/timing-windows", "timing-windows", TIMING),
    ] {
        let r = listed.iter().find(|r| r["uri"] == uri).expect(uri);
        assert_eq!(r["name"], name);
        assert_eq!(r["mimeType"], "text/markdown");
        assert_eq!(r["size"], text.len());
    }
}

// @spec MCP-RES-002
#[tokio::test]
async fn reading_a_listed_uri_returns_the_whole_document_as_markdown() {
    let state = lazy_state().await;
    for (uri, text) in [
        ("turfops://docs/agronomy", AGRONOMY),
        ("turfops://docs/timing-windows", TIMING),
    ] {
        let response = rpc(&state, "resources/read", json!({"uri": uri})).await;
        let contents = response["result"]["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["uri"], uri);
        assert_eq!(contents[0]["mimeType"], "text/markdown");
        assert_eq!(contents[0]["text"], text);
    }
}

// @spec MCP-RES-003
#[tokio::test]
async fn an_unlisted_uri_is_a_resource_not_found_protocol_error() {
    let response = rpc(
        &lazy_state().await,
        "resources/read",
        json!({"uri": "turfops://docs/nope"}),
    )
    .await;
    assert!(response.get("result").is_none(), "{response}");
    // JSON-RPC code MCP assigns to "resource not found".
    assert_eq!(response["error"]["code"], -32002, "{response}");
}

// @spec MCP-RES-004
#[test]
fn the_documents_are_compiled_in_and_the_image_build_supplies_them() {
    let source = include_str!("resources.rs");
    for doc in ["agronomy-methods.md", "timing-windows.md"] {
        assert!(
            source.contains(&format!("include_str!(\"../../../docs/{doc}\")")),
            "{doc} is not embedded at build time"
        );
    }
    // The image's build stage copies both documents to where that path resolves, so a
    // missing document fails `cargo build` there too.
    let dockerfile = include_str!("../../../Dockerfile");
    let copy = dockerfile
        .lines()
        .find(|l| l.starts_with("COPY") && l.trim_end().ends_with("/docs/"))
        .expect("a COPY of the docs into /docs/");
    assert!(copy.contains("docs/agronomy-methods.md") && copy.contains("docs/timing-windows.md"));
}
