//! Disease risk: the overview across diseases and one disease in full.

use super::{json_result, to_value, tool_error};
use crate::api;
use crate::error::TurfOpsError;
use crate::mcp::{trim, TurfOpsMcp};
use axum::extract::State;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiseaseDetailParams {
    /// The disease: brown-patch, dollar-spot, pythium-blight, gray-leaf-spot or red-thread.
    pub slug: String,
}

#[tool_router(router = disease_tools, vis = "pub(in crate::mcp)")]
impl TurfOpsMcp {
    #[tool(
        description = "Turf disease risk for brown patch, dollar spot, Pythium blight, gray \
                       leaf spot and red thread: each disease's tier (Low / Moderate / High \
                       / Severe — the only thing comparable across diseases), native score, \
                       the weather factors behind it, what to do (action + headline), \
                       whether a logged fungicide still protects, and the recommended \
                       preventative FRAC class with any shelf products in it.",
        annotations(read_only_hint = true)
    )]
    async fn disease_risk(&self) -> Result<CallToolResult, ErrorData> {
        match to_value(api::disease_risk::get_disease_risk(State(
            self.state.clone(),
        )))
        .await
        {
            Ok(risk) => json_result(&trim::disease_overview(&risk)),
            Err(err) => Ok(tool_error(err)),
        }
    }

    #[tool(
        description = "One disease in full: the 11-day daily risk series (observed and \
                       forecast days flagged), factors, methodology, cultural practices, \
                       and the preventative and curative programs with every FRAC option \
                       (efficacy, last used, recommended, restricted, on the shelf). Use \
                       when the user asks what to spray or whether a product will work.",
        annotations(read_only_hint = true)
    )]
    async fn disease_detail(
        &self,
        Parameters(params): Parameters<DiseaseDetailParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let risk = match to_value(api::disease_risk::get_disease_risk(State(
            self.state.clone(),
        )))
        .await
        {
            Ok(risk) => risk,
            Err(err) => return Ok(tool_error(err)),
        };
        let slug = params
            .slug
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '_'], "-");
        match trim::disease_detail(&risk, &slug) {
            Some(detail) => json_result(&detail),
            None => Ok(tool_error(TurfOpsError::InvalidData(format!(
                "Unknown disease `{}`. Use one of: {}",
                params.slug,
                trim::disease_slugs(&risk).join(", ")
            )))),
        }
    }
}
