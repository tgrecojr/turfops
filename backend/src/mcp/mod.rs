//! Read-only MCP server over the lawn's data, served in-process at `/mcp` (streamable
//! HTTP) behind a bearer token. Tools call the REST handlers directly, so the answers
//! are exactly what the UI shows. Plan: `docs/plans/ai-assistant.md`.

mod auth;
mod config;
#[cfg(test)]
mod guard_tests;
mod instructions;
mod resources;
mod tools;
mod trim;
#[cfg(test)]
mod trim_model_tests;

pub use config::{parse_token, McpConfig};

use crate::state::AppState;
use axum::Router;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{
    Implementation, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{tool_handler, ErrorData, RoleServer, ServerHandler};
use std::sync::Arc;

#[derive(Clone)]
pub struct TurfOpsMcp {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

impl TurfOpsMcp {
    /// Every tool, from the per-area routers in `tools/`.
    fn tools() -> ToolRouter<Self> {
        Self::lawn_tools()
            + Self::season_tools()
            + Self::weather_tools()
            + Self::disease_tools()
            + Self::log_tools()
            + Self::shelf_tools()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TurfOpsMcp {
    fn get_info(&self) -> ServerConfig {
        server_config()
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(resources::list())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        resources::read(&request.uri).map(Into::into)
    }
}

fn server_config() -> ServerConfig {
    ServerConfig::new(
        ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .build(),
    )
    .with_server_info(Implementation::new("turfops", env!("CARGO_PKG_VERSION")))
    .with_instructions(instructions::INSTRUCTIONS)
}

/// The `/mcp` routes, guarded by `Authorization: Bearer <token>`.
///
/// Stateless with plain JSON responses: every tool is a read-only request/response, so
/// there is no session to keep and a restart never strands a client. `Host` validation
/// (rmcp's loopback-only default, against DNS rebinding) is off because the gateway calls
/// in by LAN hostname; the bearer token is the guard, and a rebinding page cannot send it.
pub fn router(state: AppState, token: &str) -> Router {
    let tools = TurfOpsMcp::tools();
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .disable_allowed_hosts();
    let service = StreamableHttpService::new(
        move || {
            Ok(TurfOpsMcp {
                state: state.clone(),
                tool_router: tools.clone(),
            })
        },
        Arc::new(LocalSessionManager::default()),
        config,
    );
    guarded(Router::new().nest_service("/mcp", service), token)
}

/// The same body limit as `/api/v1`.
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// The `/mcp` routes behind the bearer guard, then a POST-only gate and the body limit,
/// so an unauthenticated request learns nothing and no tool runs on an oversized body.
// @spec MCP-SRV-006, MCP-SRV-007
pub(crate) fn guarded(inner: Router, token: &str) -> Router {
    inner
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            MAX_BODY_BYTES,
        ))
        .layer(axum::middleware::from_fn(post_only))
        .layer(axum::middleware::from_fn_with_state(
            Arc::<str>::from(token),
            auth::require_bearer,
        ))
}

/// Stateless streamable HTTP with JSON responses needs only POST.
async fn post_only(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::{header, Method, StatusCode};
    use axum::response::IntoResponse;
    if request.method() != Method::POST {
        return (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "POST")]).into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec MCP-TOOL-023
    #[test]
    fn the_products_tool_reports_a_label_rate_only_when_asked() {
        let tools = TurfOpsMcp::tools().list_all();
        let products = tools.iter().find(|t| t.name == "products").unwrap();
        let d = products.description.as_deref().unwrap_or_default();
        assert!(d.contains("only when the owner asks"), "{d}");
        assert!(d.contains("verify against your label"), "{d}");
        assert!(d.contains("never inside advice"), "{d}");
    }

    // @spec MCP-DOC-005
    #[test]
    fn the_rates_rule_separates_label_rates_from_agronomic_amounts() {
        let doc = instructions::INSTRUCTIONS;
        for phrase in [
            "label rate",
            "fungicide, herbicide, insect control",
            "per 1,000 sq ft",
            "nitrogen budget",
            "lime or sulfur",
            "mulch depth",
            "also a pesticide active",
            "only when the owner asks",
            "verify against your label",
        ] {
            assert!(doc.contains(phrase), "doctrine lacks {phrase:?}");
        }
    }

    // @spec MCP-DOC-011
    #[test]
    fn staleness_comes_from_the_flag_not_a_limit_of_its_own() {
        let doc = instructions::INSTRUCTIONS;
        assert!(doc.contains("soil_stale"));
        assert!(doc.contains("data_notes"));
        assert!(
            !doc.contains("72 h"),
            "the doctrine must not state its own staleness limit"
        );
    }

    #[test]
    fn every_tool_has_a_description_and_is_read_only() {
        let tools = TurfOpsMcp::tools().list_all();
        assert!(!tools.is_empty());
        for tool in tools {
            let description = tool.description.as_deref().unwrap_or_default();
            assert!(
                !description.trim().is_empty(),
                "{} has no description",
                tool.name
            );
            let read_only = tool.annotations.as_ref().and_then(|a| a.read_only_hint);
            assert_eq!(
                read_only,
                Some(true),
                "{} is not marked read-only",
                tool.name
            );
        }
    }

    #[test]
    fn every_planned_tool_is_registered() {
        let names: Vec<String> = TurfOpsMcp::tools()
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in [
            "applications",
            "current_conditions",
            "disease_detail",
            "disease_risk",
            "gdd",
            "lawn_profile",
            "lawn_snapshot",
            "nitrogen_budget",
            "plants",
            "products",
            "recommendations",
            "seasonal_plan",
            "shopping_list",
            "soil_temp_outlook",
            "soil_test_advice",
            "soil_tests",
            "timing_windows",
            "weather_history",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing tool {expected}"
            );
        }
    }

    #[test]
    fn server_info_advertises_tools_and_instructions() {
        let info = server_config();
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert_eq!(info.server_info.name, "turfops");
        assert!(info.instructions.is_some_and(|i| !i.is_empty()));
    }

    #[test]
    fn instructions_lead_with_the_snapshot_and_forbid_invented_rates() {
        assert!(instructions::INSTRUCTIONS.contains("lawn_snapshot"));
        assert!(instructions::INSTRUCTIONS.contains("verify against your label"));
    }

    // @spec MCP-DOC-004
    #[test]
    fn instructions_name_one_clock_the_lawns_tz() {
        let text = instructions::INSTRUCTIONS;
        assert!(!text.contains("station-local"), "second clock named");
        assert!(text.contains("TZ"), "the lawn's clock is not named");
    }
}
