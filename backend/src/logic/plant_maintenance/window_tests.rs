//! Task ids, completing types, the next-year lead-in and plan activities.

use super::tests::{make_plant, make_task, pruning_app};
use super::*;
use crate::models::ApplicationType;
use crate::models::{EnvironmentalSummary, Severity};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn recs(plant: &Plant, apps: &[Application], today: NaiveDate) -> Vec<Recommendation> {
    generate_plant_maintenance_recommendations(
        std::slice::from_ref(plant),
        apps,
        &EnvironmentalSummary::default(),
        crate::clock::Moment {
            now: chrono::Utc::now(),
            today,
        },
    )
}

fn logged(kind: ApplicationType, plant_id: i64, on: NaiveDate) -> Application {
    Application {
        application_type: kind,
        ..pruning_app(plant_id, on)
    }
}

// @spec PLANT-REC-002
#[test]
fn a_task_id_names_its_type_and_window_start_not_its_position() {
    let inspect = make_task(
        TaskType::PestInspection,
        "03-15",
        "04-15",
        Severity::Advisory,
    );
    let prune = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
    let today = d(2026, 3, 20);
    let one = make_plant(12, vec![prune.clone(), inspect.clone()]);
    let reordered = make_plant(12, vec![inspect, prune]);
    let ids = |p: &Plant| {
        let mut ids: Vec<String> = recs(p, &[], today).into_iter().map(|r| r.id).collect();
        ids.sort();
        ids
    };
    assert_eq!(
        ids(&one),
        vec![
            "plant_12_pest_inspection_2026-03-15",
            "plant_12_pruning_2026-03-01"
        ]
    );
    assert_eq!(
        ids(&one),
        ids(&reordered),
        "regenerating a plan must not move answers"
    );
}

// @spec PLANT-REC-002
#[test]
fn a_window_that_wraps_the_new_year_keeps_its_id_in_january() {
    let plant = make_plant(
        12,
        vec![make_task(
            TaskType::Pruning,
            "12-01",
            "02-28",
            Severity::Info,
        )],
    );
    let december = recs(&plant, &[], d(2026, 12, 10));
    let january = recs(&plant, &[], d(2027, 1, 10));
    assert_eq!(december[0].id, "plant_12_pruning_2026-12-01");
    assert_eq!(january[0].id, december[0].id);
}

// @spec PLANT-REC-005
#[test]
fn the_action_names_the_completing_type_or_says_to_mark_addressed() {
    let action = |task: TaskType| {
        let plant = make_plant(
            12,
            vec![make_task(task, "03-01", "03-31", Severity::Advisory)],
        );
        recs(&plant, &[], d(2026, 3, 10))[0]
            .suggested_action
            .clone()
            .unwrap()
    };
    let feed = action(TaskType::Fertilizing);
    assert!(
        feed.contains("Applications") && feed.contains("Plant Fertilizer"),
        "{feed}"
    );
    assert!(action(TaskType::PestInspection).contains("Other"));
    let water = action(TaskType::Watering);
    assert!(water.contains("Mark Addressed"), "{water}");
    assert!(
        !water.contains("log it"),
        "logging does not clear a watering reminder: {water}"
    );
}

// @spec PLANT-WIN-006
#[test]
fn only_the_matching_application_type_completes_a_task() {
    let inspect = make_plant(
        12,
        vec![make_task(
            TaskType::PestInspection,
            "03-01",
            "03-31",
            Severity::Info,
        )],
    );
    let other = logged(ApplicationType::Other, 12, d(2026, 3, 5));
    assert!(
        recs(&inspect, std::slice::from_ref(&other), d(2026, 3, 10)).is_empty(),
        "Other completes a pest inspection"
    );
    let water = make_plant(
        12,
        vec![make_task(
            TaskType::Watering,
            "03-01",
            "03-31",
            Severity::Info,
        )],
    );
    assert_eq!(
        recs(&water, &[other], d(2026, 3, 10)).len(),
        1,
        "nothing completes watering"
    );
    assert_eq!(
        TaskType::Other.matching_application_types(),
        &[ApplicationType::Other]
    );
    assert!(TaskType::Watering.matching_application_types().is_empty());
}

// @spec PLANT-REC-008
#[test]
fn a_january_window_gets_its_lead_in_in_late_december() {
    let plant = make_plant(
        12,
        vec![make_task(
            TaskType::Pruning,
            "01-03",
            "01-31",
            Severity::Info,
        )],
    );
    let late_december = recs(&plant, &[], d(2026, 12, 29));
    assert_eq!(late_december.len(), 1);
    assert_eq!(late_december[0].id, "plant_12_pruning_2027-01-03");
    assert!(recs(&plant, &[], d(2026, 12, 20)).is_empty(), "too early");
}

// @spec PLANT-PLAN-003
#[test]
fn a_plan_activity_carries_its_reminders_id_and_the_task_label() {
    let mut task = make_task(TaskType::PestInspection, "12-01", "02-28", Severity::Info);
    task.zone_note = Some("Zone 7a".into());
    let plant = make_plant(12, vec![task]);
    let today = d(2027, 1, 10);
    let activity = &build_plant_activities(std::slice::from_ref(&plant), &[], 2027, today)[0];
    assert_eq!(activity.id, recs(&plant, &[], today)[0].id);
    assert_eq!(activity.id, "plant_12_pest_inspection_2026-12-01");
    assert_eq!(activity.name, "Test Hydrangea — Pest Inspection");
    assert_eq!(activity.category, "Plant Maintenance");
    assert_eq!(activity.details.notes.as_deref(), Some("Zone 7a"));
    assert_eq!(activity.details.rate, None);
    assert_eq!(activity.details.soil_temp_trigger, None);
    assert_eq!(activity.date_window.earliest_historical, None);
}

// @spec PLANT-PLAN-005
#[test]
fn the_plan_reads_the_log_through_the_end_of_its_latest_window() {
    let wraps = make_plant(
        12,
        vec![
            make_task(TaskType::Pruning, "12-01", "02-28", Severity::Info),
            make_task(TaskType::Mulching, "04-01", "05-15", Severity::Info),
        ],
    );
    assert_eq!(
        plant_log_range(std::slice::from_ref(&wraps), 2026),
        Some((d(2025, 10, 1), d(2027, 3, 1))),
        "through Feb 28, 2027, end exclusive"
    );
    let spring = make_plant(
        13,
        vec![make_task(
            TaskType::Mulching,
            "04-01",
            "05-15",
            Severity::Info,
        )],
    );
    assert_eq!(
        plant_log_range(std::slice::from_ref(&spring), 2026),
        Some((d(2025, 10, 1), d(2027, 1, 1))),
        "never shorter than the plan year"
    );
}
