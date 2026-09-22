//! What the owner has: the product shelf, the shopping list and landscape plants.

use super::{call, tool_error};
use crate::api;
use crate::api::products::ListQuery;
use crate::error::TurfOpsError;
use crate::mcp::TurfOpsMcp;
use crate::models::product::ProductCategory;
use axum::extract::{Query, State};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ProductsParams {
    /// Only this category: Fertilizer, Supplement, Fungicide, Herbicide, InsectControl,
    /// Seed, SoilAmendment, Surfactant or Other.
    pub category: Option<String>,
    /// Also list archived products (no longer used). Default false.
    #[serde(default)]
    pub include_archived: bool,
}

#[tool_router(router = shelf_tools, vis = "pub(in crate::mcp)")]
impl TurfOpsMcp {
    #[tool(
        description = "The product shelf: every product the owner has, with category, form, \
                       N-P-K, FRAC classes, herbicide timing (pre / post-emergent), targets, \
                       amendment kind, stock status (InStock / Low / Out — there are no \
                       quantities) and the stored label rate (quote it only with 'verify \
                       against your label'). `profile` is AI-generated background, not fact. \
                       Check this before suggesting any product.",
        annotations(read_only_hint = true)
    )]
    async fn products(
        &self,
        Parameters(params): Parameters<ProductsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let category = match params.category.as_deref().map(ProductCategory::from_str) {
            None => None,
            Some(Ok(category)) => Some(category),
            Some(Err(_)) => {
                return Ok(tool_error(TurfOpsError::InvalidData(format!(
                    "Unknown product category `{}`",
                    params.category.unwrap_or_default()
                ))))
            }
        };
        call(api::products::list_products(
            State(self.state.clone()),
            Query(ListQuery {
                category,
                include_archived: params.include_archived,
            }),
        ))
        .await
    }

    #[tool(
        description = "The shopping list built from the active recommendations: Buy (a \
                       need nothing on the shelf covers, with the recommendations that call \
                       for it) and Restock (Low / Out products an active recommendation \
                       needs), most urgent first.",
        annotations(read_only_hint = true)
    )]
    async fn shopping_list(&self) -> Result<CallToolResult, ErrorData> {
        call(api::inventory::get_shopping_list(State(self.state.clone()))).await
    }

    #[tool(
        description = "Landscape plants (shrubs, trees, perennials, …) with location, \
                       planting date and each plant's AI-generated care plan: task windows \
                       for pruning, fertilizing, mulching, deadheading and winter \
                       protection, plus warnings.",
        annotations(read_only_hint = true)
    )]
    async fn plants(&self) -> Result<CallToolResult, ErrorData> {
        call(api::plants::list_plants(State(self.state.clone()))).await
    }
}
