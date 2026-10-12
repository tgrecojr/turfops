//! `GET /api/v1/seasonal-plan`: the landscape-plant overlay (`--features db-tests`).

use super::db_support::*;
use crate::db::{plant_queries, queries};
use crate::logic::plant_maintenance::build_plant_activities;
use crate::models::plant::{
    IdentificationConfidence, MaintenanceTask, Plant, PlantMaintenancePlan, PlantType,
    TaskFrequency, TaskType,
};
use crate::models::{ApplicationType, Severity};
use chrono::Utc;
use sqlx::PgPool;

fn holly(profile: i64) -> Plant {
    Plant {
        id: None,
        lawn_profile_id: profile,
        common_name: "Holly".into(),
        scientific_name: None,
        plant_type: PlantType::Shrub,
        location: None,
        planting_date: None,
        notes: None,
        maintenance_plan: PlantMaintenancePlan {
            identified_name: "Holly".into(),
            scientific_name: None,
            identification_confidence: IdentificationConfidence::High,
            summary: "Dormant pruning".into(),
            tasks: vec![MaintenanceTask {
                task_type: TaskType::Pruning,
                window_start_month_day: "12-01".into(),
                window_end_month_day: "02-28".into(),
                frequency: TaskFrequency::Once,
                description: "Prune while dormant".into(),
                severity: Severity::Info,
                zone_note: None,
            }],
            warnings: vec![],
        },
        plan_generated_at: Utc::now(),
        plan_model: "test-model".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// @spec PLAN-MERGE-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn plant_activities_are_appended_as_built_over_the_plant_log_range(pool: PgPool) {
    let year = this_year() - 1;
    let profile = create_profile(&pool, "Front").await;
    settled_history(&pool, STATION, year).await;
    let mut plant = holly(profile);
    plant.id = Some(plant_queries::create_plant(&pool, &plant).await.unwrap());
    // The window opens Dec 1 of the plan year and closes in February of the next one;
    // a January pruning completes it only if the log reaches past the plan year.
    let mut pruned = application(profile, ApplicationType::Pruning, d(year + 1, 1, 15));
    pruned.plant_id = plant.id;
    queries::create_application(&pool, &pruned).await.unwrap();

    let state = state(&pool, None);
    let plan = get_plan(&state, Some(year)).await.unwrap();
    let overlay: Vec<_> = plan
        .activities
        .iter()
        .filter(|a| a.category == "Plant Maintenance")
        .collect();
    let today = state.clock.today();
    let expected = build_plant_activities(&[plant.clone()], &[pruned], year, today);
    assert_eq!(overlay.len(), expected.len());
    for (got, want) in overlay.iter().zip(&expected) {
        assert_eq!(
            serde_json::to_value(got).unwrap(),
            serde_json::to_value(want).unwrap(),
            "appended unchanged"
        );
    }
    let pruning = overlay[0];
    assert_eq!(
        pruning.id,
        format!("plant_{}_pruning_{year}-12-01", plant.id.unwrap())
    );
    assert_eq!(format!("{:?}", pruning.status), "Completed");
}
