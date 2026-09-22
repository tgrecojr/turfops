//! Resolve the shelf product an application was logged against.
use crate::db::product_queries;
use crate::error::TurfOpsError;
use crate::models::ApplicationType;
use crate::state::AppState;

/// The product must exist, belong to this lawn and accept the application type. Returns
/// the product's name to use when the request did not carry one, so the copy on the
/// application row survives a later rename or delete of the product.
pub async fn resolve(
    state: &AppState,
    lawn_profile_id: i64,
    product_id: Option<i64>,
    application_type: ApplicationType,
    product_name: Option<String>,
) -> Result<(Option<i64>, Option<String>), TurfOpsError> {
    let Some(id) = product_id else {
        return Ok((None, product_name));
    };
    let product = product_queries::get_product(&state.pool, id)
        .await?
        .filter(|p| p.lawn_profile_id == lawn_profile_id)
        .ok_or_else(|| TurfOpsError::NotFound(format!("Product {} not found", id)))?;
    if !product.facts.category.accepts(application_type) {
        return Err(TurfOpsError::InvalidData(format!(
            "{} is a {} product and cannot be logged as {}",
            product.name, product.facts.category, application_type
        )));
    }
    let name = product_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(product.name);
    Ok((Some(id), Some(name)))
}
