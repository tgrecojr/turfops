//! What the shopping list buys and restocks, for whom, and in what order.

use super::*;
use crate::logic::inventory::enrich;
use crate::models::product::*;
use crate::models::{ProductNeed, RecommendationCategory};

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

fn rec(id: &str, title: &str, severity: Severity, needs: Vec<ProductNeed>) -> Recommendation {
    let mut r = Recommendation::new(id, RecommendationCategory::General, severity, title, "d");
    r.needs = needs;
    r
}

/// The feed as the handler sees it: enriched against the shelf, then listed.
fn shop(mut recs: Vec<Recommendation>, shelf: &[Product]) -> Vec<(ShoppingKind, String)> {
    enrich(&mut recs, shelf);
    build(&recs, shelf)
        .items
        .into_iter()
        .map(|i| (i.kind, i.label))
        .collect()
}

fn seed() -> ProductNeed {
    ProductNeed::new("grass seed", ProductCategory::Seed)
}

// @spec INV-SHOP-001
#[test]
fn the_list_reads_only_active_advice_and_unarchived_products() {
    let mut old = product(1, "Old blend", ProductCategory::Seed, StockStatus::Out);
    old.archived = true;
    let mut answered = rec(
        "a",
        "Answered",
        Severity::Critical,
        vec![ProductNeed::new("lime", ProductCategory::SoilAmendment)],
    );
    answered.addressed = true;
    let mut dismissed = rec(
        "d",
        "Dismissed",
        Severity::Critical,
        vec![ProductNeed::new("sulfur", ProductCategory::SoilAmendment)],
    );
    dismissed.dismissed = true;
    let active = rec("s", "Fall Seeding", Severity::Advisory, vec![seed()]);
    assert_eq!(
        shop(vec![answered, dismissed, active], &[old]),
        [(ShoppingKind::Buy, "grass seed".to_string())],
        "no restock of the archived bag, nothing for answered advice"
    );
    let json = serde_json::to_value(build(&[], &[])).unwrap();
    let mut keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["generated_at", "items"]);
}

// @spec INV-SHOP-002
#[test]
fn every_low_or_out_product_that_fits_is_restocked_even_when_covered() {
    let shelf = [
        product(
            1,
            "Fresh blend",
            ProductCategory::Seed,
            StockStatus::InStock,
        ),
        product(2, "Low blend", ProductCategory::Seed, StockStatus::Low),
        product(3, "Empty blend", ProductCategory::Seed, StockStatus::Out),
        product(
            4,
            "Low lime",
            ProductCategory::SoilAmendment,
            StockStatus::Low,
        ),
    ];
    let recs = vec![
        rec("s", "Fall Seeding", Severity::Advisory, vec![seed()]),
        rec("o", "Overseed", Severity::Info, vec![seed()]),
    ];
    assert_eq!(
        shop(recs, &shelf),
        [
            (ShoppingKind::Restock, "Empty blend".to_string()),
            (ShoppingKind::Restock, "Low blend".to_string()),
        ]
    );
}

// @spec INV-SHOP-003
#[test]
fn a_missing_need_nothing_restocks_is_one_buy_line_per_label() {
    let feed = ProductNeed::new("nitrogen fertilizer", ProductCategory::Fertilizer)
        .or_category(ProductCategory::Supplement);
    let recs = vec![
        rec(
            "a",
            "Spring Feeding",
            Severity::Advisory,
            vec![feed.clone()],
        ),
        rec("b", "Fall Feeding", Severity::Warning, vec![feed]),
        rec(
            "c",
            "Grubs",
            Severity::Info,
            vec![ProductNeed::new(
                "grub control",
                ProductCategory::InsectControl,
            )],
        ),
    ];
    let mut shelf_recs = recs.clone();
    let shelf = [product(
        1,
        "GrubEx",
        ProductCategory::InsectControl,
        StockStatus::Out,
    )];
    enrich(&mut shelf_recs, &shelf);
    let list = build(&shelf_recs, &shelf);
    let buys: Vec<&ShoppingItem> = list
        .items
        .iter()
        .filter(|i| i.kind == ShoppingKind::Buy)
        .collect();
    assert_eq!(
        buys.len(),
        1,
        "the Out GrubEx is restocked instead of a grub Buy line"
    );
    assert_eq!(buys[0].label, "nitrogen fertilizer");
    assert_eq!(
        buys[0].categories,
        [ProductCategory::Fertilizer, ProductCategory::Supplement]
    );
    assert!(buys[0].product.is_none());
}

