//! Request checks for the product endpoints that need no database.

use crate::error::TurfOpsError;
use crate::models::frac_class::frac_classes_for_product;
use crate::models::product::{Product, ProductCategory, ProductFacts, ProductForm};

/// Whether `name` is already on the shelf (case-insensitive, trimmed, archived included),
/// ignoring the product being edited.
// @spec INV-API-013
pub(crate) fn name_taken(existing: &[Product], name: &str, editing: Option<i64>) -> bool {
    let wanted = name.trim().to_lowercase();
    existing
        .iter()
        .filter(|p| editing.is_none() || p.id != editing)
        .any(|p| p.name.trim().to_lowercase() == wanted)
}

/// The 409 for a name already on the shelf.
pub(crate) fn name_conflict(name: &str) -> TurfOpsError {
    TurfOpsError::Conflict(format!(
        "A product named \"{}\" is already on the shelf. Edit that one, or pick another name.",
        name.trim()
    ))
}

/// The facts for a product saved without an assistant profile: the owner's category and
/// form (Other when not given) and the curated FRAC classes for its name. With no category,
/// only a failed assistant saves the product (as Other); otherwise a category is required.
// @spec INV-LLM-011
pub(crate) fn facts_without_profile(
    category: Option<ProductCategory>,
    form: Option<ProductForm>,
    name: &str,
    assistant_failed: bool,
) -> Result<ProductFacts, TurfOpsError> {
    let category = match (category, assistant_failed) {
        (Some(category), _) => category,
        (None, true) => ProductCategory::Other,
        (None, false) => {
            return Err(TurfOpsError::InvalidData(
                "Pick a category to save the product without the assistant.".into(),
            ))
        }
    };
    let mut facts = ProductFacts::manual(category, form.unwrap_or(ProductForm::Other));
    facts.frac_classes = frac_classes_for_product(name);
    Ok(facts)
}
