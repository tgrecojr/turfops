//! Adding and refreshing products with the product assistant, against PostgreSQL and a
//! stand-in OpenRouter (`--features db-tests`).

use super::crud_db_tests::{add, request, status_of};
use super::test_state::{lawn, openrouter_stub, profile_json, state};
use super::*;
use crate::models::FracClass;
use sqlx::PgPool;

async fn add_with(
    pool: &PgPool,
    reply: String,
    req: CreateProductRequest,
) -> (ProductCreated, String) {
    let (url, sent) = openrouter_stub(reply).await;
    let state = state(pool.clone(), Some(url)).await;
    let (status, Json(created)) = create_product(State(state), Json(req)).await.unwrap();
    assert_eq!(status, StatusCode::CREATED);
    (created, sent.await.unwrap())
}

fn user_prompt(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    v["messages"][1]["content"].as_str().unwrap().to_string()
}

// @spec INV-LLM-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_assistant_is_asked_with_brand_name_lawn_and_category_hint(pool: PgPool) {
    lawn(&pool).await;
    let mut req = request("Heritage G", Some(ProductCategory::Fungicide));
    req.brand = Some("Syngenta".into());
    req.assist = None;
    let (_, body) = add_with(&pool, profile_json("Fungicide", None), req).await;
    let prompt = user_prompt(&body);
    assert!(
        prompt.contains("Product: Syngenta Heritage G\n"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Category (user-reported): Fungicide"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Lawn grass type: Kentucky Bluegrass"),
        "{prompt}"
    );
    assert!(prompt.contains("USDA hardiness zone: 6b"), "{prompt}");

    let mut req = request("Syngenta Daconil", None);
    req.brand = Some("syngenta".into());
    req.assist = Some(true);
    let (_, body) = add_with(&pool, profile_json("Fungicide", None), req).await;
    let prompt = user_prompt(&body);
    assert!(
        prompt.contains("Product: Syngenta Daconil\n"),
        "brand not repeated: {prompt}"
    );
    assert!(
        prompt.contains("unknown — you decide"),
        "no hint without a category"
    );
}

// @spec INV-LLM-002, INV-API-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn facts_are_copied_once_from_the_profile(pool: PgPool) {
    lawn(&pool).await;
    let req = CreateProductRequest {
        assist: None,
        ..request("Mystery Mix", None)
    };
    let (created, _) = add_with(&pool, profile_json("Other", Some("Acme")), req).await;
    let f = &created.product.facts;
    assert_eq!(
        (f.category, f.form),
        (ProductCategory::Other, ProductForm::Granular)
    );
    assert_eq!(
        (f.nitrogen_pct, f.phosphorus_pct, f.potassium_pct),
        (Some(0.0), Some(0.0), Some(0.0))
    );
    assert_eq!(
        f.frac_classes,
        vec![FracClass::Frac3],
        "nothing curated: the profile's"
    );
    assert_eq!(f.herbicide_timing, Some(HerbicideTiming::PostEmergent));
    assert_eq!(
        f.targets,
        vec![ProductTarget::Iron, ProductTarget::Broadleaf]
    );
    assert_eq!(f.amendment_kind, Some(AmendmentKind::Humic));
    assert_eq!(
        (f.label_rate_per_1000sqft, f.label_rate_unit),
        (Some(2.0), Some(RateUnit::Lb))
    );
    assert_eq!(
        created.product.brand.as_deref(),
        Some("Acme"),
        "the profile's manufacturer"
    );
    assert_eq!(created.product.stock_status, StockStatus::InStock);
    assert!(created.product.profile.is_some() && created.profile_error.is_none());
    assert_eq!(created.product.profile_model.as_deref(), Some("test/model"));

    let req = CreateProductRequest {
        assist: None,
        brand: Some("Syngenta".into()),
        ..request("Heritage G", None)
    };
    let (created, _) = add_with(&pool, profile_json("Fungicide", Some("Acme")), req).await;
    assert_eq!(
        created.product.facts.frac_classes,
        vec![FracClass::Frac11],
        "curated wins"
    );
    assert_eq!(
        created.product.brand.as_deref(),
        Some("Syngenta"),
        "the owner's brand"
    );
}

