//! The shelf-product link on an application, against PostgreSQL.

use super::db_support::{lawn, product, request, state};
use super::{create_application, update_application};
use crate::db::product_queries::{self, ProductUpdate};
use crate::db::queries;
use crate::models::product::{ProductCategory, ProductFacts, ProductForm, StockStatus};
use crate::models::Application;
use axum::extract::{Path, State};
use axum::Json;
use sqlx::PgPool;

async fn stored(pool: &PgPool, id: Option<i64>) -> Application {
    queries::get_application_by_id(pool, id.unwrap())
        .await
        .unwrap()
        .unwrap()
}

// @spec APP-LINK-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_linked_entry_keeps_its_own_name_or_copies_the_products(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let milo = product(&pool, lawn_id, "Milorganite", ProductCategory::Fertilizer).await;
    let state = state(&pool).await;
    let linked = |day: &str, name: Option<&str>| {
        let mut req = request("Fertilizer", day);
        req.product_id = Some(milo);
        req.product_name = name.map(String::from);
        req
    };

    for (day, name) in [("2026-05-01", None), ("2026-05-02", Some("   "))] {
        let (_, Json(app)) = create_application(State(state.clone()), Json(linked(day, name)))
            .await
            .unwrap();
        let row = stored(&pool, app.id).await;
        assert_eq!(row.product_name.as_deref(), Some("Milorganite"), "{name:?}");
        assert_eq!(row.product_id, Some(milo));
    }

    let (_, Json(app)) = create_application(
        State(state.clone()),
        Json(linked("2026-05-03", Some("Milo 6-4-0"))),
    )
    .await
    .unwrap();
    assert_eq!(
        stored(&pool, app.id).await.product_name.as_deref(),
        Some("Milo 6-4-0")
    );

    let _updated = update_application(
        State(state),
        Path(app.id.unwrap()),
        Json(linked("2026-05-03", None)),
    )
    .await
    .unwrap();
    assert_eq!(
        stored(&pool, app.id).await.product_name.as_deref(),
        Some("Milorganite"),
        "an update copies the name too"
    );
}

// @spec APP-LINK-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn renaming_or_deleting_a_product_never_rewrites_the_log(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let milo = product(&pool, lawn_id, "Milorganite", ProductCategory::Fertilizer).await;
    let state = state(&pool).await;
    let mut req = request("Fertilizer", "2026-05-01");
    req.product_id = Some(milo);
    let (_, Json(app)) = create_application(State(state), Json(req)).await.unwrap();

    let rename = ProductUpdate {
        name: "Milorganite 6-4-0".into(),
        brand: None,
        stock_status: StockStatus::InStock,
        facts: ProductFacts::manual(ProductCategory::Fertilizer, ProductForm::Granular),
        notes: None,
        archived: false,
    };
    product_queries::update_product(&pool, milo, &rename)
        .await
        .unwrap();
    assert_eq!(
        stored(&pool, app.id).await.product_name.as_deref(),
        Some("Milorganite")
    );

    product_queries::delete_product(&pool, milo).await.unwrap();
    let row = stored(&pool, app.id).await;
    assert_eq!(row.product_id, None, "the link is cleared");
    assert_eq!(
        row.product_name.as_deref(),
        Some("Milorganite"),
        "the name stays"
    );
}
