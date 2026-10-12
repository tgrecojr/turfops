//! Editing and storing products against PostgreSQL (`--features db-tests`).

use super::crud_db_tests::{add, edit_of, status_of};
use super::test_state::{lawn, state};
use super::*;
use crate::models::plant::IdentificationConfidence;
use crate::models::FracClass;
use sqlx::PgPool;

pub(super) fn stored(lawn_profile_id: i64, name: &str, facts: ProductFacts) -> Product {
    let now = Utc::now();
    Product {
        id: None,
        lawn_profile_id,
        name: name.into(),
        brand: Some("Syngenta".into()),
        stock_status: StockStatus::Low,
        facts,
        profile: Some(profile()),
        profile_generated_at: Some(now),
        profile_model: Some("test/model".into()),
        notes: Some("half a bag".into()),
        archived: false,
        created_at: now,
        updated_at: now,
    }
}

fn profile() -> ProductProfile {
    ProductProfile {
        identified_name: "Heritage G".into(),
        manufacturer: Some("Syngenta".into()),
        category: ProductCategory::Fungicide,
        form: ProductForm::Granular,
        active_ingredients: vec![],
        npk: None,
        frac_classes: vec![FracClass::Frac11],
        herbicide_timing: None,
        targets: vec![],
        amendment_kind: None,
        mobility: Mobility::Systemic,
        nitrogen_release: NitrogenRelease::NotApplicable,
        suitable_for: SuitableFor::Turf,
        suggested_label_rate: None,
        summary: "A strobilurin.".into(),
        cautions: vec![],
        confidence: IdentificationConfidence::High,
    }
}

// @spec INV-API-007, INV-SHELF-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_edit_replaces_owned_fields_and_never_the_profile(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let state = state(pool.clone(), None).await;
    let facts = ProductFacts::manual(ProductCategory::Fungicide, ProductForm::Granular);
    let id = product_queries::create_product(&pool, &stored(lawn_id, "Heritage G", facts))
        .await
        .unwrap();
    let before = product_queries::get_product(&pool, id)
        .await
        .unwrap()
        .unwrap();

    let mut facts = ProductFacts::manual(ProductCategory::Herbicide, ProductForm::Liquid);
    facts.herbicide_timing = Some(HerbicideTiming::PostEmergent);
    facts.frac_classes = vec![FracClass::Frac3];
    facts.targets = vec![ProductTarget::Broadleaf, ProductTarget::Iron];
    let req = UpdateProductRequest {
        name: "Speedzone".into(),
        brand: Some("  ".into()),
        stock_status: StockStatus::Out,
        facts,
        notes: Some(" ".into()),
        archived: true,
    };
    let Json(saved) = update_product(State(state), Path(id), Json(req))
        .await
        .unwrap();

    assert_eq!(saved.name, "Speedzone");
    assert_eq!((saved.brand, saved.notes), (None, None), "blank → absent");
    assert_eq!(saved.stock_status, StockStatus::Out);
    assert!(saved.archived);
    assert_eq!(saved.facts.category, ProductCategory::Herbicide);
    assert_eq!(
        saved.facts.herbicide_timing,
        Some(HerbicideTiming::PostEmergent)
    );
    assert!(saved.facts.frac_classes.is_empty(), "scoped to Herbicide");
    assert_eq!(saved.facts.targets, vec![ProductTarget::Broadleaf]);
    assert_eq!(saved.profile, before.profile);
    assert_eq!(saved.profile_model, before.profile_model);
    assert_eq!(saved.profile_generated_at, before.profile_generated_at);
}

// @spec INV-API-008, INV-SHELF-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_blank_name_or_half_a_label_rate_is_400_and_changes_nothing(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let p = add(&state, "Milorganite", ProductCategory::Fertilizer).await;
    let id = p.id.unwrap();

    let mut blank = edit_of(&p);
    blank.name = "  ".into();
    blank.stock_status = StockStatus::Out;
    let mut amount_only = edit_of(&p);
    amount_only.facts.label_rate_per_1000sqft = Some(2.0);
    let mut unit_only = edit_of(&p);
    unit_only.facts.label_rate_unit = Some(RateUnit::Lb);
    for req in [blank, amount_only, unit_only] {
        let err = update_product(State(state.clone()), Path(id), Json(req))
            .await
            .unwrap_err();
        assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
    }
    let Json(after) = get_product(State(state), Path(id)).await.unwrap();
    assert_eq!(after.name, "Milorganite");
    assert_eq!(after.stock_status, StockStatus::InStock);
    assert_eq!(after.facts, p.facts);
}

// @spec INV-SHELF-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_store_keeps_a_label_rate_only_as_a_positive_amount_with_its_unit(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let with_rate = |amount: Option<f64>, unit: Option<RateUnit>| {
        let mut f = ProductFacts::manual(ProductCategory::Fertilizer, ProductForm::Granular);
        f.label_rate_per_1000sqft = amount;
        f.label_rate_unit = unit;
        stored(lawn_id, &format!("{amount:?} {unit:?}"), f)
    };
    for (amount, unit) in [
        (Some(2.0), None),
        (None, Some(RateUnit::Oz)),
        (Some(0.0), Some(RateUnit::Lb)),
    ] {
        let refused = product_queries::create_product(&pool, &with_rate(amount, unit)).await;
        assert!(refused.is_err(), "{amount:?} {unit:?} must not be stored");
    }
    for (amount, unit) in [(Some(3.5), Some(RateUnit::FlOz)), (None, None)] {
        let id = product_queries::create_product(&pool, &with_rate(amount, unit))
            .await
            .unwrap();
        let p = product_queries::get_product(&pool, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (p.facts.label_rate_per_1000sqft, p.facts.label_rate_unit),
            (amount, unit)
        );
    }
}

// @spec INV-SHELF-001, INV-SHELF-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn every_vocabulary_value_round_trips_through_the_store_under_its_own_name(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    for (i, category) in ProductCategory::ALL.iter().enumerate() {
        let mut f = ProductFacts::manual(*category, ProductForm::ALL[i % ProductForm::ALL.len()]);
        f.label_rate_per_1000sqft = Some(1.5);
        f.label_rate_unit = Some(RateUnit::ALL[i % RateUnit::ALL.len()]);
        f.herbicide_timing = Some(HerbicideTiming::ALL[i % HerbicideTiming::ALL.len()]);
        f.amendment_kind = Some(AmendmentKind::ALL[i % AmendmentKind::ALL.len()]);
        f.targets = ProductTarget::ALL.to_vec();
        f.frac_classes = vec![FracClass::Frac11, FracClass::FracP07, FracClass::FracM3];
        let mut p = stored(lawn_id, category.as_str(), f.clone());
        p.stock_status = StockStatus::ALL[i % StockStatus::ALL.len()];
        let id = product_queries::create_product(&pool, &p).await.unwrap();

        let back = product_queries::get_product(&pool, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(back.facts, f);
        assert_eq!(back.stock_status, p.stock_status);
        assert_eq!(back.brand, p.brand);
        assert_eq!(back.notes, p.notes);
        assert_eq!(back.profile, p.profile);
        assert_eq!(back.profile_model, p.profile_model);
        assert!(back.profile_generated_at.is_some() && !back.archived);
        let (cat, stock): (String, String) =
            sqlx::query_as("SELECT category, stock_status FROM products WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            (cat.as_str(), stock.as_str()),
            (category.as_str(), p.stock_status.as_str())
        );
    }
}
