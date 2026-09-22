//! What a recommendation needs from the shelf, and what the shelf answers. A source
//! declares [`ProductNeed`]s; the feed's enrichment pass turns them into an
//! [`InventoryStatus`] the UI shows as On hand / Low / Need to buy / Partly on hand.
use super::product::*;
use super::FracClass;
use serde::{Deserialize, Serialize};

/// One thing a recommendation calls for. Categories, classes, targets and kinds are
/// each "any of"; an empty list is no constraint on that axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductNeed {
    /// Plain words for the UI and the shopping list: "pre-emergent herbicide".
    pub label: String,
    pub categories: Vec<ProductCategory>,
    #[serde(default)]
    pub frac_classes: Vec<FracClass>,
    #[serde(default)]
    pub herbicide_timing: Option<HerbicideTiming>,
    #[serde(default)]
    pub targets: Vec<ProductTarget>,
    #[serde(default)]
    pub amendment_kinds: Vec<AmendmentKind>,
    /// Nice to have: never makes the recommendation "need to buy" on its own.
    #[serde(default)]
    pub optional: bool,
}

impl ProductNeed {
    pub fn new(label: impl Into<String>, category: ProductCategory) -> Self {
        Self {
            label: label.into(),
            categories: vec![category],
            frac_classes: Vec::new(),
            herbicide_timing: None,
            targets: Vec::new(),
            amendment_kinds: Vec::new(),
            optional: false,
        }
    }

    pub fn or_category(mut self, category: ProductCategory) -> Self {
        self.categories.push(category);
        self
    }

    pub fn with_frac_classes(mut self, classes: Vec<FracClass>) -> Self {
        self.frac_classes = classes;
        self
    }

    pub fn with_timing(mut self, timing: HerbicideTiming) -> Self {
        self.herbicide_timing = Some(timing);
        self
    }

    pub fn with_targets(mut self, targets: Vec<ProductTarget>) -> Self {
        self.targets = targets;
        self
    }

