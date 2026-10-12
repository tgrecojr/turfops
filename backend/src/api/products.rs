//! Product inventory CRUD. Unlike plants, a product saves without the LLM: the profile is
//! optional and can be generated later (`products/refresh.rs`).
#[cfg(all(test, feature = "db-tests"))]
mod assistant_db_tests;
mod checks;
#[cfg(test)]
mod checks_tests;
#[cfg(all(test, feature = "db-tests"))]
mod crud_db_tests;
#[cfg(all(test, feature = "db-tests"))]
mod edit_db_tests;
mod link;
#[cfg(all(test, feature = "db-tests"))]
mod link_db_tests;
mod refresh;
#[cfg(all(test, feature = "db-tests"))]
mod test_state;

pub use link::{link_applications, logged_product_suggestions};
pub use refresh::refresh_product_profile;

use crate::datasources::openrouter::ProductProfileRequest;
use crate::db::{product_queries, queries};
use crate::error::{Source, TurfOpsError};
use crate::models::frac_class::frac_classes_for_product;
use crate::models::product::*;
use crate::models::LawnProfile;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub category: Option<ProductCategory>,
    #[serde(default)]
    pub include_archived: bool,
}

// @spec INV-API-001
pub async fn list_products(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Product>>, TurfOpsError> {
    let profile = active_profile(&state).await?;
    let products = product_queries::list_products_for_profile(
        &state.pool,
        profile_id(&profile)?,
        q.category,
        q.include_archived,
    )
    .await?;
    Ok(Json(products))
}

// @spec INV-API-002
pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Product>, TurfOpsError> {
    Ok(Json(load(&state, id).await?))
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub brand: Option<String>,
    /// Required when the assistant is off or unavailable; otherwise the LLM decides and
    /// the user's choice, if given, wins.
    pub category: Option<ProductCategory>,
    pub form: Option<ProductForm>,
    pub stock_status: Option<StockStatus>,
    pub notes: Option<String>,
    /// Ask the LLM for a profile (default true). Ignored when OpenRouter is not configured.
    pub assist: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ProductCreated {
    pub product: Product,
    /// Set when the assistant was asked for but failed; the product was saved by hand.
    pub profile_error: Option<String>,
}

// @spec INV-API-003, INV-API-004, INV-API-006, INV-LLM-001, INV-LLM-002, INV-LLM-003, INV-LLM-004, INV-SHELF-007
pub async fn create_product(
    State(state): State<AppState>,
    Json(req): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<ProductCreated>), TurfOpsError> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(TurfOpsError::InvalidData("Product name is required".into()));
    }
    let profile = active_profile(&state).await?;
    let lawn_profile_id = profile_id(&profile)?;
    ensure_name_free(&state, lawn_profile_id, &name, None).await?;

    let (ai, assistant_failure) = match (req.assist.unwrap_or(true), state.openrouter.as_ref()) {
        (true, Some(openrouter)) => {
            let lookup = lookup_name(req.brand.as_deref(), &name);
            match openrouter
                .generate_product_profile(ProductProfileRequest {
                    name: &lookup,
                    category_hint: req.category,
                    grass_type: profile.grass_type.as_str(),
                    usda_zone: &profile.usda_zone,
                })
                .await
            {
                Ok(p) => (Some((p, openrouter.model().to_string())), None),
                Err(e) => {
                    warn!(product = %lookup, error = %e, "Product profile generation failed");
                    // @spec INV-API-012
                    (None, Some(e.for_products()))
                }
            }
        }
        (true, None) if req.category.is_none() => {
            return Err(TurfOpsError::not_configured(Source::ProductAssistant));
        }
        _ => (None, None),
    };

    let mut facts = match &ai {
        Some((p, _)) => {
            // @spec INV-LLM-010
            let (facts, dropped) = ProductFacts::from_profile(p, frac_classes_for_product(&name))
                .without_out_of_range();
            if !dropped.is_empty() {
                warn!(product = %name, ?dropped, "Left out-of-range assistant suggestions out of the facts");
            }
            facts
        }
        None => checks::facts_without_profile(
            req.category,
            req.form,
            &name,
            assistant_failure.is_some(),
        )?,
    };
    // The user's own category / form beat the LLM's guess.
    if let Some(c) = req.category {
        facts.category = c;
    }
    if let Some(f) = req.form {
        facts.form = f;
    }
    let facts = facts.scoped_to_category();

    let now = Utc::now();
    let brand = req
        .brand
        .filter(|b| !b.trim().is_empty())
        .or_else(|| ai.as_ref().and_then(|(p, _)| p.manufacturer.clone()));
    let product = Product {
        id: None,
        lawn_profile_id,
        name,
        brand,
        stock_status: req.stock_status.unwrap_or(StockStatus::InStock),
        facts,
        profile_generated_at: ai.as_ref().map(|_| now),
        profile_model: ai.as_ref().map(|(_, m)| m.clone()),
        profile: ai.map(|(p, _)| p),
        notes: req.notes,
        archived: false,
        created_at: now,
        updated_at: now,
    };

    let id = product_queries::create_product(&state.pool, &product).await?;
    let created = load(&state, id).await?;
    Ok((
        StatusCode::CREATED,
        Json(ProductCreated {
            product: created,
            profile_error: assistant_failure.map(|_| {
                if req.category.is_some() {
                    SAVED_BY_HAND
                } else {
                    SAVED_AS_OTHER
                }
                .to_string()
            }),
        }),
    ))
}

