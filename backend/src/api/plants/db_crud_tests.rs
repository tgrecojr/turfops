//! Listing, reading, editing and deleting plants, against PostgreSQL.

use super::db_support::{profile, saved_plant, state, status};
use super::*;
use crate::logic::plant_maintenance::generate_plant_maintenance_recommendations;
use crate::models::plant::{MaintenanceTask, TaskFrequency, TaskType};
use crate::models::{Application, ApplicationType, EnvironmentalSummary, Severity};
use serde_json::json;
use sqlx::PgPool;

fn edit(body: serde_json::Value) -> Json<UpdatePlantRequest> {
    Json(serde_json::from_value(body).unwrap())
}

// @spec PLANT-API-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_list_is_the_lawns_plants_newest_first_or_404_without_a_lawn(pool: PgPool) {
    let nobody = state(pool.clone(), None).await;
    let err = list_plants(State(nobody)).await.unwrap_err();
    assert_eq!(status(err), StatusCode::NOT_FOUND);

    let lawn = profile(&pool, "7a").await;
    let other_lawn = profile(&pool, "7a").await;
    let older = saved_plant(&pool, lawn, "Boxwood").await;
    let newer = saved_plant(&pool, lawn, "Azalea").await;
    saved_plant(&pool, other_lawn, "Not mine").await;
    sqlx::query("UPDATE plants SET created_at = NOW() - INTERVAL '2 days' WHERE id = $1")
        .bind(older)
        .execute(&pool)
        .await
        .unwrap();

    let Json(plants) = list_plants(State(state(pool, None).await)).await.unwrap();
    let ids: Vec<_> = plants.iter().map(|p| p.id.unwrap()).collect();
    assert_eq!(ids, vec![newer, older]);
}

// @spec PLANT-API-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn reading_a_plant_returns_it_or_404(pool: PgPool) {
    let id = saved_plant(&pool, profile(&pool, "7a").await, "Boxwood").await;
    let state = state(pool, None).await;
    let Json(plant) = get_plant(State(state.clone()), Path(id)).await.unwrap();
    assert_eq!(plant.common_name, "Boxwood");
    let err = get_plant(State(state), Path(id + 1)).await.unwrap_err();
    assert_eq!(status(err), StatusCode::NOT_FOUND);
}

// @spec PLANT-API-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_edit_changes_only_the_given_fields_and_never_the_plan(pool: PgPool) {
    let id = saved_plant(&pool, profile(&pool, "7a").await, "Boxwood").await;
    let before = plant_queries::get_plant(&pool, id).await.unwrap().unwrap();
    let Json(after) = update_plant(
        State(state(pool, None).await),
        Path(id),
        edit(json!({
            "plant_type": "Tree",
            "planting_date": "2023-05-01",
            "notes": "moved in 2023"
        })),
    )
    .await
    .unwrap();
    assert_eq!(after.plant_type, PlantType::Tree);
    assert_eq!(after.planting_date, NaiveDate::from_ymd_opt(2023, 5, 1));
    assert_eq!(after.notes.as_deref(), Some("moved in 2023"));
    assert_eq!(
        after.scientific_name, before.scientific_name,
        "absent is kept"
    );
    assert_eq!(after.common_name, before.common_name, "absent is kept");
    assert_eq!(after.location, before.location, "absent is kept");
    assert!(after.updated_at > before.updated_at);
    assert_eq!(after.plan_model, before.plan_model);
    assert_eq!(after.plan_generated_at, before.plan_generated_at);
    assert_eq!(
        after.maintenance_plan.summary,
        before.maintenance_plan.summary
    );
}

