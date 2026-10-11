use super::checks::*;
use crate::error::TurfOpsError;
use crate::models::frac_class::FracClass;
use crate::models::product::*;
use chrono::Utc;

fn shelf_item(id: i64, name: &str, archived: bool) -> Product {
    let now = Utc::now();
    Product {
        id: Some(id),
        lawn_profile_id: 1,
        name: name.into(),
        brand: None,
        stock_status: StockStatus::InStock,
        facts: ProductFacts::manual(ProductCategory::Fertilizer, ProductForm::Granular),
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: None,
        archived,
        created_at: now,
        updated_at: now,
    }
}

// @spec INV-API-013
#[test]
fn a_name_already_on_the_shelf_is_taken() {
    let shelf = [
        shelf_item(1, "Milorganite", false),
        shelf_item(2, "Old Lime", true),
    ];
    assert!(name_taken(&shelf, "  milorganite ", None));
    assert!(
        name_taken(&shelf, "OLD LIME", None),
        "archived products count"
    );
    assert!(
        !name_taken(&shelf, "Milorganite", Some(1)),
        "renaming to itself"
    );
    assert!(name_taken(&shelf, "old lime", Some(1)));
    assert!(!name_taken(&shelf, "Scotts Turf Builder", None));
}

// @spec INV-LLM-011
#[test]
fn a_failed_assistant_with_no_category_saves_as_other() {
    let facts = facts_without_profile(None, None, "Heritage G", true).unwrap();
    assert_eq!(facts.category, ProductCategory::Other);
    assert_eq!(facts.form, ProductForm::Other);
    assert_eq!(facts.frac_classes, vec![FracClass::Frac11]);
    let granular = facts_without_profile(None, Some(ProductForm::Granular), "x", true).unwrap();
    assert_eq!(
        granular.form,
        ProductForm::Granular,
        "the owner's form is kept"
    );
}

#[test]
fn without_the_assistant_a_category_is_required() {
    assert!(matches!(
        facts_without_profile(None, None, "x", false),
        Err(TurfOpsError::InvalidData(_))
    ));
    let lime =
        facts_without_profile(Some(ProductCategory::SoilAmendment), None, "Lime", false).unwrap();
    assert_eq!(lime.category, ProductCategory::SoilAmendment);
}
