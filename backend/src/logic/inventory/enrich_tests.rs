//! The feed's inventory pass: what it adds to a recommendation, what it never touches, and
//! that a product's label rate never reaches advice or the shopping list.

use super::*;
use crate::models::product::*;
use crate::models::{
    InventoryState, ProductNeed, RecommendationCategory, Severity, ShelfFungicide, ShelfProduct,
};
use chrono::Utc;

fn product(id: i64, name: &str, category: ProductCategory, stock: StockStatus) -> Product {
    let now = Utc::now();
    Product {
        id: Some(id),
        lawn_profile_id: 1,
        name: name.into(),
        brand: None,
        stock_status: stock,
        facts: ProductFacts::manual(category, ProductForm::Granular),
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: None,
        archived: false,
        created_at: now,
        updated_at: now,
    }
}

fn rec(id: &str, severity: Severity, needs: Vec<ProductNeed>) -> Recommendation {
    let mut r = Recommendation::new(id, RecommendationCategory::General, severity, "Title", "d");
    r.needs = needs;
    r
}

// @spec INV-ENRICH-003
#[test]
fn the_pass_records_the_status_and_one_data_point_per_match_and_gap() {
    let shelf = [
        product(1, "TTTF blend", ProductCategory::Seed, StockStatus::InStock),
        product(2, "Starter", ProductCategory::Seed, StockStatus::Low),
    ];
    let needs = vec![
        ProductNeed::new("grass seed", ProductCategory::Seed),
        ProductNeed::new("starter fertilizer", ProductCategory::Fertilizer),
    ];
    let mut recs = vec![rec("seed", Severity::Advisory, needs)];
    recs[0]
        .data_points
        .push(DataPoint::new("Soil temp", "68°F", "Lake"));
    enrich(&mut recs, &shelf);

    let r = &recs[0];
    assert_eq!(r.inventory.as_ref().unwrap().state, InventoryState::Partial);
    let points: Vec<(&str, &str, &str)> = r
        .data_points
        .iter()
        .map(|d| (d.label.as_str(), d.value.as_str(), d.source.as_str()))
        .collect();
    assert_eq!(
        points,
        [
            ("Soil temp", "68°F", "Lake"),
            ("On hand", "TTTF blend (grass seed)", "Inventory"),
            ("On hand", "Starter (grass seed)", "Inventory"),
            ("Not on shelf", "starter fertilizer", "Inventory"),
        ]
    );
}

// @spec INV-ENRICH-004
#[test]
fn the_pass_never_changes_identity_and_skips_recommendations_without_needs() {
    let shelf = [product(
        1,
        "GrubEx",
        ProductCategory::InsectControl,
        StockStatus::Out,
    )];
    let grub = ProductNeed::new("grub preventative", ProductCategory::InsectControl);
    let mut recs = vec![
        rec("grub_control", Severity::Warning, vec![grub]),
        rec("mowing", Severity::Info, vec![]),
    ];
    let before = recs.clone();
    enrich(&mut recs, &shelf);

    for (after, was) in recs.iter().zip(&before) {
        assert_eq!(after.id, was.id);
        assert_eq!(after.severity, was.severity);
        assert_eq!(after.title, was.title);
    }
    assert_eq!(
        recs[0].inventory.as_ref().unwrap().state,
        InventoryState::NotOnHand
    );
    let untouched = serde_json::to_value(&recs[1]).unwrap();
    assert_eq!(untouched, serde_json::to_value(&before[1]).unwrap());
}

// @spec INV-RATE-001
#[test]
fn a_products_label_rate_never_reaches_advice_or_the_shopping_list() {
    let mut fungicide = product(
        1,
        "Heritage G",
        ProductCategory::Fungicide,
        StockStatus::Low,
    );
    fungicide.facts.label_rate_per_1000sqft = Some(7.77);
    fungicide.facts.label_rate_unit = Some(RateUnit::FlOz);
    let mut seed = product(2, "TTTF blend", ProductCategory::Seed, StockStatus::Out);
    seed.facts.label_rate_per_1000sqft = Some(6.66);
    seed.facts.label_rate_unit = Some(RateUnit::Lb);
    let shelf = [fungicide.clone(), seed];
    let mut recs = vec![
        rec(
            "disease_brown_patch",
            Severity::Warning,
            vec![ProductNeed::new("fungicide", ProductCategory::Fungicide)],
        ),
        rec(
            "timing_fall_seeding_2026",
            Severity::Advisory,
            vec![ProductNeed::new("grass seed", ProductCategory::Seed)],
        ),
    ];
    enrich(&mut recs, &shelf);
    let list = shopping::build(&recs, &shelf);
    let disease_view = ShelfFungicide {
        product: ShelfProduct::of(&fungicide).unwrap(),
        classes: fungicide.facts.frac_classes.clone(),
    };

    let text = format!(
        "{} {} {}",
        serde_json::to_string(&recs).unwrap(),
        serde_json::to_string(&list).unwrap(),
        serde_json::to_string(&disease_view).unwrap(),
    );
    assert!(!list.items.is_empty() && recs[0].inventory.is_some());
    for rate in ["7.77", "6.66", "label_rate", "fl oz", "FlOz"] {
        assert!(!text.contains(rate), "{rate} leaked: {text}");
    }
}