// @spec PLANT-API-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_null_clears_the_optional_fields(pool: PgPool) {
    let id = saved_plant(&pool, profile(&pool, "7a").await, "Boxwood").await;
    let state = state(pool, None).await;
    let filled = json!({
        "scientific_name": "Buxus sempervirens",
        "location": "front walk",
        "planting_date": "2023-05-01",
        "notes": "moved in 2023"
    });
    let Json(set) = update_plant(State(state.clone()), Path(id), edit(filled))
        .await
        .unwrap();
    assert_eq!(set.location.as_deref(), Some("front walk"));

    let cleared = json!({
        "scientific_name": null,
        "location": null,
        "planting_date": null,
        "notes": null
    });
    let Json(after) = update_plant(State(state), Path(id), edit(cleared))
        .await
        .unwrap();
    assert_eq!(after.scientific_name, None);
    assert_eq!(after.location, None);
    assert_eq!(after.planting_date, None);
    assert_eq!(after.notes, None);
    assert_eq!(after.common_name, "Boxwood", "absent is still kept");
}

// @spec PLANT-API-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_edit_of_a_missing_plant_is_404_and_a_bad_field_is_400(pool: PgPool) {
    let id = saved_plant(&pool, profile(&pool, "7a").await, "Boxwood").await;
    let state = state(pool, None).await;
    let err = update_plant(
        State(state.clone()),
        Path(id + 1),
        edit(json!({ "notes": "x" })),
    )
    .await
    .unwrap_err();
    assert_eq!(status(err), StatusCode::NOT_FOUND);
    for bad in [
        json!({ "plant_type": "Bush" }),
        json!({ "planting_date": "05/01/2023" }),
    ] {
        let err = update_plant(State(state.clone()), Path(id), edit(bad.clone()))
            .await
            .unwrap_err();
        assert_eq!(status(err), StatusCode::BAD_REQUEST, "{bad}");
    }
}

fn pruning_task() -> MaintenanceTask {
    MaintenanceTask {
        task_type: TaskType::Pruning,
        window_start_month_day: "01-01".into(),
        window_end_month_day: "12-31".into(),
        frequency: TaskFrequency::Once,
        description: "Prune.".into(),
        severity: Severity::Advisory,
        zone_note: None,
    }
}

fn pruned(lawn: i64, plant: i64) -> Application {
    Application {
        id: None,
        lawn_profile_id: lawn,
        application_type: ApplicationType::Pruning,
        product_name: None,
        application_date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: Some(plant),
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

// @spec PLANT-API-011
#[sqlx::test(migrations = "src/db/migrations")]
async fn deleting_a_plant_ends_its_reminders_and_keeps_its_log_unlinked(pool: PgPool) {
    let lawn = profile(&pool, "7a").await;
    let id = saved_plant(&pool, lawn, "Boxwood").await;
    let mut plant = plant_queries::get_plant(&pool, id).await.unwrap().unwrap();
    plant.maintenance_plan.tasks = vec![pruning_task()];
    plant_queries::update_plant_plan(&pool, id, &plant.maintenance_plan, "m")
        .await
        .unwrap();
    queries::create_application(&pool, &pruned(lawn, id))
        .await
        .unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
    let reminders = |plants: Vec<Plant>| {
        generate_plant_maintenance_recommendations(
            &plants,
            &[],
            &EnvironmentalSummary::default(),
            today,
        )
    };
    let listed = plant_queries::list_plants_for_profile(&pool, lawn)
        .await
        .unwrap();
    assert_eq!(reminders(listed).len(), 1);

    let code = delete_plant(State(state(pool.clone(), None).await), Path(id))
        .await
        .unwrap();
    assert_eq!(code, StatusCode::NO_CONTENT);
    let listed = plant_queries::list_plants_for_profile(&pool, lawn)
        .await
        .unwrap();
    assert!(reminders(listed).is_empty());
    let log = queries::get_applications_for_profile(&pool, lawn, 100, 0)
        .await
        .unwrap();
    assert_eq!(log.len(), 1, "the entry stays in the log");
    assert_eq!(log[0].plant_id, None, "its plant link is cleared");
    assert_eq!(log[0].plant_name.as_deref(), Some("Boxwood"));
}
