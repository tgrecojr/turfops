//! The MCP server over its real `/mcp` router: transport, server info, the tool list and
//! the resources. No tool here reaches a database or lake.

use super::test_support::*;
use super::{instructions, router};
use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};

async fn lazy_state() -> crate::state::AppState {
    state(unreachable_pool(), &Lake::missing("server")).await
}

async fn tool_list() -> Vec<Value> {
    let response = rpc(&lazy_state().await, "tools/list", json!({})).await;
    response["result"]["tools"].as_array().unwrap().clone()
}

fn tool<'a>(tools: &'a [Value], name: &str) -> &'a Value {
    tools.iter().find(|t| t["name"] == name).unwrap()
}

// @spec MCP-SRV-001
#[tokio::test]
async fn every_request_is_answered_alone_with_a_plain_json_body() {
    let state = lazy_state().await;
    let list = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/list", "params": {}});
    // No initialize first, no session id: a fresh server answers each request.
    let request = post(&list).body(Body::from(list.to_string())).unwrap();
    let (status, headers, text) = send(router(state.clone(), TOKEN), request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    assert!(headers.get("mcp-session-id").is_none(), "no session");
    let reply: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(reply["id"], 7);
    assert!(reply["result"]["tools"].is_array());

    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": PROTOCOL, "capabilities": {},
        "clientInfo": {"name": "test", "version": "1"}}});
    let request = post(&init).body(Body::from(init.to_string())).unwrap();
    let (status, headers, _) = send(router(state, TOKEN), request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers.get("mcp-session-id").is_none(),
        "initialize opens no session"
    );
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
}

// @spec MCP-SRV-002
#[tokio::test]
async fn a_lan_hostname_in_the_host_header_is_accepted() {
    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}});
    for host in [
        "fridge:3000",
        "localhost:3000",
        "turfops.example.org",
        "192.168.1.20:3000",
    ] {
        let request = post_to(host, &list)
            .body(Body::from(list.to_string()))
            .unwrap();
        let (status, _, text) = send(router(lazy_state().await, TOKEN), request).await;
        assert_eq!(status, StatusCode::OK, "{host}: {text}");
    }
}

// @spec MCP-SRV-003
#[tokio::test]
async fn initialize_reports_turfops_its_version_capabilities_and_the_doctrine() {
    let response = rpc(
        &lazy_state().await,
        "initialize",
        json!({"protocolVersion": PROTOCOL, "capabilities": {},
               "clientInfo": {"name": "test", "version": "1"}}),
    )
    .await;
    let result = &response["result"];
    assert_eq!(result["serverInfo"]["name"], "turfops");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
    assert!(
        result["capabilities"].get("prompts").is_none(),
        "no prompts"
    );
    assert_eq!(result["instructions"], instructions::INSTRUCTIONS);
}

// @spec MCP-SRV-004
#[tokio::test]
async fn every_listed_tool_is_annotated_read_only() {
    for tool in tool_list().await {
        assert_eq!(
            tool["annotations"]["readOnlyHint"], true,
            "{} is not read-only",
            tool["name"]
        );
    }
}

/// The tool routers' source: what handlers and queries the tools can reach.
const TOOL_SOURCES: [(&str, &str); 7] = [
    ("tools/mod.rs", include_str!("tools/mod.rs")),
    ("tools/lawn.rs", include_str!("tools/lawn.rs")),
    ("tools/season.rs", include_str!("tools/season.rs")),
    ("tools/weather.rs", include_str!("tools/weather.rs")),
    ("tools/disease.rs", include_str!("tools/disease.rs")),
    ("tools/log.rs", include_str!("tools/log.rs")),
    ("tools/shelf.rs", include_str!("tools/shelf.rs")),
];

// @spec MCP-SRV-004
#[test]
fn no_tool_reaches_a_handler_or_query_that_writes() {
    let writes = [
        "::create_",
        "::update_",
        "::delete_",
        "::patch_",
        "::refresh_",
        "force_refresh",
        "link_applications",
        "::insert_",
        "::upsert_",
        "::set_",
        "::mark_",
        "::save_",
        "::archive_",
        "get_or_refresh",
    ];
    for (file, source) in TOOL_SOURCES {
        let code = source.split("#[cfg(test)]").next().unwrap();
        for write in writes {
            assert!(!code.contains(write), "{file} calls `{write}`");
        }
    }
}

// @spec MCP-TOOL-001
#[tokio::test]
async fn exactly_the_eighteen_tools_are_listed() {
    let mut names: Vec<String> = tool_list()
        .await
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    let mut expected = vec![
        "lawn_profile",
        "lawn_snapshot",
        "current_conditions",
        "recommendations",
        "timing_windows",
        "seasonal_plan",
        "gdd",
        "soil_temp_outlook",
        "nitrogen_budget",
        "weather_history",
        "disease_risk",
        "disease_detail",
        "applications",
        "soil_tests",
        "soil_test_advice",
        "products",
        "shopping_list",
        "plants",
    ];
    expected.sort();
    assert_eq!(names, expected);
}

// @spec MCP-TOOL-002
#[tokio::test]
async fn no_tool_declares_an_output_schema() {
    for tool in tool_list().await {
        assert!(
            tool.get("outputSchema").is_none(),
            "{} declares an output schema",
            tool["name"]
        );
    }
}

// @spec MCP-TOOL-002
#[test]
fn handler_backed_tools_call_handlers_in_process_never_over_http() {
    for (file, source) in TOOL_SOURCES {
        let code = source.split("#[cfg(test)]").next().unwrap();
        for http in ["reqwest", "/api/v1", "http://", "hyper::"] {
            assert!(!code.contains(http), "{file} mentions `{http}`");
        }
    }
}

// @spec MCP-TOOL-003
#[tokio::test]
async fn only_the_disease_slug_is_a_required_parameter() {
    for tool in tool_list().await {
        let required: Vec<&str> = tool["inputSchema"]["required"]
            .as_array()
            .map(|r| r.iter().map(|v| v.as_str().unwrap()).collect())
            .unwrap_or_default();
        let expected: &[&str] = if tool["name"] == "disease_detail" {
            &["slug"]
        } else {
            &[]
        };
        assert_eq!(required, expected, "{}", tool["name"]);
    }
}

// @spec MCP-SNAP-008
#[tokio::test]
async fn the_snapshot_tells_the_model_to_call_it_first() {
    let tools = tool_list().await;
    let d = tool(&tools, "lawn_snapshot")["description"]
        .as_str()
        .unwrap();
    assert!(
        d.contains("CALL FIRST for any question about what to do on this lawn now or this season"),
        "{d}"
    );
}

// @spec MCP-TOOL-009
#[tokio::test]
async fn the_gdd_tool_points_to_weather_history_for_daily_values() {
    let tools = tool_list().await;
    let d = tool(&tools, "gdd")["description"].as_str().unwrap();
    assert!(d.contains("Daily values: use `weather_history`"), "{d}");
}

// @spec MCP-TOOL-021
#[tokio::test]
async fn the_products_tool_calls_the_profile_background_not_fact() {
    let tools = tool_list().await;
    let d = tool(&tools, "products")["description"].as_str().unwrap();
    assert!(
        d.contains("`profile` is AI-generated background, not fact"),
        "{d}"
    );
}

// @spec MCP-SRV-001
#[tokio::test]
async fn there_is_no_stream_to_open() {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/mcp")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .header(header::ACCEPT, "text/event-stream")
        .body(Body::empty())
        .unwrap();
    let (status, _, _) = send(router(lazy_state().await, TOKEN), request).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}
