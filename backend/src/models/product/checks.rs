//! The ranges a product's facts must stay within: the label-rate amount above 0 and each
//! N-P-K percentage within 0–100 (the products table enforces the same with CHECKs).

use super::ProductFacts;

const RATE: &str = "The label rate amount must be more than 0.";

fn percent_out_of_range(value: Option<f64>) -> bool {
    value.is_some_and(|v| !(0.0..=100.0).contains(&v))
}

impl ProductFacts {
    /// The N, P and K fields with their owner-facing names.
    fn npk(&self) -> [(&'static str, Option<f64>); 3] {
        [
            ("Nitrogen (N)", self.nitrogen_pct),
            ("Phosphorus (P)", self.phosphorus_pct),
            ("Potassium (K)", self.potassium_pct),
        ]
    }

    /// The first value outside its allowed range, as a message for the owner.
    // @spec INV-API-011
    pub fn range_problem(&self) -> Option<String> {
        if self.label_rate_per_1000sqft.is_some_and(|r| r <= 0.0) {
            return Some(RATE.into());
        }
        self.npk()
            .into_iter()
            .find(|(_, v)| percent_out_of_range(*v))
            .map(|(name, _)| format!("{name} must be a percentage from 0 to 100."))
    }

    /// The facts with every out-of-range value removed (the label-rate amount and unit
    /// together; only the offending N, P or K percentage), and the names of what was removed.
    // @spec INV-LLM-010
    pub fn without_out_of_range(mut self) -> (Self, Vec<&'static str>) {
        let mut dropped = Vec::new();
        if self.label_rate_per_1000sqft.is_some_and(|r| r <= 0.0) {
            self.label_rate_per_1000sqft = None;
            self.label_rate_unit = None;
            dropped.push("label rate");
        }
        for (field, name) in [
            (&mut self.nitrogen_pct, "nitrogen"),
            (&mut self.phosphorus_pct, "phosphorus"),
            (&mut self.potassium_pct, "potassium"),
        ] {
            if percent_out_of_range(*field) {
                *field = None;
                dropped.push(name);
            }
        }
        (self, dropped)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::product::{ProductCategory, ProductFacts, ProductForm, RateUnit};

    fn fertilizer() -> ProductFacts {
        let mut f = ProductFacts::manual(ProductCategory::Fertilizer, ProductForm::Granular);
        f.label_rate_per_1000sqft = Some(3.0);
        f.label_rate_unit = Some(RateUnit::Lb);
        f.nitrogen_pct = Some(24.0);
        f.phosphorus_pct = Some(0.0);
        f.potassium_pct = Some(6.0);
        f
    }

    // @spec INV-API-011
    #[test]
    fn a_rate_of_zero_or_npk_outside_0_to_100_is_named() {
        assert_eq!(fertilizer().range_problem(), None);
        let mut f = fertilizer();
        f.label_rate_per_1000sqft = Some(0.0);
        let msg = f.range_problem().unwrap();
        assert!(
            msg.contains("label rate") && msg.contains("more than 0"),
            "{msg}"
        );
        let mut f = fertilizer();
        f.nitrogen_pct = Some(120.0);
        let msg = f.range_problem().unwrap();
        assert!(
            msg.contains("Nitrogen") && msg.contains("0 to 100"),
            "{msg}"
        );
        let mut f = fertilizer();
        f.potassium_pct = Some(-1.0);
        assert!(f.range_problem().unwrap().contains("Potassium"));
    }

    // @spec INV-LLM-010
    #[test]
    fn out_of_range_suggestions_are_left_out_one_value_at_a_time() {
        let mut f = fertilizer();
        f.label_rate_per_1000sqft = Some(-2.0);
        f.phosphorus_pct = Some(250.0);
        let (kept, dropped) = f.without_out_of_range();
        assert_eq!(kept.label_rate_per_1000sqft, None);
        assert_eq!(kept.label_rate_unit, None, "amount and unit go together");
        assert_eq!(kept.phosphorus_pct, None);
        assert_eq!(
            (kept.nitrogen_pct, kept.potassium_pct),
            (Some(24.0), Some(6.0))
        );
        assert_eq!(dropped.len(), 2);
        let (same, none) = fertilizer().without_out_of_range();
        assert!(none.is_empty());
        assert_eq!(same.label_rate_per_1000sqft, Some(3.0));
    }
}
