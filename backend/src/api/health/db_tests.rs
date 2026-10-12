//! A database that answers makes the server healthy, whatever the external sources do.

use super::health_check;
use crate::logic::data_sync::test_env::*;
use axum::extract::State;
use sqlx::PgPool;

// @spec SYNC-CONN-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn health_is_ok_on_the_database_alone_even_with_every_source_down(pool: PgPool) {
    let ha = stub(500, "{}").await;
    let mut vars = with_home_assistant(&ha.url);
    vars.extend(with_forecast());
    let service = service(&config_of(&vars), Some(UNREACHABLE), pool.clone()).await;
    let state = app_state(pool, service, false);

    let axum::Json(health) = health_check(State(state)).await;
    assert_eq!(health.status, "ok");
    assert!(health.database);
    let sources = &health.datasources;
    assert!(!sources.soildata && !sources.homeassistant && !sources.openweathermap);
}
