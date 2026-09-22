//! The feed's inventory pass: for every recommendation that declares needs, ask the shelf
//! and record the answer. Runs after every source has contributed and before the user's
//! persisted answers are applied, so ids and severities are untouched.
use crate::models::{DataPoint, InventoryStatus, Product, Recommendation};

const SOURCE: &str = "Inventory";

pub fn enrich(recommendations: &mut [Recommendation], products: &[Product]) {
    for rec in recommendations.iter_mut().filter(|r| !r.needs.is_empty()) {
        let Some(status) = InventoryStatus::assess(&rec.needs, products) else {
            continue;
        };
        for m in &status.matches {
            rec.data_points.push(DataPoint::new(
                "On hand",
                format!("{} ({})", m.product.name, m.need),
                SOURCE,
            ));
        }
        for label in &status.missing {
            rec.data_points
                .push(DataPoint::new("Not on shelf", label, SOURCE));
        }
        rec.inventory = Some(status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::product::*;
    use crate::models::{InventoryState, ProductNeed, RecommendationCategory, Severity};
    use chrono::Utc;

    fn rec(id: &str, needs: Vec<ProductNeed>) -> Recommendation {
        let mut r = Recommendation::new(
            id,
            RecommendationCategory::General,
            Severity::Advisory,
            "t",
            "d",
        );
        r.needs = needs;
        r
    }

    #[test]
    fn only_recommendations_with_needs_are_touched_and_ids_stay_put() {
        let now = Utc::now();
        let seed = Product {
            id: Some(7),
            lawn_profile_id: 1,
            name: "TTTF blend".into(),
            brand: None,
            stock_status: StockStatus::InStock,
            facts: ProductFacts::manual(ProductCategory::Seed, ProductForm::Seed),
            profile: None,
            profile_generated_at: None,
            profile_model: None,
            notes: None,
            archived: false,
            created_at: now,
            updated_at: now,
        };
        let mut recs = vec![
            rec("plain", vec![]),
            rec(
                "seed",
                vec![ProductNeed::new("grass seed", ProductCategory::Seed)],
            ),
            rec(
                "fert",
                vec![ProductNeed::new("fertilizer", ProductCategory::Fertilizer)],
            ),
        ];
        enrich(&mut recs, &[seed]);
        assert!(recs[0].inventory.is_none() && recs[0].data_points.is_empty());
        let seed_status = recs[1].inventory.as_ref().unwrap();
        assert_eq!(seed_status.state, InventoryState::OnHand);
        assert_eq!(recs[1].data_points[0].value, "TTTF blend (grass seed)");
        assert_eq!(
            recs[2].inventory.as_ref().unwrap().state,
            InventoryState::NotOnHand
        );
        assert_eq!(recs[2].data_points[0].label, "Not on shelf");
        assert_eq!(recs[1].id, "seed");
    }
}
