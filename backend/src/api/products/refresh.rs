//! Regenerate a product's LLM profile without touching the user's facts: the response
//! carries what the new profile *would* change, for the user to apply or ignore.
use super::{active_profile, load};
use crate::datasources::openrouter::ProductProfileRequest;
use crate::db::product_queries;
use crate::error::TurfOpsError;
use crate::models::frac_class::frac_classes_for_product;
use crate::models::product::*;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProductRefreshed {
    pub product: Product,
    /// The facts the fresh profile implies (curated FRAC resolver already applied).
    pub suggested_facts: ProductFacts,
    /// Which of the product's current facts differ from `suggested_facts`.
    pub changed_fields: Vec<&'static str>,
}

pub async fn refresh_product_profile(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProductRefreshed>, TurfOpsError> {
    let openrouter = state
        .openrouter
        .as_ref()
        .ok_or_else(|| {
            TurfOpsError::DataSourceUnavailable(
                "OpenRouter not configured — set OPENROUTER_API_KEY to generate product profiles"
                    .into(),
            )
        })?
        .clone();

    let product = load(&state, id).await?;
    let lawn = active_profile(&state).await?;
    let lookup = product.lookup_name();

    let profile = openrouter
        .generate_product_profile(ProductProfileRequest {
            name: &lookup,
            category_hint: Some(product.facts.category),
            grass_type: lawn.grass_type.as_str(),
            usda_zone: &lawn.usda_zone,
        })
        .await?;

    product_queries::update_product_profile(&state.pool, id, &profile, openrouter.model()).await?;

    let suggested_facts =
        ProductFacts::from_profile(&profile, frac_classes_for_product(&product.name));
    let product = load(&state, id).await?;
    let changed_fields = product.facts.changed_fields(&suggested_facts);
    Ok(Json(ProductRefreshed {
        product,
        suggested_facts,
        changed_fields,
    }))
}
