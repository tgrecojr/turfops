//! Declaring needs, fitting and matching products, and assessing a recommendation's needs.

use super::*;
use crate::models::plant::IdentificationConfidence;
use chrono::Utc;

fn product(id: i64, category: ProductCategory) -> Product {
    let now = Utc::now();
    Product {
        id: Some(id),
        lawn_profile_id: 1,
        name: format!("P{id}"),
        brand: None,
        stock_status: StockStatus::InStock,
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

// @spec INV-NEED-001
#[test]
fn a_source_declares_a_need_in_plain_words_with_any_of_lists() {
    let need = ProductNeed::new("grub preventative", ProductCategory::InsectControl)
        .or_category(ProductCategory::Other)
        .with_frac_classes(vec![FracClass::Frac11])
        .with_targets(vec![ProductTarget::Grubs, ProductTarget::SurfaceInsects])
        .with_timing(HerbicideTiming::PreEmergent)
        .optional();
    assert_eq!(need.label, "grub preventative");
    assert_eq!(
        need.categories,
        [ProductCategory::InsectControl, ProductCategory::Other]
    );
    assert_eq!(need.frac_classes, [FracClass::Frac11]);
    assert_eq!(
        need.targets,
        [ProductTarget::Grubs, ProductTarget::SurfaceInsects]
    );
    assert_eq!(need.herbicide_timing, Some(HerbicideTiming::PreEmergent));
    assert!(need.optional);
    assert!(need.amendment_kinds.is_empty());

    let plain = ProductNeed::new("lime", ProductCategory::SoilAmendment);
    assert!(!plain.optional, "required unless marked nice-to-have");
    let json = serde_json::json!({ "label": "lime", "categories": ["SoilAmendment"],
        "amendment_kinds": ["CalciticLime", "DolomiticLime"] });
    let read: ProductNeed = serde_json::from_value(json).unwrap();
    assert_eq!(read.amendment_kinds.len(), 2);
    assert!(!read.optional && read.herbicide_timing.is_none());
}

// @spec INV-NEED-002
#[test]
fn a_product_fits_only_its_categories_and_every_constrained_axis() {
    let mut fert = product(1, ProductCategory::Fertilizer);
    fert.facts.targets = vec![ProductTarget::Iron];
    let mut nothing = ProductNeed::new("x", ProductCategory::Fertilizer);
    nothing.categories.clear();
    assert!(!nothing.fits(&fert), "an empty category list fits nothing");
    assert!(ProductNeed::new("feed", ProductCategory::Fertilizer).fits(&fert));
    assert!(!ProductNeed::new("seed", ProductCategory::Seed).fits(&fert));
    let iron = ProductNeed::new("iron", ProductCategory::Supplement)
        .or_category(ProductCategory::Fertilizer)
        .with_targets(vec![ProductTarget::Manganese, ProductTarget::Iron]);
    assert!(iron.fits(&fert), "shares one target");
    let zinc = iron.clone().with_targets(vec![ProductTarget::Zinc]);
    assert!(!zinc.fits(&fert));

    let mut lime = product(2, ProductCategory::SoilAmendment);
    lime.facts.amendment_kind = Some(AmendmentKind::DolomiticLime);
    let mut need = ProductNeed::new("lime", ProductCategory::SoilAmendment);
    need.amendment_kinds = vec![AmendmentKind::CalciticLime, AmendmentKind::DolomiticLime];
    assert!(need.fits(&lime));
    need.amendment_kinds = vec![AmendmentKind::Sulfur];
    assert!(!need.fits(&lime));

    let mut both = product(3, ProductCategory::Herbicide);
    both.facts.herbicide_timing = Some(HerbicideTiming::Both);
    let post = ProductNeed::new("post", ProductCategory::Herbicide)
        .with_timing(HerbicideTiming::PostEmergent);
    assert!(post.fits(&both), "Both meets either timing");
    let mut fungicide = product(4, ProductCategory::Fungicide);
    fungicide.facts.frac_classes = vec![FracClass::Frac3, FracClass::Frac11];
    let rotate = ProductNeed::new("fungicide", ProductCategory::Fungicide);
    assert!(rotate
        .clone()
        .with_frac_classes(vec![FracClass::Frac11])
        .fits(&fungicide));
    assert!(!rotate
        .with_frac_classes(vec![FracClass::Frac7])
        .fits(&fungicide));
}

// @spec INV-NEED-003
#[test]
fn a_fact_not_recorded_never_meets_a_constraint_on_it() {
    let blank = |c| product(1, c);
    let fungicide = ProductNeed::new("f", ProductCategory::Fungicide)
        .with_frac_classes(vec![FracClass::Frac11]);
    let grubs = ProductNeed::new("g", ProductCategory::InsectControl)
        .with_targets(vec![ProductTarget::Grubs]);
    let pre =
        ProductNeed::new("p", ProductCategory::Herbicide).with_timing(HerbicideTiming::PreEmergent);
    let mut lime = ProductNeed::new("l", ProductCategory::SoilAmendment);
    lime.amendment_kinds = vec![AmendmentKind::CalciticLime];
    assert!(!fungicide.fits(&blank(ProductCategory::Fungicide)));
    assert!(!grubs.fits(&blank(ProductCategory::InsectControl)));
    assert!(!pre.fits(&blank(ProductCategory::Herbicide)));
    assert!(!lime.fits(&blank(ProductCategory::SoilAmendment)));
}

// @spec INV-NEED-004
#[test]
fn only_a_usable_product_matches_and_low_still_counts() {
    let need = ProductNeed::new("seed", ProductCategory::Seed);
    let mut p = product(1, ProductCategory::Seed);
    assert!(need.matches(&p));
    p.stock_status = StockStatus::Low;
    assert!(need.matches(&p), "Low is still on the shelf");
    p.stock_status = StockStatus::Out;
    assert!(
        !need.matches(&p) && need.fits(&p),
        "Out fits but does not match"
    );
    p.stock_status = StockStatus::InStock;
    p.archived = true;
    assert!(!need.matches(&p), "archived never matches");
}

// @spec INV-ENRICH-001
#[test]
fn assessing_lists_each_match_and_each_uncovered_required_need() {
    let seed = ProductNeed::new("grass seed", ProductCategory::Seed);
    let fert = ProductNeed::new("fertilizer", ProductCategory::Fertilizer);
    let wetting = ProductNeed::new("wetting agent", ProductCategory::Surfactant).optional();
    let soap = ProductNeed::new("soap", ProductCategory::Other).optional();
    let shelf = [
        product(1, ProductCategory::Seed),
        product(2, ProductCategory::Seed),
        product(3, ProductCategory::Surfactant),
    ];
    let status = InventoryStatus::assess(&[seed, fert, wetting, soap], &shelf).unwrap();
    let pairs: Vec<(&str, i64)> = status
        .matches
        .iter()
        .map(|m| (m.need.as_str(), m.product.product_id))
        .collect();
    assert_eq!(
        pairs,
        [("grass seed", 1), ("grass seed", 2), ("wetting agent", 3)]
    );
    assert_eq!(
        status.missing,
        ["fertilizer"],
        "the unmet optional is not missing"
    );
    assert_eq!(
        status.state,
        InventoryState::Partial,
        "1 of 2 required needs"
    );
}

// @spec INV-LLM-007
#[test]
fn matching_reads_the_facts_never_the_profile() {
    let mut p = product(1, ProductCategory::Fertilizer);
    p.profile = Some(ProductProfile {
        identified_name: "Heritage G".into(),
        manufacturer: None,
        category: ProductCategory::Fungicide,
        form: ProductForm::Granular,
        active_ingredients: vec![],
        npk: None,
        frac_classes: vec![FracClass::Frac11],
        herbicide_timing: None,
        targets: vec![ProductTarget::Iron],
        amendment_kind: None,
        mobility: Mobility::Systemic,
        nitrogen_release: NitrogenRelease::NotApplicable,
        suitable_for: SuitableFor::Turf,
        suggested_label_rate: None,
        summary: "s".into(),
        cautions: vec![],
        confidence: IdentificationConfidence::High,
    });
    let fungicide = ProductNeed::new("fungicide", ProductCategory::Fungicide)
        .with_frac_classes(vec![FracClass::Frac11]);
    assert!(
        !fungicide.fits(&p),
        "the profile's category and class are not facts"
    );
    let iron = ProductNeed::new("iron", ProductCategory::Fertilizer)
        .with_targets(vec![ProductTarget::Iron]);
    assert!(!iron.matches(&p), "the profile's target is not a fact");
    p.facts.targets = vec![ProductTarget::Iron];
    assert!(iron.matches(&p));
}
