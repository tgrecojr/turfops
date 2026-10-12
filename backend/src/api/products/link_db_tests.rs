//! The log ↔ shelf bridge against PostgreSQL (`--features db-tests`).

use super::crud_db_tests::{add, archive};
use super::test_state::{lawn, state};
use super::*;
use crate::models::GrassType;
use sqlx::PgPool;

async fn log(pool: &PgPool, lawn: i64, kind: &str, name: &str, date: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO applications (lawn_profile_id, application_type, product_name, application_date)
         VALUES ($1, $2, $3, $4::date) RETURNING id",
    )
    .bind(lawn)
    .bind(kind)
    .bind(name)
    .bind(date)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn link_of(pool: &PgPool, application: i64) -> (Option<i64>, Option<String>) {
    sqlx::query_as("SELECT product_id, product_name FROM applications WHERE id = $1")
        .bind(application)
        .fetch_one(pool)
        .await
        .unwrap()
}

// @spec INV-LOG-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn suggestions_group_unlinked_uncatalogued_names_by_use(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let state = state(pool.clone(), None).await;
    let shelf = add(&state, " Milorganite ", ProductCategory::Fertilizer).await;
    assert_eq!(shelf.name, "Milorganite");
    let old = add(&state, "Old Seed", ProductCategory::Seed).await;
    archive(&state, &old).await;

    log(
        &pool,
        lawn_id,
        "Fertilizer",
        "scotts turf builder",
        "2026-04-01",
    )
    .await;
    log(
        &pool,
        lawn_id,
        "Fertilizer",
        "Scotts Turf Builder ",
        "2026-05-01",
    )
    .await;
    log(
        &pool,
        lawn_id,
        "Fungicide",
        "Scotts Turf Builder",
        "2026-03-01",
    )
    .await;
    log(&pool, lawn_id, "GrubControl", "GrubEx", "2026-06-01").await;
    log(&pool, lawn_id, "Fertilizer", "milorganite", "2026-04-02").await;
    log(&pool, lawn_id, "Overseed", "old seed", "2026-09-01").await;
    log(&pool, lawn_id, "Mowing", "  ", "2026-06-02").await;
    let linked = log(&pool, lawn_id, "Insecticide", "Linked Spray", "2026-06-03").await;
    let spray = add(&state, "Other name", ProductCategory::InsectControl).await;
    sqlx::query("UPDATE applications SET product_id = $1 WHERE id = $2")
        .bind(spray.id)
        .bind(linked)
        .execute(&pool)
        .await
        .unwrap();
    let other = queries::create_lawn_profile(
        &pool,
        &LawnProfile::new("Back".into(), GrassType::TallFescue, "7a".into()),
    )
    .await
    .unwrap();
    log(&pool, other, "Fertilizer", "Neighbour's bag", "2026-06-04").await;

    let Json(rows) = logged_product_suggestions(State(state)).await.unwrap();
    let summary: Vec<(&str, i64, String)> = rows
        .iter()
        .map(|r| (r.name.as_str(), r.uses, r.last_used.to_string()))
        .collect();
    assert_eq!(
        summary,
        [
            ("Scotts Turf Builder ", 3, "2026-05-01".to_string()),
            ("GrubEx", 1, "2026-06-01".to_string()),
        ]
    );
    let mut types = rows[0].application_types.clone();
    types.sort();
    assert_eq!(types, ["Fertilizer", "Fungicide"]);
}

// @spec INV-LOG-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn linking_takes_only_unlinked_rows_of_the_same_name(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let state = state(pool.clone(), None).await;
    let product = add(&state, "Heritage G", ProductCategory::Fungicide).await;
    let earlier = add(&state, "Heritage (old)", ProductCategory::Fungicide).await;
    let a = log(&pool, lawn_id, "Fungicide", " heritage g", "2026-06-01").await;
    let b = log(&pool, lawn_id, "Fungicide", "HERITAGE G", "2026-07-01").await;
    let taken = log(&pool, lawn_id, "Fungicide", "Heritage G", "2026-08-01").await;
    let other = log(&pool, lawn_id, "Fungicide", "Heritage", "2026-08-02").await;
    sqlx::query("UPDATE applications SET product_id = $1 WHERE id = $2")
        .bind(earlier.id)
        .bind(taken)
        .execute(&pool)
        .await
        .unwrap();

    let Json(res) = link_applications(State(state), Path(product.id.unwrap()))
        .await
        .unwrap();
    assert_eq!(res.linked, 2);
    assert_eq!(link_of(&pool, a).await.0, product.id);
    assert_eq!(link_of(&pool, b).await.0, product.id);
    assert_eq!(
        link_of(&pool, taken).await.0,
        earlier.id,
        "never reassigned"
    );
    assert_eq!(link_of(&pool, other).await.0, None);
}

// @spec INV-SHELF-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn deleting_a_product_keeps_its_applications_and_their_names(pool: PgPool) {
    let lawn_id = lawn(&pool).await;
    let state = state(pool.clone(), None).await;
    let product = add(&state, "Dimension", ProductCategory::Herbicide).await;
    let app = log(
        &pool,
        lawn_id,
        "PreEmergent",
        "Dimension 0.25G",
        "2026-03-20",
    )
    .await;
    sqlx::query("UPDATE applications SET product_id = $1 WHERE id = $2")
        .bind(product.id)
        .bind(app)
        .execute(&pool)
        .await
        .unwrap();

    delete_product(State(state), Path(product.id.unwrap()))
        .await
        .unwrap();
    assert_eq!(
        link_of(&pool, app).await,
        (None, Some("Dimension 0.25G".into()))
    );
}