/// What the owner sees when the assistant failed and no category was given.
const SAVED_AS_OTHER: &str = "The assistant couldn't identify this product, so it was saved \
     under Other. Edit it to set its category, or use Refresh profile to try again later.";

/// What the owner sees when the assistant failed but the product saved with their details.
const SAVED_BY_HAND: &str = "The assistant couldn't identify this product, so it was saved with \
     the details you entered. Use Refresh profile to try again later.";

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: String,
    pub brand: Option<String>,
    pub stock_status: StockStatus,
    #[serde(flatten)]
    pub facts: ProductFacts,
    pub notes: Option<String>,
    #[serde(default)]
    pub archived: bool,
}

// @spec INV-API-007, INV-API-008, INV-API-009, INV-SHELF-007, INV-SHELF-008
pub async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Product>, TurfOpsError> {
    if req.name.trim().is_empty() {
        return Err(TurfOpsError::InvalidData("Product name is required".into()));
    }
    if req.facts.label_rate_per_1000sqft.is_some() != req.facts.label_rate_unit.is_some() {
        return Err(TurfOpsError::InvalidData(
            "label rate needs both an amount and a unit".into(),
        ));
    }
    // @spec INV-API-011
    if let Some(problem) = req.facts.range_problem() {
        return Err(TurfOpsError::InvalidData(problem));
    }
    let lawn_profile_id = profile_id(&active_profile(&state).await?)?;
    ensure_name_free(&state, lawn_profile_id, &req.name, Some(id)).await?;
    let update = product_queries::ProductUpdate {
        name: req.name,
        brand: req.brand.filter(|b| !b.trim().is_empty()),
        stock_status: req.stock_status,
        facts: req.facts.scoped_to_category(),
        notes: req.notes.filter(|n| !n.trim().is_empty()),
        archived: req.archived,
    };
    product_queries::update_product(&state.pool, id, &update).await?;
    Ok(Json(load(&state, id).await?))
}

// @spec INV-API-010
pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    product_queries::delete_product(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 409 when another of the lawn's products (archived included) already has this name.
// @spec INV-API-013
async fn ensure_name_free(
    state: &AppState,
    lawn_profile_id: i64,
    name: &str,
    editing: Option<i64>,
) -> Result<(), TurfOpsError> {
    let shelf =
        product_queries::list_products_for_profile(&state.pool, lawn_profile_id, None, true)
            .await?;
    if checks::name_taken(&shelf, name, editing) {
        return Err(checks::name_conflict(name));
    }
    Ok(())
}

async fn load(state: &AppState, id: i64) -> Result<Product, TurfOpsError> {
    product_queries::get_product(&state.pool, id)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound(format!("Product {} not found", id)))
}

async fn active_profile(state: &AppState) -> Result<LawnProfile, TurfOpsError> {
    queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))
}

fn profile_id(profile: &LawnProfile) -> Result<i64, TurfOpsError> {
    profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))
}
