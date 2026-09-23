//! Product inventory CRUD. Unlike plants, a product saves without the LLM: the profile is
//! optional and can be generated later (`products/refresh.rs`).
mod link;
mod refresh;

pub use link::{link_applications, logged_product_suggestions};
pub use refresh::refresh_product_profile;

use crate::datasources::openrouter::ProductProfileRequest;
use crate::db::{product_queries, queries};
use crate::error::TurfOpsError;
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

    let (ai, profile_error) = match (req.assist.unwrap_or(true), state.openrouter.as_ref()) {
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
                    (None, Some(e.to_string()))
                }
            }
        }
        (true, None) if req.category.is_none() => {
            return Err(TurfOpsError::InvalidData(
                "The assistant is not configured (set OPENROUTER_API_KEY) — pick a category to save the product by hand".into(),
            ));
        }
        _ => (None, None),
    };

    let mut facts = match &ai {
        Some((p, _)) => ProductFacts::from_profile(p, frac_classes_for_product(&name)),
        None => {
            let category = req.category.ok_or_else(|| match &profile_error {
                Some(e) => TurfOpsError::DataSourceUnavailable(format!(
                    "Could not identify the product ({}); pick a category to save it by hand",
                    e
                )),
                None => TurfOpsError::InvalidData(
                    "category is required when the assistant is not used".into(),
                ),
            })?;
            let mut f = ProductFacts::manual(category, req.form.unwrap_or(ProductForm::Other));
            f.frac_classes = frac_classes_for_product(&name);
            f
        }
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
            profile_error,
        }),
    ))
}

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

pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, TurfOpsError> {
    product_queries::delete_product(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
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
