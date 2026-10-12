//! Reading stored product rows whose vocabulary values the app no longer knows.

use super::*;

fn row() -> ProductRow {
    let now = Utc::now();
    ProductRow {
        id: 7,
        lawn_profile_id: 1,
        name: "Mystery".into(),
        brand: None,
        category: "Herbicide".into(),
        form: "Granular".into(),
        stock_status: "InStock".into(),
        label_rate_per_1000sqft: Some(2.0),
        label_rate_unit: Some("Lb".into()),
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        frac_classes: vec!["Frac11".into()],
        herbicide_timing: Some("PreEmergent".into()),
        targets: vec!["Crabgrass".into()],
        amendment_kind: Some("Gypsum".into()),
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: None,
        archived: false,
        created_at: now,
        updated_at: now,
    }
}

// @spec INV-SHELF-009
#[test]
fn unknown_stored_values_are_dropped_and_the_product_still_reads() {
    let mut r = row();
    r.label_rate_unit = Some("Bushel".into());
    r.frac_classes = vec!["Frac11".into(), "Frac99".into()];
    r.herbicide_timing = Some("Sometimes".into());
    r.targets = vec!["Dandelion".into(), "Crabgrass".into()];
    r.amendment_kind = Some("Sand".into());
    r.stock_status = "Plenty".into();
    let p = r.into_product().unwrap();
    assert_eq!(p.facts.label_rate_unit, None);
    assert_eq!(p.facts.frac_classes, vec![FracClass::Frac11]);
    assert_eq!(p.facts.herbicide_timing, None);
    assert_eq!(p.facts.targets, vec![ProductTarget::Crabgrass]);
    assert_eq!(p.facts.amendment_kind, None);
    assert_eq!(
        p.stock_status,
        StockStatus::InStock,
        "unknown stock reads as In stock"
    );
    assert_eq!(p.name, "Mystery");
}

#[test]
fn known_stored_values_read_back_unchanged() {
    let p = row().into_product().unwrap();
    assert_eq!(p.facts.label_rate_unit, Some(RateUnit::Lb));
    assert_eq!(p.facts.herbicide_timing, Some(HerbicideTiming::PreEmergent));
    assert_eq!(p.facts.amendment_kind, Some(AmendmentKind::Gypsum));
}
