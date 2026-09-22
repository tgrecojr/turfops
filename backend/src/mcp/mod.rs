//! Read-only MCP server over the lawn's data, served in-process at `/mcp` (streamable
//! HTTP) behind a bearer token. Tools call the REST handlers directly, so the answers
//! are exactly what the UI shows. Plan: `docs/plans/ai-assistant.md`.

mod auth;
mod config;
mod instructions;
mod resources;
mod tools;
mod trim;

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
    Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            Arc::<str>::from(token),
            auth::require_bearer,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
