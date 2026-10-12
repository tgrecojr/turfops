//! The fungicide shelf the disease planner sees, against PostgreSQL.

use super::db_tests::{eastern, lawn, silver_fixture, state_with};
use super::*;
use crate::models::{FracClass, Product, ProductFacts, ProductForm};
use sqlx::PgPool;

fn fungicide(profile_id: i64, name: &str, stock: StockStatus, classes: &[FracClass]) -> Product {
    product(profile_id, name, stock, ProductCategory::Fungicide, classes)
}

fn product(
    profile_id: i64,
    name: &str,
    stock: StockStatus,
    category: ProductCategory,
    classes: &[FracClass],
) -> Product {
    let mut facts = ProductFacts::manual(category, ProductForm::Granular);
    facts.frac_classes = classes.to_vec();
    Product {
        id: None,
        lawn_profile_id: profile_id,
        name: name.into(),
        brand: None,
        stock_status: stock,
        facts,
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: None,
        archived: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// @spec DIS-MGMT-PICK-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_shelf_is_usable_fungicides_by_their_recorded_classes(pool: PgPool) {
    let clock = eastern();
    let silver = silver_fixture("shelf", "03761", clock.today());
    let state = state_with(&pool, &silver, clock).await;
    let id = lawn(&pool).await.id.unwrap();
    let add = |p: Product| {
        let pool = pool.clone();
        async move { product_queries::create_product(&pool, &p).await.unwrap() }
    };
    add(fungicide(
        id,
        "Heritage G",
        StockStatus::InStock,
        &[FracClass::Frac11],
    ))
    .await;
    add(fungicide(
        id,
        "Banner Maxx",
        StockStatus::Low,
        &[FracClass::Frac3],
    ))
    .await;
    add(fungicide(
        id,
        "Daconil",
        StockStatus::Out,
        &[FracClass::FracM5],
    ))
    .await;
    let old = add(fungicide(
        id,
        "Velista",
        StockStatus::InStock,
        &[FracClass::Frac7],
    ))
    .await;
    sqlx::query("UPDATE products SET archived = true WHERE id = $1")
        .bind(old)
        .execute(&pool)
        .await
        .unwrap();
    let not_fungicide = product(
        id,
        "Mislabeled",
        StockStatus::InStock,
        ProductCategory::Herbicide,
        &[FracClass::Frac12],
    );
    add(not_fungicide).await;

    let Json(response) = get_disease_risk(State(state)).await.unwrap();
    std::fs::remove_file(&silver).ok();
    let bp = response
        .diseases
        .iter()
        .find(|r| r.disease == Disease::BrownPatch)
        .unwrap();
    let on_hand = |class: FracClass| -> Vec<String> {
        bp.management
            .preventative
            .options
            .iter()
            .find(|o| o.frac_class == class)
            .unwrap()
            .on_hand
            .iter()
            .map(|p| p.name.clone())
            .collect()
    };
    assert_eq!(on_hand(FracClass::Frac11), vec!["Heritage G"]);
    assert_eq!(on_hand(FracClass::Frac3), vec!["Banner Maxx"]);
    assert!(
        on_hand(FracClass::FracM5).is_empty(),
        "Out is not on the shelf"
    );
    assert!(
        on_hand(FracClass::Frac7).is_empty(),
        "archived is not on the shelf"
    );
    assert!(
        on_hand(FracClass::Frac12).is_empty(),
        "only fungicides count"
    );
}
