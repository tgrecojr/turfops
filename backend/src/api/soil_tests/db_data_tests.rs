//! What is stored and how the latest test and its advice are read (`--features db-tests`).

use super::db_tests::{add, advice, lawn, req};
use super::*;
use sqlx::PgPool;

// @spec SOIL-DATA-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn deleting_the_profile_deletes_its_soil_tests(pool: PgPool) {
    let (state, profile_id) = lawn(pool.clone()).await;
    add(&state, req("2026-03-23", 6.5)).await;
    add(&state, req("2025-03-23", 6.1)).await;
    sqlx::query("DELETE FROM lawn_profiles WHERE id = $1")
        .bind(profile_id)
        .execute(&pool)
        .await
        .unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM soil_tests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

// @spec SOIL-DATA-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_latest_test_is_the_newest_sample_not_the_newest_entry(pool: PgPool) {
    let (state, profile_id) = lawn(pool.clone()).await;
    let newest_sample = add(&state, req("2026-03-23", 6.5)).await;
    add(&state, req("2024-09-01", 5.8)).await; // entered later, sampled earlier
    let latest = soil_test_queries::get_latest_soil_test(&pool, profile_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.id, newest_sample.id);
    assert_eq!(advice(&state).await.unwrap().soil_test.id, newest_sample.id);
}

// @spec SOIL-DATA-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn advice_is_derived_afresh_on_every_read(pool: PgPool) {
    let (state, _) = lawn(pool.clone()).await;
    add(&state, req("2023-04-01", 6.5)).await;
    let id = add(&state, req("2026-03-23", 6.5)).await.id.unwrap();
    assert!(advice(&state).await.unwrap().ph_recommendation.is_none());

    let _ = update_soil_test(State(state.clone()), Path(id), Json(req("2026-03-23", 5.5)))
        .await
        .unwrap();
    let edited = advice(&state).await.unwrap();
    assert_eq!(
        edited.ph_recommendation.unwrap().current_ph,
        5.5,
        "the edit shows at once"
    );

    delete_soil_test(State(state.clone()), Path(id))
        .await
        .unwrap();
    let after_delete = advice(&state).await.unwrap();
    assert_eq!(after_delete.soil_test.test_date.to_string(), "2023-04-01");

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name::TEXT FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name LIKE '%soil%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(tables, ["soil_tests"], "no table holds derived advice");
}
