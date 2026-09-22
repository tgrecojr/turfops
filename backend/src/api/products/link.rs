//! Bridging the log and the shelf: "you've logged X but it isn't on the shelf" and
//! "link my past applications of X to this product".
use super::{active_profile, load, profile_id};
use crate::db::product_queries::{self, LoggedProductSuggestion};
use crate::error::TurfOpsError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

pub async fn logged_product_suggestions(
    State(state): State<AppState>,
) -> Result<Json<Vec<LoggedProductSuggestion>>, TurfOpsError> {
    let profile = active_profile(&state).await?;
    let rows =
        product_queries::logged_product_suggestions(&state.pool, profile_id(&profile)?).await?;
    Ok(Json(rows))
}

#[derive(Debug, Serialize)]
pub struct LinkedApplications {
    pub linked: u64,
}

pub async fn link_applications(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<LinkedApplications>, TurfOpsError> {
    let product = load(&state, id).await?;
    let linked = product_queries::link_applications_by_name(
        &state.pool,
        id,
        product.lawn_profile_id,
        &product.name,
    )
    .await?;
    Ok(Json(LinkedApplications { linked }))
}
