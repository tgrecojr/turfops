//! Startup against a real PostgreSQL (`--features db-tests`).

use crate::config::test_config;
use crate::db::pool::create_pool;
use crate::db::queries;
use crate::error::TurfOpsError;
use crate::logic::data_sync::DataSyncService;
use crate::models::{
    Application, ApplicationType, GrassType, IrrigationType, LawnProfile, SoilType,
};
use crate::state::AppState;
use axum::response::IntoResponse;
use sqlx::postgres::PgConnectOptions;
use sqlx::PgPool;

/// The migration versions shipped in `src/db/migrations`, ascending.
fn shipped_versions() -> Vec<i64> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/db/migrations");
    let mut versions: Vec<i64> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".sql"))
        .map(|name| name.split('_').next().unwrap().parse().unwrap())
        .collect();
    versions.sort_unstable();
    versions
}

async fn applied_versions(pool: &PgPool) -> Vec<i64> {
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// A new, empty database beside the test's own, for startup to migrate from scratch.
async fn empty_database(pool: &PgPool, suffix: &str) -> (String, PgConnectOptions) {
    let current: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(pool)
        .await
        .unwrap();
    // The test database's own name can already be the 63-byte identifier limit.
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        current.hash(&mut h);
        h.finish()
    };
    let name = format!("turfops_{suffix}_{hash:x}");
    for sql in [
        format!(r#"DROP DATABASE IF EXISTS "{name}""#),
        format!(r#"CREATE DATABASE "{name}""#),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .execute(pool)
            .await
            .unwrap();
    }
    let options = pool.connect_options().as_ref().clone().database(&name);
    (name, options)
}

async fn drop_database(pool: &PgPool, name: &str) {
    let sql = format!(r#"DROP DATABASE IF EXISTS "{name}" WITH (FORCE)"#);
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(pool)
        .await
        .unwrap();
}

// @spec DEPLOY-CI-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn each_database_test_starts_fresh_with_every_migration_applied(pool: PgPool) {
    assert_eq!(applied_versions(&pool).await, shipped_versions());
    let profiles: i64 = sqlx::query_scalar("SELECT count(*) FROM lawn_profiles")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(profiles, 0, "nothing left from another test");
}

// @spec DEPLOY-START-002, DEPLOY-CONFIG-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn startup_migrates_an_empty_database_to_the_shipped_schema(pool: PgPool) {
    let (name, options) = empty_database(&pool, "startup").await;
    let app = create_pool(options.clone(), 3).await.unwrap();
    assert_eq!(app.options().get_max_connections(), 3);
    assert_eq!(applied_versions(&app).await, shipped_versions());
    app.close().await;

    let again = create_pool(options, 3)
        .await
        .expect("a restart migrates nothing");
    assert_eq!(applied_versions(&again).await, shipped_versions());
    again.close().await;
    drop_database(&pool, &name).await;
}

// @spec DEPLOY-START-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failing_migration_stops_startup(pool: PgPool) {
    let (name, options) = empty_database(&pool, "badmigration").await;
    let app = create_pool(options.clone(), 2).await.unwrap();
    // The database now disagrees with the migration the binary ships.
    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = '\\x00'::bytea \
         WHERE version = (SELECT min(version) FROM _sqlx_migrations)",
    )
    .execute(&app)
    .await
    .unwrap();
    app.close().await;

    match create_pool(options, 2).await {
        Err(TurfOpsError::Config(msg)) => assert!(msg.starts_with("Migration failed"), "{msg}"),
        other => panic!("expected a startup error, got ok={}", other.is_ok()),
    }
    drop_database(&pool, &name).await;
}

// @spec DEPLOY-CONFIG-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_lawn_settings_seed_the_first_profile_only(pool: PgPool) {
    let config = test_config(&[
        ("LAWN_NAME", "Back Forty"),
        ("LAWN_GRASS_TYPE", "KentuckyBluegrass"),
        ("LAWN_USDA_ZONE", "6b"),
        ("LAWN_SOIL_TYPE", "Clay"),
        ("LAWN_SIZE_SQFT", "3200"),
        ("LAWN_IRRIGATION_TYPE", "Hose"),
    ]);
    crate::ensure_default_profile(&pool, &config).await.unwrap();
    let profile = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(profile.name, "Back Forty");
    assert_eq!(profile.grass_type, GrassType::KentuckyBluegrass);
    assert_eq!(profile.usda_zone, "6b");
    assert_eq!(profile.soil_type, Some(SoilType::Clay));
    assert_eq!(profile.lawn_size_sqft, Some(3200.0));
    assert_eq!(profile.irrigation_type, Some(IrrigationType::Hose));

    let later = test_config(&[("LAWN_NAME", "Front Yard"), ("LAWN_GRASS_TYPE", "Zoysia")]);
    crate::ensure_default_profile(&pool, &later).await.unwrap();
    let profiles: Vec<String> = sqlx::query_scalar("SELECT name FROM lawn_profiles")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(profiles, ["Back Forty"], "an existing profile is kept");
}

// @spec DEPLOY-HEALTH-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn health_is_ok_when_the_database_answers(pool: PgPool) {
    let config = test_config(&[("DATALAKE_ROOT", "/nonexistent/turfops-health")]);
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    let state = AppState::new(pool, sync, None, false, config.clock);
    let health = crate::api::health::health_check(axum::extract::State(state))
        .await
        .0;
    assert!(health.database);
    assert_eq!(health.status, "ok");
}

// @spec DEPLOY-HTTP-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_duplicate_application_is_a_409_saying_how_to_fix_it(pool: PgPool) {
    let lawn = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    let lawn_id = queries::create_lawn_profile(&pool, &lawn).await.unwrap();
    let entry = Application {
        id: None,
        lawn_profile_id: lawn_id,
        application_type: ApplicationType::Fertilizer,
        product_name: Some("Milorganite".into()),
        application_date: chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        plant_name: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: chrono::Utc::now(),
    };
    queries::create_application(&pool, &entry).await.unwrap();
    let err = queries::create_application(&pool, &entry)
        .await
        .expect_err("the duplicate guard");

    let response = err.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body["error"],
        "An identical entry (same type, product and date) is already logged. Edit the \
         existing entry, or change the product name if this was a different product."
    );
}
