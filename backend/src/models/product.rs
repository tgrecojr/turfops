//! A product on the shelf. `facts` are the user-owned, matchable columns; `profile` is the
//! LLM's description (optional — a product may be entered by hand and profiled later).
mod enums;
mod profile;

pub use enums::*;
pub use profile::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<i64>,
    pub lawn_profile_id: i64,
    pub name: String,
    pub brand: Option<String>,
    pub stock_status: StockStatus,
    #[serde(flatten)]
    pub facts: ProductFacts,
    pub profile: Option<ProductProfile>,
    pub profile_generated_at: Option<DateTime<Utc>>,
    pub profile_model: Option<String>,
    pub notes: Option<String>,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    /// The name the LLM should identify: brand + name when both are known.
    pub fn lookup_name(&self) -> String {
        lookup_name(self.brand.as_deref(), &self.name)
    }
}

/// "Brand Name" for the LLM, unless the name already carries the brand.
pub fn lookup_name(brand: Option<&str>, name: &str) -> String {
    match brand.map(str::trim).filter(|b| !b.is_empty()) {
        Some(b) if !name.to_lowercase().contains(&b.to_lowercase()) => format!("{} {}", b, name),
        _ => name.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::plant::IdentificationConfidence;
    use crate::models::FracClass;

    fn profile() -> ProductProfile {
        ProductProfile {
            identified_name: "Heritage G".into(),
            manufacturer: Some("Syngenta".into()),
            category: ProductCategory::Fungicide,
            form: ProductForm::Granular,
            active_ingredients: vec![ActiveIngredient {
                name: "azoxystrobin".into(),
                pct: Some(0.31),
            }],
            npk: None,
            frac_classes: vec![FracClass::Frac3],
            herbicide_timing: None,
            targets: vec![],
            amendment_kind: None,
            mobility: Mobility::Systemic,
            nitrogen_release: NitrogenRelease::NotApplicable,
            suitable_for: SuitableFor::Turf,
            suggested_label_rate: Some(SuggestedRate {
                amount: 2.0,
                unit: RateUnit::Lb,
            }),
            summary: "Strobilurin fungicide".into(),
            cautions: vec![],
            confidence: IdentificationConfidence::High,
        }
    }

    #[test]
    fn curated_frac_beats_the_llm() {
        // The LLM said FRAC 3 (wrong); the app's own table knows Heritage is FRAC 11.
        let facts = ProductFacts::from_profile(&profile(), vec![FracClass::Frac11]);
        assert_eq!(facts.frac_classes, vec![FracClass::Frac11]);
        assert_eq!(facts.label_rate_per_1000sqft, Some(2.0));
        assert_eq!(facts.label_rate_unit, Some(RateUnit::Lb));
    }

    #[test]
    fn llm_frac_fills_the_gap_when_nothing_is_curated() {
        let facts = ProductFacts::from_profile(&profile(), vec![]);
        assert_eq!(facts.frac_classes, vec![FracClass::Frac3]);
    }

    #[test]
    fn changed_fields_lists_only_differences() {
        let a = ProductFacts::from_profile(&profile(), vec![FracClass::Frac11]);
        let mut b = a.clone();
        assert!(a.changed_fields(&b).is_empty());
        b.frac_classes = vec![FracClass::Frac3];
        b.label_rate_per_1000sqft = None;
        b.label_rate_unit = None;
        assert_eq!(a.changed_fields(&b), vec!["label_rate", "frac_classes"]);
    }

    #[test]
    fn lookup_name_adds_brand_only_when_missing() {
        assert_eq!(
            lookup_name(Some("Syngenta"), "Heritage G"),
            "Syngenta Heritage G"
        );
        assert_eq!(
            lookup_name(Some("Scotts"), "Scotts Turf Builder"),
            "Scotts Turf Builder"
        );
        assert_eq!(lookup_name(Some("  "), " Milorganite "), "Milorganite");
    }

    #[test]
    fn product_json_is_flat() {
        let now = Utc::now();
        let p = Product {
            id: Some(1),
            lawn_profile_id: 1,
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
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["category"], "Fertilizer");
        assert!(json.get("facts").is_none());
    }
}
