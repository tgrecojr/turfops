//! GET /api/v1/inventory/shopping-list — what to buy or restock, from the active feed.
use crate::db::{product_queries, queries};
use crate::error::TurfOpsError;
use crate::logic::inventory::shopping;
use crate::models::ShoppingList;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn get_shopping_list(
    State(state): State<AppState>,
) -> Result<Json<ShoppingList>, TurfOpsError> {
    let summary = {
        let mut service = state.sync_service.write().await;
        service.get_or_refresh().await?
    };
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let active = super::recommendation_feed::active(&state, &profile, &summary).await?;
    let products =
        product_queries::list_products_for_profile(&state.pool, profile_id, None, false).await?;
    Ok(Json(shopping::build(&active, &products)))
}
