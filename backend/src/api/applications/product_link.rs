//! Resolve the shelf product an application was logged against.
use crate::db::product_queries;
use crate::error::TurfOpsError;
use crate::models::product::Product;
use crate::models::ApplicationType;
use crate::state::AppState;

/// The product must exist, belong to this lawn and accept the application type. Returns
/// the product's name to use when the request did not carry one, so the copy on the
/// application row survives a later rename or delete of the product.
// @spec APP-LINK-003
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
    let product = product_queries::get_product(&state.pool, id).await?;
    let product = check_link(product, lawn_profile_id, id, application_type)?;
    let name = product_name
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(product.name);
    Ok((Some(id), Some(name)))
}

/// The form was opened before the product left the shelf: the owner picks again.
const NOT_ON_SHELF: &str = "That product is no longer on your shelf";

/// The linked product, if it is this lawn's and accepts the application type.
// @spec APP-LINK-001, APP-LINK-002
pub(super) fn check_link(
    product: Option<Product>,
    lawn_profile_id: i64,
    _id: i64,
    application_type: ApplicationType,
) -> Result<Product, TurfOpsError> {
    let product = product
        .filter(|p| p.lawn_profile_id == lawn_profile_id)
        .ok_or_else(|| TurfOpsError::InvalidData(NOT_ON_SHELF.into()))?;
    if !product.facts.category.accepts(application_type) {
        return Err(TurfOpsError::InvalidData(format!(
            "{} is a {} product and cannot be logged as {}",
            product.name, product.facts.category, application_type
        )));
    }
    Ok(product)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::product::*;
    use chrono::Utc;

    fn fertilizer(lawn: i64) -> Product {
        let now = Utc::now();
        Product {
            id: Some(5),
            lawn_profile_id: lawn,
            name: "Milorganite".into(),
            brand: None,
            stock_status: StockStatus::InStock,
            facts: ProductFacts::manual(ProductCategory::Fertilizer, ProductForm::Granular),
            profile: None,
            profile_generated_at: None,
            profile_model: None,
            notes: None,
            archived: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn shelf_message(result: Result<Product, TurfOpsError>) -> String {
        match result {
            Err(TurfOpsError::InvalidData(msg)) => msg,
            other => panic!("expected a 400, got ok={}", other.is_ok()),
        }
    }

    // @spec APP-LINK-001
    #[test]
    fn a_gone_or_foreign_product_is_a_400_the_owner_can_fix() {
        let gone = shelf_message(check_link(None, 1, 5, ApplicationType::Fertilizer));
        assert_eq!(gone, "That product is no longer on your shelf");
        let foreign = shelf_message(check_link(
            Some(fertilizer(2)),
            1,
            5,
            ApplicationType::Fertilizer,
        ));
        assert_eq!(foreign, "That product is no longer on your shelf");
        assert!(check_link(Some(fertilizer(1)), 1, 5, ApplicationType::Fertilizer).is_ok());
    }

    // @spec APP-LINK-002
    #[test]
    fn a_product_whose_category_cannot_be_logged_as_the_type_is_named() {
        let msg = shelf_message(check_link(
            Some(fertilizer(1)),
            1,
            5,
            ApplicationType::Fungicide,
        ));
        assert_eq!(
            msg,
            "Milorganite is a Fertilizer product and cannot be logged as Fungicide"
        );
        assert!(check_link(Some(fertilizer(1)), 1, 5, ApplicationType::PlantFertilizer).is_ok());
        assert!(
            check_link(Some(fertilizer(1)), 1, 5, ApplicationType::Other).is_ok(),
            "Other accepts anything"
        );
    }
}
