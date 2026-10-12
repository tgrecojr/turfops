//! The shelf's vocabularies, the product record and fact scoping.

use super::*;
use crate::models::FracClass;
use std::str::FromStr;

// @spec INV-SHELF-002
#[test]
fn a_product_is_in_exactly_one_of_nine_categories() {
    use ProductCategory as C;
    assert_eq!(
        ProductCategory::ALL,
        [
            C::Fertilizer,
            C::Supplement,
            C::Fungicide,
            C::Herbicide,
            C::InsectControl,
            C::Seed,
            C::SoilAmendment,
            C::Surfactant,
            C::Other,
        ]
    );
    let json = serde_json::json!({ "category": "Seed", "form": "Seed" });
    let facts: ProductFacts = serde_json::from_value(json).unwrap();
    assert_eq!(facts.category, C::Seed);
    let two = serde_json::json!({ "category": ["Seed", "Fertilizer"], "form": "Seed" });
    assert!(
        serde_json::from_value::<ProductFacts>(two).is_err(),
        "one category, not a list"
    );
}

fn spelled_the_same<T>(all: &[T])
where
    T: serde::Serialize + FromStr + PartialEq + std::fmt::Debug + Copy + std::fmt::Display,
{
    for v in all {
        let json = serde_json::to_value(v).unwrap();
        assert_eq!(json.as_str().unwrap(), v.to_string(), "serde name = as_str");
        assert_eq!(
            T::from_str(&v.to_string()).ok(),
            Some(*v),
            "as_str parses back"
        );
        assert_eq!(
            crate::db::queries::enum_to_db_string(*v).unwrap(),
            v.to_string(),
            "database string = as_str"
        );
    }
}

// @spec INV-SHELF-003
#[test]
fn every_vocabulary_value_has_one_spelling_in_json_and_the_database() {
    spelled_the_same(ProductCategory::ALL);
    spelled_the_same(ProductForm::ALL);
    spelled_the_same(StockStatus::ALL);
    spelled_the_same(HerbicideTiming::ALL);
    spelled_the_same(AmendmentKind::ALL);
    spelled_the_same(RateUnit::ALL);
    spelled_the_same(ProductTarget::ALL);
    spelled_the_same(Mobility::ALL);
    spelled_the_same(NitrogenRelease::ALL);
    spelled_the_same(SuitableFor::ALL);
}

// @spec INV-SHELF-004
#[test]
fn targets_are_a_closed_list_of_pests_weeds_nutrients_and_species() {
    let names: Vec<&str> = ProductTarget::ALL.iter().map(|t| t.as_str()).collect();
    assert_eq!(
        names,
        [
            "Grubs",
            "SurfaceInsects",
            "Ants",
            "Termites",
            "Ticks",
            "Mosquitoes",
            "Broadleaf",
            "Crabgrass",
            "Nutsedge",
            "Poa",
            "Moss",
            "Iron",
            "Manganese",
            "Magnesium",
            "Zinc",
            "Boron",
            "Copper",
            "Calcium",
            "Kbg",
            "Ttf",
            "Prg",
            "FineFescue",
            "Bermuda",
            "Zoysia",
        ]
    );
    assert!(serde_json::from_value::<ProductTarget>(serde_json::json!("Dandelion")).is_err());
}

fn everything(category: ProductCategory) -> ProductFacts {
    let mut f = ProductFacts::manual(category, ProductForm::Granular);
    f.frac_classes = vec![FracClass::Frac11];
    f.herbicide_timing = Some(HerbicideTiming::PreEmergent);
    f.amendment_kind = Some(AmendmentKind::Gypsum);
    f.targets = ProductTarget::ALL.to_vec();
    f.nitrogen_pct = Some(10.0);
    f.phosphorus_pct = Some(5.0);
    f.potassium_pct = Some(5.0);
    f.label_rate_per_1000sqft = Some(2.5);
    f.label_rate_unit = Some(RateUnit::Oz);
    f
}

// @spec INV-SHELF-007
#[test]
fn scoping_keeps_only_the_facts_that_apply_to_the_category() {
    use ProductCategory as C;
    use ProductTarget as T;
    let pests = [
        T::Grubs,
        T::SurfaceInsects,
        T::Ants,
        T::Termites,
        T::Ticks,
        T::Mosquitoes,
    ];
    let weeds = [T::Broadleaf, T::Crabgrass, T::Nutsedge, T::Poa, T::Moss];
    let nutrients = [
        T::Iron,
        T::Manganese,
        T::Magnesium,
        T::Zinc,
        T::Boron,
        T::Copper,
        T::Calcium,
    ];
    let species = [T::Kbg, T::Ttf, T::Prg, T::FineFescue, T::Bermuda, T::Zoysia];
    for c in ProductCategory::ALL {
        let s = everything(*c).scoped_to_category();
        let other = *c == C::Other;
        assert_eq!(
            !s.frac_classes.is_empty(),
            other || *c == C::Fungicide,
            "{c} FRAC"
        );
        assert_eq!(
            s.herbicide_timing.is_some(),
            other || *c == C::Herbicide,
            "{c} timing"
        );
        assert_eq!(
            s.amendment_kind.is_some(),
            other || *c == C::SoilAmendment,
            "{c} kind"
        );
        let family: Vec<T> = match c {
            C::InsectControl => pests.to_vec(),
            C::Herbicide => weeds.to_vec(),
            C::Fertilizer | C::Supplement => nutrients.to_vec(),
            C::Seed => species.to_vec(),
            C::Other => T::ALL.to_vec(),
            C::Fungicide | C::SoilAmendment | C::Surfactant => vec![],
        };
        assert_eq!(s.targets, family, "{c} targets");
        assert_eq!(
            (s.nitrogen_pct, s.phosphorus_pct, s.potassium_pct),
            (Some(10.0), Some(5.0), Some(5.0))
        );
        assert_eq!(
            (s.label_rate_per_1000sqft, s.label_rate_unit),
            (Some(2.5), Some(RateUnit::Oz))
        );
    }
}

// @spec INV-SHELF-001
#[test]
fn a_product_records_its_fields_and_no_quantity() {
    let now = chrono::Utc::now();
    let p = Product {
        id: Some(1),
        lawn_profile_id: 1,
        name: "Milorganite".into(),
        brand: Some("Milorganite".into()),
        stock_status: StockStatus::Low,
        facts: ProductFacts::manual(ProductCategory::Fertilizer, ProductForm::Granular),
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: Some("in the shed".into()),
        archived: false,
        created_at: now,
        updated_at: now,
    };
    let json = serde_json::to_value(&p).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    let mut expected = vec![
        "id",
        "lawn_profile_id",
        "name",
        "brand",
        "stock_status",
        "profile",
        "profile_generated_at",
        "profile_model",
        "notes",
        "archived",
        "created_at",
        "updated_at",
        "category",
        "form",
        "label_rate_per_1000sqft",
        "label_rate_unit",
        "nitrogen_pct",
        "phosphorus_pct",
        "potassium_pct",
        "frac_classes",
        "herbicide_timing",
        "targets",
        "amendment_kind",
    ];
    expected.sort();
    assert_eq!(keys, expected);
    assert!(keys
        .iter()
        .all(|k| !k.contains("quantity") && !k.contains("amount")));
    let stock: Vec<&str> = StockStatus::ALL.iter().map(|s| s.as_str()).collect();
    assert_eq!(stock, ["InStock", "Low", "Out"]);
}
