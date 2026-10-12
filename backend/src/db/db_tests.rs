//! PostgreSQL-backed tests (`--features db-tests`). Each `#[sqlx::test]` runs against a fresh
//! database that `DATABASE_URL`'s server creates and migrates, then drops.

use super::queries;
use crate::models::{GrassType, LawnProfile};
use sqlx::PgPool;

fn profile(name: &str) -> LawnProfile {
    LawnProfile::new(name.to_string(), GrassType::TallFescue, "7a".to_string())
}

// @spec APP-PROF-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_lawn_profile_is_the_lowest_id_row(pool: PgPool) {
    let first = queries::create_lawn_profile(&pool, &profile("Front"))
        .await
        .unwrap();
    queries::create_lawn_profile(&pool, &profile("Back"))
        .await
        .unwrap();

    let resolved = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.id, Some(first));
    assert_eq!(resolved.name, "Front");
}