    pub fn with_amendment_kinds(mut self, kinds: Vec<AmendmentKind>) -> Self {
        self.amendment_kinds = kinds;
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// A product satisfies the need when it is usable (not archived, not Out) and meets
    /// every constrained axis. Constraints are strict: a product with no FRAC class,
    /// no timing or no targets recorded does not satisfy a need that asks for one — a
    /// false "on hand" sends the user to the shed for the wrong bag.
    pub fn matches(&self, p: &Product) -> bool {
        !p.archived && p.stock_status != StockStatus::Out && self.fits(p)
    }

    /// The facts fit, whatever the stock status — what to restock when it runs out.
    pub fn fits(&self, p: &Product) -> bool {
        fn any<T: PartialEq>(need: &[T], have: &[T]) -> bool {
            need.is_empty() || need.iter().any(|n| have.contains(n))
        }
        let f = &p.facts;
        let timing_ok = match self.herbicide_timing {
            None => true,
            Some(t) => {
                matches!(f.herbicide_timing, Some(have) if have == t || have == HerbicideTiming::Both)
            }
        };
        let amendment_ok = self.amendment_kinds.is_empty()
            || f.amendment_kind
                .is_some_and(|k| self.amendment_kinds.contains(&k));
        self.categories.contains(&f.category)
            && any(&self.frac_classes, &f.frac_classes)
            && any(&self.targets, &f.targets)
            && timing_ok
            && amendment_ok
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryState {
    OnHand,
    Low,
    NotOnHand,
    Partial,
}

/// A product on the shelf, as referenced from a recommendation or a fungicide option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShelfProduct {
    pub product_id: i64,
    pub name: String,
    pub stock_status: StockStatus,
}

impl ShelfProduct {
    pub fn of(p: &Product) -> Option<Self> {
        Some(Self {
            product_id: p.id?,
            name: p.name.clone(),
            stock_status: p.stock_status,
        })
    }
}

/// A fungicide on the shelf with its classes — what the disease planner sees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShelfFungicide {
    pub product: ShelfProduct,
    pub classes: Vec<FracClass>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryMatch {
    /// The need's label.
    pub need: String,
    #[serde(flatten)]
    pub product: ShelfProduct,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryStatus {
    pub state: InventoryState,
    pub matches: Vec<InventoryMatch>,
    /// Labels of the required needs nothing on the shelf covers.
    pub missing: Vec<String>,
}

impl InventoryStatus {
    /// `None` when there is nothing to say: no needs, or only optional ones unmet.
    pub fn assess(needs: &[ProductNeed], products: &[Product]) -> Option<Self> {
        let mut matches = Vec::new();
        let mut missing = Vec::new();
        let mut required = 0;
        let mut covered = 0;
        for need in needs {
            let found: Vec<InventoryMatch> = products
                .iter()
                .filter(|p| need.matches(p))
                .filter_map(|p| {
                    Some(InventoryMatch {
                        need: need.label.clone(),
                        product: ShelfProduct::of(p)?,
                    })
                })
                .collect();
            if !need.optional {
                required += 1;
                if found.is_empty() {
                    missing.push(need.label.clone());
                } else {
                    covered += 1;
                }
            }
            matches.extend(found);
        }
        if required == 0 && matches.is_empty() {
            return None;
        }
        let state = if required > 0 && covered == 0 {
            InventoryState::NotOnHand
        } else if covered < required {
            InventoryState::Partial
        } else if matches
            .iter()
            .all(|m| m.product.stock_status == StockStatus::Low)
        {
            InventoryState::Low
        } else {
            InventoryState::OnHand
        };
        Some(Self {
            state,
            matches,
            missing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn product(id: i64, category: ProductCategory, status: StockStatus) -> Product {
        let now = Utc::now();
        Product {
            id: Some(id),
            lawn_profile_id: 1,
            name: format!("P{id}"),
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

    #[test]
    fn constrained_axes_are_strict_and_out_never_counts() {
        let need = ProductNeed::new("fungicide", ProductCategory::Fungicide)
            .with_frac_classes(vec![FracClass::Frac11, FracClass::Frac3]);
        let mut heritage = product(1, ProductCategory::Fungicide, StockStatus::InStock);
        heritage.facts.frac_classes = vec![FracClass::Frac11];
        let unknown = product(2, ProductCategory::Fungicide, StockStatus::InStock);
        let mut out = heritage.clone();
        out.stock_status = StockStatus::Out;
        assert!(need.matches(&heritage));
        assert!(
            !need.matches(&unknown),
            "no class recorded ≠ the right class"
        );
        assert!(!need.matches(&out));
    }

    #[test]
    fn timing_accepts_both_and_rejects_the_other_direction() {
        let need = ProductNeed::new("pre-emergent", ProductCategory::Herbicide)
            .with_timing(HerbicideTiming::PreEmergent);
        let mut pre = product(1, ProductCategory::Herbicide, StockStatus::InStock);
        pre.facts.herbicide_timing = Some(HerbicideTiming::PreEmergent);
        let mut both = pre.clone();
        both.facts.herbicide_timing = Some(HerbicideTiming::Both);
        let mut post = pre.clone();
        post.facts.herbicide_timing = Some(HerbicideTiming::PostEmergent);
        assert!(need.matches(&pre) && need.matches(&both) && !need.matches(&post));
    }

    #[test]
    fn status_is_on_hand_low_partial_or_not_on_hand() {
        let fert = ProductNeed::new("fertilizer", ProductCategory::Fertilizer);
        let seed = ProductNeed::new("seed", ProductCategory::Seed);
        let low_fert = product(1, ProductCategory::Fertilizer, StockStatus::Low);
        let seed_bag = product(2, ProductCategory::Seed, StockStatus::InStock);

        let both = InventoryStatus::assess(
            &[fert.clone(), seed.clone()],
            &[low_fert.clone(), seed_bag.clone()],
        )
        .unwrap();
        assert_eq!(both.state, InventoryState::OnHand);
        assert_eq!(both.matches.len(), 2);

        let only_low =
            InventoryStatus::assess(std::slice::from_ref(&fert), std::slice::from_ref(&low_fert))
                .unwrap();
        assert_eq!(only_low.state, InventoryState::Low);

        let partial = InventoryStatus::assess(&[fert.clone(), seed.clone()], &[seed_bag]).unwrap();
        assert_eq!(partial.state, InventoryState::Partial);
        assert_eq!(partial.missing, vec!["fertilizer"]);

        let none = InventoryStatus::assess(&[fert], &[]).unwrap();
        assert_eq!(none.state, InventoryState::NotOnHand);
        assert!(InventoryStatus::assess(&[], &[low_fert]).is_none());
    }

    #[test]
    fn an_optional_need_never_makes_it_need_to_buy() {
        let wetting = ProductNeed::new("wetting agent", ProductCategory::Surfactant).optional();
        assert!(InventoryStatus::assess(std::slice::from_ref(&wetting), &[]).is_none());
        let bottle = product(1, ProductCategory::Surfactant, StockStatus::InStock);
        let status = InventoryStatus::assess(&[wetting], &[bottle]).unwrap();
        assert_eq!(status.state, InventoryState::OnHand);
        assert!(status.missing.is_empty());
    }
}
