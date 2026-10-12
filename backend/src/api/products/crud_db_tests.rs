//! The products CRUD handlers against PostgreSQL (`--features db-tests`).

use super::test_state::{lawn, state};
use super::*;
use crate::models::FracClass;
use crate::state::AppState;
use axum::response::IntoResponse;
use sqlx::PgPool;

pub(super) fn request(name: &str, category: Option<ProductCategory>) -> CreateProductRequest {
    CreateProductRequest {
        name: name.into(),
        brand: None,
        category,
        form: None,
        stock_status: None,
        notes: None,
        assist: Some(false),
    }
}

pub(super) async fn add(state: &AppState, name: &str, category: ProductCategory) -> Product {
    let (_, Json(created)) =
        create_product(State(state.clone()), Json(request(name, Some(category))))
            .await
            .unwrap();
    created.product
}

pub(super) fn edit_of(p: &Product) -> UpdateProductRequest {
    UpdateProductRequest {
        name: p.name.clone(),
        brand: p.brand.clone(),
        stock_status: p.stock_status,
        facts: p.facts.clone(),
        notes: p.notes.clone(),
        archived: p.archived,
    }
}

pub(super) async fn archive(state: &AppState, p: &Product) {
    let mut req = edit_of(p);
    req.archived = true;
    let _ = update_product(State(state.clone()), Path(p.id.unwrap()), Json(req))
        .await
        .unwrap();
}

pub(super) fn status_of(err: TurfOpsError) -> StatusCode {
    err.into_response().status()
}

async fn list(state: &AppState, category: Option<ProductCategory>, archived: bool) -> Vec<String> {
    let q = ListQuery {
        category,
        include_archived: archived,
    };
    let Json(list) = list_products(State(state.clone()), Query(q)).await.unwrap();
    list.into_iter().map(|p| p.name).collect()
}

// @spec INV-API-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_list_is_by_category_then_name_without_archived_unless_asked(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    add(&state, "banner", ProductCategory::Fungicide).await;
    add(&state, "zeta", ProductCategory::Fertilizer).await;
    add(&state, "Beta", ProductCategory::Fertilizer).await;
    add(&state, "alpha", ProductCategory::Fertilizer).await;
    let old = add(&state, "Old bag", ProductCategory::Fertilizer).await;
    archive(&state, &old).await;

    assert_eq!(
        list(&state, None, false).await,
        ["alpha", "Beta", "zeta", "banner"]
    );
    assert_eq!(
        list(&state, Some(ProductCategory::Fungicide), false).await,
        ["banner"]
    );
    assert_eq!(
        list(&state, None, true).await,
        ["alpha", "Beta", "Old bag", "zeta", "banner"]
    );
}

// @spec INV-API-002, INV-API-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_unknown_product_id_is_404_to_read_and_to_edit(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let shelf = add(&state, "Milorganite", ProductCategory::Fertilizer).await;

    let err = get_product(State(state.clone()), Path(9999))
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);
    let mut edit = edit_of(&shelf);
    edit.name = "A name nothing uses".into();
    let err = update_product(State(state.clone()), Path(9999), Json(edit))
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);
}

// @spec INV-API-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_blank_name_is_400_and_saves_nothing(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let err = create_product(
        State(state.clone()),
        Json(request("   ", Some(ProductCategory::Seed))),
    )
    .await
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::BAD_REQUEST);
    assert!(list(&state, None, true).await.is_empty());
}

// @spec INV-API-004, INV-API-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_product_saved_by_hand_takes_the_owners_facts_and_curated_frac(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let mut req = request("Heritage G", Some(ProductCategory::Fungicide));
    req.brand = Some("Syngenta".into());
    let (status, Json(created)) = create_product(State(state.clone()), Json(req))
        .await
        .unwrap();

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created.profile_error, None);
    let p = created.product;
    assert_eq!(p.facts.category, ProductCategory::Fungicide);
    assert_eq!(p.facts.form, ProductForm::Other, "no form given");
    assert_eq!(
        p.facts.frac_classes,
        vec![FracClass::Frac11],
        "curated table"
    );
    assert_eq!(p.brand.as_deref(), Some("Syngenta"));
    assert_eq!(p.stock_status, StockStatus::InStock);
    assert!(p.profile.is_none());

    // The assistant left on but not configured: still saved by hand when a category is given.
    let mut req = request("Daconil Weather Stik", Some(ProductCategory::Fungicide));
    req.assist = None;
    req.form = Some(ProductForm::Liquid);
    let (_, Json(created)) = create_product(State(state), Json(req)).await.unwrap();
    assert_eq!(created.product.facts.form, ProductForm::Liquid);
    assert_eq!(created.product.facts.frac_classes, vec![FracClass::FracM5]);
}

// @spec INV-API-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn delete_removes_the_product_and_answers_204(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let p = add(&state, "Dimension", ProductCategory::Herbicide).await;
    let status = delete_product(State(state.clone()), Path(p.id.unwrap()))
        .await
        .unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);
    let err = get_product(State(state), Path(p.id.unwrap()))
        .await
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);
}