// @spec INV-SHOP-004
#[test]
fn an_optional_need_never_shops() {
    let wetting = ProductNeed::new("wetting agent", ProductCategory::Surfactant).optional();
    let irrigation = rec("i", "Irrigation", Severity::Critical, vec![wetting]);
    assert!(shop(vec![irrigation.clone()], &[]).is_empty(), "no Buy");
    let empty = product(
        1,
        "Hydretain",
        ProductCategory::Surfactant,
        StockStatus::Out,
    );
    assert!(shop(vec![irrigation], &[empty]).is_empty(), "no Restock");
}

// @spec INV-SHOP-005
#[test]
fn an_item_lists_each_calling_title_once_and_takes_the_highest_severity() {
    let mut recs = vec![
        rec("a", "Fall Seeding", Severity::Info, vec![seed()]),
        rec("b", "Bare Patch", Severity::Warning, vec![seed()]),
        rec("c", "Fall Seeding", Severity::Advisory, vec![seed()]),
    ];
    enrich(&mut recs, &[]);
    let list = build(&recs, &[]);
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].reasons, ["Fall Seeding", "Bare Patch"]);
    assert_eq!(list.items[0].urgency, Severity::Warning);
}

// @spec INV-SHOP-006
#[test]
fn items_sort_by_urgency_then_buy_before_restock_then_label() {
    let shelf = [
        product(1, "Aardvark seed", ProductCategory::Seed, StockStatus::Low),
        product(
            2,
            "Zed lime",
            ProductCategory::SoilAmendment,
            StockStatus::Out,
        ),
    ];
    let recs = vec![
        rec("s", "Seeding", Severity::Warning, vec![seed()]),
        rec(
            "f",
            "Feeding",
            Severity::Warning,
            vec![ProductNeed::new("zinc", ProductCategory::Fertilizer)],
        ),
        rec(
            "g",
            "Grubs",
            Severity::Warning,
            vec![ProductNeed::new(
                "grub control",
                ProductCategory::InsectControl,
            )],
        ),
        rec(
            "l",
            "Lime",
            Severity::Critical,
            vec![ProductNeed::new("lime", ProductCategory::SoilAmendment)],
        ),
        rec(
            "w",
            "Weeds",
            Severity::Info,
            vec![ProductNeed::new("herbicide", ProductCategory::Herbicide)],
        ),
    ];
    let order: Vec<(ShoppingKind, String)> = shop(recs, &shelf);
    let labels: Vec<&str> = order.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Zed lime",
            "grub control",
            "zinc",
            "Aardvark seed",
            "herbicide"
        ]
    );
}

// @spec INV-LLM-007
#[test]
fn the_shopping_list_reads_facts_not_the_profile() {
    let mut p = product(
        1,
        "Mislabelled",
        ProductCategory::Fertilizer,
        StockStatus::Out,
    );
    p.facts.category = ProductCategory::Fertilizer;
    let profile_json = serde_json::json!({
        "identified_name": "Seed", "manufacturer": null, "category": "Seed", "form": "Seed",
        "active_ingredients": [], "npk": null, "frac_classes": [], "herbicide_timing": null,
        "targets": ["Ttf"], "amendment_kind": null, "mobility": "NotApplicable",
        "nitrogen_release": "NotApplicable", "suitable_for": "Turf",
        "suggested_label_rate": null, "summary": "s", "cautions": [], "confidence": "High"
    });
    p.profile = Some(serde_json::from_value(profile_json).unwrap());
    let recs = vec![rec("s", "Seeding", Severity::Warning, vec![seed()])];
    assert_eq!(
        shop(recs, &[p]),
        [(ShoppingKind::Buy, "grass seed".to_string())],
        "the profile says Seed, the facts say Fertilizer: nothing to restock"
    );
}
