//! Which application types a shelf product may be logged under.

use super::*;
use crate::models::ApplicationType;

/// Every application type; the match fails to compile when a variant is added.
fn all_application_types() -> Vec<ApplicationType> {
    use ApplicationType as A;
    let all = vec![
        A::PreEmergent,
        A::PostEmergent,
        A::Fertilizer,
        A::Fungicide,
        A::Insecticide,
        A::GrubControl,
        A::Overseed,
        A::Aeration,
        A::Dethatching,
        A::Lime,
        A::Sulfur,
        A::Wetting,
        A::Mowing,
        A::Other,
        A::Pruning,
        A::PlantFertilizer,
        A::Mulching,
        A::Deadheading,
        A::WinterProtection,
    ];
    for t in &all {
        match t {
            A::PreEmergent
            | A::PostEmergent
            | A::Fertilizer
            | A::Fungicide
            | A::Insecticide
            | A::GrubControl
            | A::Overseed
            | A::Aeration
            | A::Dethatching
            | A::Lime
            | A::Sulfur
            | A::Wetting
            | A::Mowing
            | A::Other
            | A::Pruning
            | A::PlantFertilizer
            | A::Mulching
            | A::Deadheading
            | A::WinterProtection => {}
        }
    }
    all
}

/// The application types each category may be logged under, as the spec tables them.
fn allowed(c: ProductCategory, t: ApplicationType) -> bool {
    use ApplicationType as A;
    use ProductCategory as C;
    t == A::Other
        || match c {
            C::Fertilizer | C::Supplement => matches!(t, A::Fertilizer | A::PlantFertilizer),
            C::Fungicide => t == A::Fungicide,
            C::Herbicide => matches!(t, A::PreEmergent | A::PostEmergent),
            C::InsectControl => matches!(t, A::GrubControl | A::Insecticide),
            C::Seed => t == A::Overseed,
            C::SoilAmendment => matches!(t, A::Lime | A::Sulfur),
            C::Surfactant => t == A::Wetting,
            C::Other => true,
        }
}

// @spec INV-SHELF-005
#[test]
fn each_category_is_logged_only_under_its_application_types() {
    for c in ProductCategory::ALL {
        for t in all_application_types() {
            assert_eq!(c.accepts(t), allowed(*c, t), "{c} under {t:?}");
        }
    }
}
