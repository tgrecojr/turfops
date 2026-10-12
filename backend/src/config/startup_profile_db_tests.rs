//! Seeding the lawn profile from `LAWN_*` at startup, against PostgreSQL.

use super::load::from_vars;
use super::Config;
use crate::db::queries;
use crate::models::{GrassType, IrrigationType, SoilType};
use sqlx::PgPool;
use std::collections::HashMap;

fn config(lawn: &[(&'static str, &str)]) -> Config {
    let mut vars = HashMap::from([("DATABASE_PASSWORD", "x".to_string())]);
    for (key, value) in lawn {
        vars.insert(*key, value.to_string());
    }
    from_vars(&|key| vars.get(key).cloned()).unwrap().0
}

// @spec APP-PROF-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn startup_seeds_the_defaults_once_and_then_ignores_lawn_settings(pool: PgPool) {
    crate::ensure_default_profile(&pool, &config(&[]))
        .await
        .unwrap();
    let seeded = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(seeded.name, "Main Lawn");
    assert_eq!(seeded.grass_type, GrassType::TallFescue);
    assert_eq!(seeded.usda_zone, "7a");

    let changed = config(&[("LAWN_NAME", "Back Forty"), ("LAWN_GRASS_TYPE", "kbg")]);
    crate::ensure_default_profile(&pool, &changed)
        .await
        .unwrap();
    let after = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((after.id, after.name.as_str()), (seeded.id, "Main Lawn"));
    assert_eq!(after.grass_type, GrassType::TallFescue);
}

// @spec APP-PROF-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn startup_seeds_from_the_lawn_settings_when_no_profile_exists(pool: PgPool) {
    let lawn = config(&[
        ("LAWN_NAME", "Back Forty"),
        ("LAWN_GRASS_TYPE", "kbg"),
        ("LAWN_USDA_ZONE", "6b"),
        ("LAWN_SOIL_TYPE", "Clay"),
        ("LAWN_SIZE_SQFT", "4200"),
        ("LAWN_IRRIGATION_TYPE", "Hose"),
    ]);
    crate::ensure_default_profile(&pool, &lawn).await.unwrap();
    let seeded = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(seeded.name, "Back Forty");
    assert_eq!(seeded.grass_type, GrassType::KentuckyBluegrass);
    assert_eq!(seeded.usda_zone, "6b");
    assert_eq!(seeded.soil_type, Some(SoilType::Clay));
    assert_eq!(seeded.lawn_size_sqft, Some(4200.0));
    assert_eq!(seeded.irrigation_type, Some(IrrigationType::Hose));
}
