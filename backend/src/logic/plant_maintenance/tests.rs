use super::*;
use crate::models::plant::{
    IdentificationConfidence, MaintenanceTask, PlantMaintenancePlan, PlantType, TaskFrequency,
};
use crate::models::Severity;
use chrono::Utc;

fn make_task(
    task_type: TaskType,
    start_mmdd: &str,
    end_mmdd: &str,
    severity: Severity,
) -> MaintenanceTask {
    MaintenanceTask {
        task_type,
        window_start_month_day: start_mmdd.into(),
        window_end_month_day: end_mmdd.into(),
        frequency: TaskFrequency::Once,
        description: "Test task".into(),
        severity,
        zone_note: None,
    }
}

fn make_plant(id: i64, tasks: Vec<MaintenanceTask>) -> Plant {
    Plant {
        id: Some(id),
        lawn_profile_id: 1,
        common_name: "Test Hydrangea".into(),
        scientific_name: Some("Hydrangea paniculata".into()),
        plant_type: PlantType::Shrub,
        location: None,
        planting_date: None,
        notes: None,
        maintenance_plan: PlantMaintenancePlan {
            identified_name: "Test Hydrangea".into(),
            scientific_name: Some("Hydrangea paniculata".into()),
            identification_confidence: IdentificationConfidence::High,
            summary: "Test plan".into(),
            tasks,
            warnings: vec![],
        },
        plan_generated_at: Utc::now(),
        plan_model: "test-model".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn pruning_app(plant_id: i64, date: NaiveDate) -> Application {
    Application {
        id: Some(1),
        lawn_profile_id: 1,
        application_type: ApplicationType::Pruning,
        product_name: None,
        application_date: date,
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        weather_snapshot: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: Some(plant_id),
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

#[test]
fn parse_month_day_clamps_invalid_day() {
    assert_eq!(
        parse_month_day("02-30", 2026),
        Some(NaiveDate::from_ymd_opt(2026, 2, 28).unwrap())
    );
}

#[test]
fn resolve_window_handles_wrap() {
    let task = make_task(TaskType::WinterProtection, "11-15", "02-15", Severity::Info);
    let (start, end) = resolve_window(&task, 2026).unwrap();
    assert_eq!(start.year(), 2026);
    assert_eq!(end.year(), 2027);
}

#[test]
fn recommendation_emitted_in_window() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let recs = generate_plant_maintenance_recommendations(&[plant], &[], today);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].category, RecommendationCategory::PlantMaintenance);
    assert!(recs[0].title.contains("Test Hydrangea"));
}

#[test]
fn recommendation_suppressed_when_completed() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let apps = vec![pruning_app(
        42,
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
    )];
    let recs = generate_plant_maintenance_recommendations(&[plant], &apps, today);
    assert!(recs.is_empty(), "Should suppress after pruning was logged");
}

#[test]
fn recommendation_not_emitted_before_lead_in() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    // Well before lead-in window.
    let today = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
    let recs = generate_plant_maintenance_recommendations(&[plant], &[], today);
    assert!(recs.is_empty());
}

#[test]
fn recommendation_not_emitted_after_end() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let today = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
    let recs = generate_plant_maintenance_recommendations(&[plant], &[], today);
    assert!(recs.is_empty());
}

#[test]
fn build_plant_activities_status_active() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let acts = build_plant_activities(&[plant], &[], 2026, today);
    assert_eq!(acts.len(), 1);
    assert!(matches!(acts[0].status, ActivityStatus::Active));
    assert_eq!(acts[0].category, "Plant Maintenance");
}

#[test]
fn build_plant_activities_status_completed_with_app() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let today = NaiveDate::from_ymd_opt(2026, 4, 5).unwrap();
    let apps = vec![pruning_app(
        42,
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
    )];
    let acts = build_plant_activities(&[plant], &apps, 2026, today);
    assert!(matches!(acts[0].status, ActivityStatus::Completed));
}

#[test]
fn build_plant_activities_status_missed() {
    let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let today = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
    let acts = build_plant_activities(&[plant], &[], 2026, today);
    assert!(matches!(acts[0].status, ActivityStatus::Missed));
}

#[test]
fn a_window_that_wraps_the_new_year_stays_open_in_january() {
    let task = make_task(TaskType::Pruning, "12-01", "02-28", Severity::Advisory);
    let plant = make_plant(42, vec![task]);
    let jan = NaiveDate::from_ymd_opt(2027, 1, 10).unwrap();

    let recs = generate_plant_maintenance_recommendations(std::slice::from_ref(&plant), &[], jan);
    assert_eq!(
        recs.len(),
        1,
        "dormant pruning has seven weeks left on Jan 10"
    );
    // Same id as in December, so an answer given then still applies.
    assert!(recs[0].id.ends_with("_2026"), "{}", recs[0].id);

    let acts = build_plant_activities(std::slice::from_ref(&plant), &[], 2027, jan);
    assert!(matches!(acts[0].status, ActivityStatus::Active));

    // Pruned in December: done for this window, on both sides of the new year.
    let apps = vec![pruning_app(
        42,
        NaiveDate::from_ymd_opt(2026, 12, 12).unwrap(),
    )];
    assert!(generate_plant_maintenance_recommendations(&[plant], &apps, jan).is_empty());
}

#[test]
fn completion_belongs_to_one_window() {
    // Two pruning windows a year: the spring job must not complete the fall one.
    let spring = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let fall = make_task(TaskType::Pruning, "09-01", "09-30", Severity::Advisory);
    let plant = make_plant(42, vec![spring, fall]);
    let apps = vec![pruning_app(
        42,
        NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
    )];

    let sept = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
    let recs =
        generate_plant_maintenance_recommendations(std::slice::from_ref(&plant), &apps, sept);
    assert_eq!(recs.len(), 1, "the fall window is still to do");

    let acts = build_plant_activities(&[plant], &apps, 2026, sept);
    assert!(matches!(acts[0].status, ActivityStatus::Completed));
    assert!(matches!(acts[1].status, ActivityStatus::Active));
}
