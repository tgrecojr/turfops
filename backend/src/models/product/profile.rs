//! The LLM's structured description of a product. Informational and regenerable; the
//! user-owned `ProductFacts` are copied out of it once, at save time.
use super::enums::*;
use crate::models::plant::IdentificationConfidence;
use crate::models::FracClass;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveIngredient {
    pub name: String,
    pub pct: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Npk {
    pub n: f64,
    pub p: f64,
    pub k: f64,
}

/// A label rate per 1,000 sq ft, only when the LLM says it is commonly published.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SuggestedRate {
    pub amount: f64,
    pub unit: RateUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductProfile {
    pub identified_name: String,
    pub manufacturer: Option<String>,
    pub category: ProductCategory,
    pub form: ProductForm,
    #[serde(default)]
    pub active_ingredients: Vec<ActiveIngredient>,
    pub npk: Option<Npk>,
    #[serde(default)]
    pub frac_classes: Vec<FracClass>,
    pub herbicide_timing: Option<HerbicideTiming>,
    #[serde(default)]
    pub targets: Vec<ProductTarget>,
    pub amendment_kind: Option<AmendmentKind>,
    pub mobility: Mobility,
    pub nitrogen_release: NitrogenRelease,
    pub suitable_for: SuitableFor,
    pub suggested_label_rate: Option<SuggestedRate>,
    pub summary: String,
    #[serde(default)]
    pub cautions: Vec<String>,
    pub confidence: IdentificationConfidence,
}

/// The matchable, user-owned facts about a product. Pre-filled from the profile, then
/// edited freely; the recommendation engine reads these, never the profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductFacts {
    pub category: ProductCategory,
    pub form: ProductForm,
    pub label_rate_per_1000sqft: Option<f64>,
    pub label_rate_unit: Option<RateUnit>,
    pub nitrogen_pct: Option<f64>,
    pub phosphorus_pct: Option<f64>,
    pub potassium_pct: Option<f64>,
    #[serde(default)]
    pub frac_classes: Vec<FracClass>,
    pub herbicide_timing: Option<HerbicideTiming>,
    #[serde(default)]
    pub targets: Vec<ProductTarget>,
    pub amendment_kind: Option<AmendmentKind>,
}

impl ProductFacts {
    /// A hand-entered product: the category (and optionally form) is all we know.
    pub fn manual(category: ProductCategory, form: ProductForm) -> Self {
        Self {
            category,
            form,
            label_rate_per_1000sqft: None,
            label_rate_unit: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            frac_classes: Vec::new(),
            herbicide_timing: None,
            targets: Vec::new(),
            amendment_kind: None,
        }
    }

    /// Facts derived from a profile. `curated_frac` is what the app's own product table
    /// (`frac_classes_for_product`) says; when it knows the product it beats the LLM.
    pub fn from_profile(profile: &ProductProfile, curated_frac: Vec<FracClass>) -> Self {
        let frac_classes = if curated_frac.is_empty() {
            profile.frac_classes.clone()
        } else {
            curated_frac
        };
        Self {
            category: profile.category,
            form: profile.form,
            label_rate_per_1000sqft: profile.suggested_label_rate.map(|r| r.amount),
            label_rate_unit: profile.suggested_label_rate.map(|r| r.unit),
            nitrogen_pct: profile.npk.map(|n| n.n),
            phosphorus_pct: profile.npk.map(|n| n.p),
            potassium_pct: profile.npk.map(|n| n.k),
            frac_classes,
            herbicide_timing: profile.herbicide_timing,
            targets: profile.targets.clone(),
            amendment_kind: profile.amendment_kind,
        }
    }

    /// Drop the facts that cannot apply to this category: FRAC classes belong to
    /// fungicides, timing to herbicides, an amendment kind to soil amendments, and
    /// targets to the family that fits (`ProductTarget::fits_category`). `Other` keeps
    /// everything. The LLM tags herbicides with FRAC classes and fungicides with nutrient
    /// targets; storing those as facts would let them match needs they cannot meet.
    pub fn scoped_to_category(mut self) -> Self {
        use ProductCategory as C;
        let category = self.category;
        if !matches!(category, C::Fungicide | C::Other) {
            self.frac_classes.clear();
        }
        if !matches!(category, C::Herbicide | C::Other) {
            self.herbicide_timing = None;
        }
        if !matches!(category, C::SoilAmendment | C::Other) {
            self.amendment_kind = None;
        }
        self.targets.retain(|t| t.fits_category(category));
        self
    }

    /// Names of the fields on which `other` differs — what a refreshed profile suggests
    /// changing, for the user to accept or ignore.
    pub fn changed_fields(&self, other: &ProductFacts) -> Vec<&'static str> {
        let mut changed = Vec::new();
        if self.category != other.category {
            changed.push("category");
        }
        if self.form != other.form {
            changed.push("form");
        }
        if self.label_rate_per_1000sqft != other.label_rate_per_1000sqft
            || self.label_rate_unit != other.label_rate_unit
        {
            changed.push("label_rate");
        }
        if self.nitrogen_pct != other.nitrogen_pct
            || self.phosphorus_pct != other.phosphorus_pct
            || self.potassium_pct != other.potassium_pct
        {
            changed.push("npk");
        }
        if self.frac_classes != other.frac_classes {
            changed.push("frac_classes");
        }
        if self.herbicide_timing != other.herbicide_timing {
            changed.push("herbicide_timing");
        }
        if self.targets != other.targets {
            changed.push("targets");
        }
        if self.amendment_kind != other.amendment_kind {
            changed.push("amendment_kind");
        }
        changed
    }
}
