//! Shared fixtures for the feed's tests: an app state whose weather lake is missing (an
//! outage), a lawn, log entries and a landscape plant.

use crate::clock::Clock;
use crate::config::load::from_vars;
use crate::logic::data_sync::DataSyncService;
use crate::models::plant::{
    IdentificationConfidence, MaintenanceTask, Plant, PlantMaintenancePlan, PlantType,
    TaskFrequency, TaskType,
};
use crate::models::{Application, ApplicationType, GrassType, LawnProfile, Severity};
use crate::state::AppState;
use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;

/// App state over `pool` with the weather lake pointed at a directory that does not
/// exist and no Home Assistant, forecast or OpenRouter: every lake read fails.
pub(crate) async fn state_without_lake(pool: PgPool, clock: Clock) -> AppState {
    let (config, _) = from_vars(&|key| match key {
        "DATABASE_PASSWORD" => Some("unused".into()),
        "DATALAKE_ROOT" => Some("/nonexistent/turfops-feed-tests".into()),
        _ => None,
    })
    .expect("test configuration");
    let sync = DataSyncService::initialize(&config, pool.clone()).await;
    AppState::new(pool, sync, None, false, clock)
}

/// A pool that never connects, for state whose test touches no table.
pub(crate) fn unconnected_pool() -> PgPool {
    PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/none").expect("lazy pool")
}

pub(crate) fn profile() -> LawnProfile {
    LawnProfile {
        id: Some(1),
        name: "Front".into(),
        grass_type: GrassType::TallFescue,
        usda_zone: "7a".into(),
        soil_type: None,
        lawn_size_sqft: Some(5000.0),
        irrigation_type: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub(crate) fn application(
    profile_id: i64,
    kind: ApplicationType,
    date: NaiveDate,
    follow_up: Option<NaiveDate>,
) -> Application {
    Application {
        id: None,
        lawn_profile_id: profile_id,
        application_type: kind,
        product_name: None,
        application_date: date,
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: follow_up,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

/// A shrub whose plan has one Advisory fertilizing task open on `today`.
pub(crate) fn plant_fertilizing_on(profile_id: i64, today: NaiveDate) -> Plant {
    let mmdd = |d: NaiveDate| d.format("%m-%d").to_string();
    let task = MaintenanceTask {
        task_type: TaskType::Fertilizing,
        window_start_month_day: mmdd(today - Duration::days(5)),
        window_end_month_day: mmdd(today + Duration::days(20)),
        frequency: TaskFrequency::Once,
        description: "Feed the hydrangea".into(),
        severity: Severity::Advisory,
        zone_note: None,
    };
    Plant {
        id: None,
        lawn_profile_id: profile_id,
        common_name: "Hydrangea".into(),
        scientific_name: None,
        plant_type: PlantType::Shrub,
        location: None,
        planting_date: None,
        notes: None,
        maintenance_plan: PlantMaintenancePlan {
            identified_name: "Hydrangea".into(),
            scientific_name: None,
            identification_confidence: IdentificationConfidence::High,
            summary: "Test plan".into(),
            tasks: vec![task],
            warnings: vec![],
        },
        plan_generated_at: Utc::now(),
        plan_model: "test-model".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}