// @spec INV-LLM-003, INV-SHELF-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_owners_category_and_form_beat_the_assistants_and_scope_the_facts(pool: PgPool) {
    lawn(&pool).await;
    let mut req = request("Mystery Spray", Some(ProductCategory::Herbicide));
    req.assist = Some(true);
    req.form = Some(ProductForm::Liquid);
    let (created, _) = add_with(&pool, profile_json("Fungicide", None), req).await;
    let f = &created.product.facts;
    assert_eq!(
        (f.category, f.form),
        (ProductCategory::Herbicide, ProductForm::Liquid)
    );
    assert!(f.frac_classes.is_empty(), "no FRAC class on a herbicide");
    assert_eq!(f.targets, vec![ProductTarget::Broadleaf], "weeds only");
    assert_eq!(f.herbicide_timing, Some(HerbicideTiming::PostEmergent));
    assert_eq!(f.amendment_kind, None);
}

// @spec INV-LLM-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failed_assistant_with_a_category_saves_by_hand_and_says_so(pool: PgPool) {
    lawn(&pool).await;
    let url = crate::test_support::stub_http(500, r#"{"error":"upstream"}"#).await;
    let state = state(pool, Some(url)).await;
    let mut req = request("Heritage G", Some(ProductCategory::Fungicide));
    req.assist = Some(true);
    let (status, Json(created)) = create_product(State(state), Json(req)).await.unwrap();
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created.profile_error.as_deref(), Some(SAVED_BY_HAND));
    assert_eq!(created.product.facts.category, ProductCategory::Fungicide);
    assert!(created.product.profile.is_none());
}

// @spec INV-LLM-008, INV-SHELF-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_refresh_stores_the_profile_and_only_suggests_facts(pool: PgPool) {
    lawn(&pool).await;
    let (url, sent) = openrouter_stub(profile_json("Fungicide", Some("Syngenta"))).await;
    let state = state(pool, Some(url)).await;
    let before = add(&state, "Heritage G", ProductCategory::Fungicide).await;
    let id = before.id.unwrap();

    let Json(res) = refresh_product_profile(State(state.clone()), Path(id))
        .await
        .unwrap();
    assert!(user_prompt(&sent.await.unwrap()).contains("Category (user-reported): Fungicide"));
    assert_eq!(res.changed_fields, ["form", "label_rate", "npk"]);
    let s = &res.suggested_facts;
    assert_eq!(
        s.frac_classes,
        vec![FracClass::Frac11],
        "curated precedence"
    );
    assert!(
        s.targets.is_empty() && s.herbicide_timing.is_none(),
        "scoped to Fungicide"
    );
    assert_eq!(s.form, ProductForm::Granular);

    let Json(after) = get_product(State(state), Path(id)).await.unwrap();
    assert_eq!(after.facts, before.facts, "the owner's facts are untouched");
    assert_eq!(
        after.profile.unwrap().manufacturer.as_deref(),
        Some("Syngenta")
    );
    assert_eq!(after.profile_model.as_deref(), Some("test/model"));
    assert!(after.profile_generated_at.is_some());
    assert_eq!(res.product.facts, before.facts);
}

// @spec INV-LLM-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_refresh_needs_openrouter_and_a_known_product(pool: PgPool) {
    lawn(&pool).await;
    let without = state(pool.clone(), None).await;
    let p = add(&without, "Heritage G", ProductCategory::Fungicide).await;
    let err = refresh_product_profile(State(without), Path(p.id.unwrap()))
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::SERVICE_UNAVAILABLE);

    let (url, mut sent) = openrouter_stub(profile_json("Fungicide", None)).await;
    let with = state(pool, Some(url)).await;
    let err = refresh_product_profile(State(with), Path(9999))
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);
    assert!(sent.try_recv().is_err(), "the LLM was not called");
}
