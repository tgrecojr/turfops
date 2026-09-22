//! Build the shopping list from the active feed and the shelf. Everything an active
//! recommendation calls for is already in its `needs`, including "opening soon" windows,
//! so the feed is the whole horizon — no separate look at the seasonal plan.
use crate::models::product::StockStatus;
use crate::models::{
    Product, Recommendation, Severity, ShelfProduct, ShoppingItem, ShoppingKind, ShoppingList,
};
use chrono::Utc;
use std::collections::BTreeMap;

pub fn build(active: &[Recommendation], products: &[Product]) -> ShoppingList {
    let mut buy: BTreeMap<String, ShoppingItem> = BTreeMap::new();
    let mut restock: BTreeMap<i64, ShoppingItem> = BTreeMap::new();

    for rec in active.iter().filter(|r| r.is_active()) {
        let missing: Vec<&str> = rec
            .inventory
            .as_ref()
            .map(|i| i.missing.iter().map(String::as_str).collect())
            .unwrap_or_default();
        for need in rec.needs.iter().filter(|n| !n.optional) {
            let restockable: Vec<&Product> = products
                .iter()
                .filter(|p| !p.archived && p.stock_status != StockStatus::InStock)
                .filter(|p| need.fits(p))
                .collect();
            // A Low / Out product that fits *is* the thing to buy; no generic line too.
            if missing.contains(&need.label.as_str()) && restockable.is_empty() {
                let item = buy
                    .entry(need.label.clone())
                    .or_insert_with(|| ShoppingItem {
                        kind: ShoppingKind::Buy,
                        label: need.label.clone(),
                        categories: need.categories.clone(),
                        product: None,
                        reasons: Vec::new(),
                        urgency: Severity::Info,
                    });
                add_reason(item, rec);
            }
            // A Low or Out product that fits this need wants restocking, whether or not
            // something else on the shelf still covers the need.
            for p in restockable {
                let Some(shelf) = ShelfProduct::of(p) else {
                    continue;
                };
                let item = restock
                    .entry(shelf.product_id)
                    .or_insert_with(|| ShoppingItem {
                        kind: ShoppingKind::Restock,
                        label: p.name.clone(),
                        categories: vec![p.facts.category],
                        product: Some(shelf),
                        reasons: Vec::new(),
                        urgency: Severity::Info,
                    });
                add_reason(item, rec);
            }
        }
    }

    let mut items: Vec<ShoppingItem> = buy.into_values().chain(restock.into_values()).collect();
    items.sort_by(|a, b| {
        b.urgency
            .cmp(&a.urgency)
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.label.cmp(&b.label))
    });
    ShoppingList {
        generated_at: Utc::now(),
        items,
    }
}

fn add_reason(item: &mut ShoppingItem, rec: &Recommendation) {
    if !item.reasons.contains(&rec.title) {
        item.reasons.push(rec.title.clone());
    }
    item.urgency = item.urgency.max(rec.severity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::inventory::enrich;
    use crate::models::product::*;
    use crate::models::{ProductNeed, RecommendationCategory};

    fn product(id: i64, name: &str, category: ProductCategory, status: StockStatus) -> Product {
        let now = Utc::now();
        Product {
            id: Some(id),
            lawn_profile_id: 1,
            name: name.into(),
            brand: None,
            stock_status: status,
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

    fn rec(id: &str, title: &str, severity: Severity, need: ProductNeed) -> Recommendation {
        Recommendation::new(id, RecommendationCategory::General, severity, title, "d")
            .with_need(need)
    }

    #[test]
    fn buys_what_is_missing_and_restocks_what_is_low_or_out() {
        let products = vec![
            product(1, "Ironite", ProductCategory::Supplement, StockStatus::Low),
            product(2, "Old seed", ProductCategory::Seed, StockStatus::Out),
            product(
                3,
                "Lime",
                ProductCategory::SoilAmendment,
                StockStatus::InStock,
            ),
        ];
        let mut recs = vec![
            rec(
                "a",
                "Iron Deficiency",
                Severity::Info,
                ProductNeed::new("iron supplement", ProductCategory::Supplement),
            ),
            rec(
                "b",
                "Fall Seeding — Open",
                Severity::Advisory,
                ProductNeed::new("grass seed", ProductCategory::Seed),
            ),
            rec(
                "c",
                "Fall Feeding",
                Severity::Warning,
                ProductNeed::new("nitrogen fertilizer", ProductCategory::Fertilizer),
            ),
            rec(
                "d",
                "pH Adjustment",
                Severity::Warning,
                ProductNeed::new("lime", ProductCategory::SoilAmendment),
            ),
            rec(
                "e",
                "Irrigation",
                Severity::Critical,
                ProductNeed::new("wetting agent", ProductCategory::Surfactant).optional(),
            ),
        ];
        enrich(&mut recs, &products);
        let list = build(&recs, &products);
        let summary: Vec<(ShoppingKind, &str, Severity)> = list
            .items
            .iter()
            .map(|i| (i.kind, i.label.as_str(), i.urgency))
            .collect();
        assert_eq!(
            summary,
            vec![
                (ShoppingKind::Buy, "nitrogen fertilizer", Severity::Warning),
                // The Out seed bag is the thing to buy — no separate "grass seed" line.
                (ShoppingKind::Restock, "Old seed", Severity::Advisory),
                (ShoppingKind::Restock, "Ironite", Severity::Info),
            ]
        );
        assert_eq!(list.items[2].reasons, vec!["Iron Deficiency"]);
        assert_eq!(list.items[2].product.as_ref().unwrap().product_id, 1);
    }

    #[test]
    fn answered_recommendations_do_not_shop() {
        let mut r = rec(
            "a",
            "Fall Feeding",
            Severity::Warning,
            ProductNeed::new("nitrogen fertilizer", ProductCategory::Fertilizer),
        );
        enrich(std::slice::from_mut(&mut r), &[]);
        r.dismissed = true;
        assert!(build(&[r], &[]).items.is_empty());
    }
}
