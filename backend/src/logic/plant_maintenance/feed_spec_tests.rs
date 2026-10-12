//! What a task recommendation says.

use super::tests::{make_plant, make_task};
use super::*;
use crate::logic::rules::nitrogen_guard;
use crate::models::{GrassType, LawnProfile, Severity};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn feed(plant: &Plant, env: &EnvironmentalSummary, today: NaiveDate) -> Vec<Recommendation> {
    generate_plant_maintenance_recommendations(
        std::slice::from_ref(plant),
        &[],
        env,
        crate::clock::Moment {
            now: chrono::Utc::now(),
            today,
        },
    )
}

fn point(label: &str, rec: &Recommendation) -> Option<String> {
    rec.data_points
        .iter()
        .find(|p| p.label == label)
        .map(|p| p.value.clone())
}

// @spec PLANT-REC-001
#[test]
fn a_task_is_recommended_from_seven_days_before_its_window_through_its_last_day() {
    let plant = make_plant(
        7,
        vec![make_task(
            TaskType::Pruning,
            "04-10",
            "04-30",
            Severity::Advisory,
        )],
    );
    let env = EnvironmentalSummary::default();
    assert!(
        feed(&plant, &env, d(2026, 4, 2)).is_empty(),
        "8 days before"
    );
    assert_eq!(feed(&plant, &env, d(2026, 4, 3)).len(), 1, "7 days before");
    assert_eq!(feed(&plant, &env, d(2026, 4, 20)).len(), 1, "open");
    assert_eq!(feed(&plant, &env, d(2026, 4, 30)).len(), 1, "last day");
    assert!(feed(&plant, &env, d(2026, 5, 1)).is_empty(), "after it");
}

// @spec PLANT-REC-003, PLANT-REC-004
#[test]
fn a_task_recommendation_names_the_plant_task_and_window() {
    let mut task = make_task(
        TaskType::PestInspection,
        "04-10",
        "04-30",
        Severity::Warning,
    );
    task.description = "Check leaf undersides for scale.".into();
    task.zone_note = Some("Later in zone 6".into());
    let plant = make_plant(7, vec![task]);
    let rec = &feed(&plant, &EnvironmentalSummary::default(), d(2026, 4, 12))[0];
    assert_eq!(rec.category, RecommendationCategory::PlantMaintenance);
    assert_eq!(rec.severity, Severity::Warning);
    assert_eq!(rec.title, "Test Hydrangea — Pest Inspection (Shrub)");
    assert_eq!(rec.description, "Check leaf undersides for scale.");
    assert!(
        rec.explanation.contains("Apr 10 through Apr 30"),
        "{}",
        rec.explanation
    );
    assert!(rec
        .explanation
        .contains("homeowner-level guideline tuned to your USDA zone"));
    assert!(rec.explanation.contains("microclimate"));
    assert_eq!(point("Plant", rec).as_deref(), Some("Test Hydrangea"));
    assert_eq!(point("Window", rec).as_deref(), Some("Apr 10 – Apr 30"));
    assert_eq!(
        point("Scientific Name", rec).as_deref(),
        Some("Hydrangea paniculata")
    );
    assert_eq!(point("Zone Note", rec).as_deref(), Some("Later in zone 6"));

    let mut bare = make_plant(
        8,
        vec![make_task(
            TaskType::Pruning,
            "04-10",
            "04-30",
            Severity::Info,
        )],
    );
    bare.scientific_name = None;
    let rec = &feed(&bare, &EnvironmentalSummary::default(), d(2026, 4, 12))[0];
    assert_eq!(point("Scientific Name", rec), None);
    assert_eq!(point("Zone Note", rec), None);
}

// @spec PLANT-REC-006
#[test]
fn a_feeding_task_needs_a_fertilizer_or_supplement_with_no_nitrogen_requirement() {
    let plant = make_plant(
        7,
        vec![
            make_task(TaskType::Fertilizing, "04-10", "04-30", Severity::Info),
            make_task(TaskType::Mulching, "04-10", "04-30", Severity::Info),
        ],
    );
    let recs = feed(&plant, &EnvironmentalSummary::default(), d(2026, 4, 12));
    let need = &recs[0].needs[..];
    assert_eq!(need.len(), 1);
    assert_eq!(need[0].label, "plant fertilizer");
    assert_eq!(
        need[0].categories,
        vec![ProductCategory::Fertilizer, ProductCategory::Supplement]
    );
    assert!(!need[0].requires_nitrogen && !need[0].optional);
    assert!(recs[1].needs.is_empty());
}

// @spec PLANT-REC-007
#[test]
fn a_plant_feeding_is_never_held_or_capped_by_the_nitrogen_guard() {
    let plant = make_plant(
        7,
        vec![make_task(
            TaskType::Fertilizing,
            "04-10",
            "04-30",
            Severity::Advisory,
        )],
    );
    let mut recs = feed(&plant, &EnvironmentalSummary::default(), d(2026, 4, 12));
    assert!(recs[0].feeding.is_none());
    let before = (recs[0].severity, recs[0].title.clone());
    recs.push(Recommendation::new(
        "fertilizer_block",
        RecommendationCategory::Fertilizer,
        Severity::Warning,
        "Avoid Fertilizer Application",
        "Saturated soil.",
    ));
    let profile = LawnProfile::new("Lawn".into(), GrassType::TallFescue, "7a".into());
    nitrogen_guard::apply(&mut recs, &profile, &[], d(2026, 4, 12));
    assert_eq!((recs[0].severity, recs[0].title.clone()), before);
    assert_eq!(point("N budget remaining", &recs[0]), None);
}
