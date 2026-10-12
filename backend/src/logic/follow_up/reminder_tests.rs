//! What a follow-up reminder says, when it appears and when it goes away.

use super::generate_follow_up_recommendations as reminders;
use crate::models::plant::{IdentificationConfidence, Plant, PlantMaintenancePlan, PlantType};
use crate::models::{Application, ApplicationType, RecommendationCategory, Severity};
use chrono::{Duration, NaiveDate, Utc};
use std::collections::HashMap;

fn d(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

fn app(id: Option<i64>, plant_id: Option<i64>, follow_up: Option<NaiveDate>) -> Application {
    Application {
        id,
        lawn_profile_id: 1,
        application_type: ApplicationType::Insecticide,
        product_name: Some("Acelepryn".into()),
        application_date: d(5, 1),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id,
        follow_up_date: follow_up,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

fn viburnum() -> Plant {
    let now = Utc::now();
    Plant {
        id: Some(42),
        lawn_profile_id: 1,
        common_name: "Viburnum".into(),
        scientific_name: None,
        plant_type: PlantType::Shrub,
        location: None,
        planting_date: None,
        notes: None,
        maintenance_plan: PlantMaintenancePlan {
            identified_name: "Viburnum".into(),
            scientific_name: None,
            identification_confidence: IdentificationConfidence::High,
            summary: String::new(),
            tasks: Vec::new(),
            warnings: Vec::new(),
        },
        plan_generated_at: now,
        plan_model: "test".into(),
        created_at: now,
        updated_at: now,
    }
}

// @spec APP-FOLLOW-001
#[test]
fn one_reminder_per_scheduled_stored_entry_and_the_same_answer_every_time() {
    let log = [
        app(Some(1), None, Some(d(5, 15))),
        app(None, None, Some(d(5, 15))),
        app(Some(3), None, None),
    ];
    let plant = viburnum();
    let plants = HashMap::from([(42, &plant)]);
    let first = reminders(&log, &plants, d(5, 15));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].id, "application_followup_1");
    let again = reminders(&log, &plants, d(5, 15));
    assert_eq!(
        (again[0].title.as_str(), again[0].description.as_str()),
        (first[0].title.as_str(), first[0].description.as_str())
    );
}

// @spec APP-FOLLOW-002
#[test]
fn a_reminder_opens_7_days_ahead_and_lapses_90_days_after() {
    let follow_up = d(6, 1);
    let log = [app(Some(1), None, Some(follow_up))];
    let shown = |today: NaiveDate| !reminders(&log, &HashMap::new(), today).is_empty();
    assert!(!shown(follow_up - Duration::days(8)));
    assert!(shown(follow_up - Duration::days(7)));
    assert!(shown(follow_up + Duration::days(90)));
    assert!(!shown(follow_up + Duration::days(91)));
}

// @spec APP-FOLLOW-004
#[test]
fn advisory_before_the_date_and_warning_from_it() {
    let log = [app(Some(1), None, Some(d(6, 1)))];
    let at = |today| reminders(&log, &HashMap::new(), today).remove(0);

    let early = at(d(5, 29));
    assert_eq!(early.severity, Severity::Advisory);
    assert!(
        early.description.contains("scheduled in 3 day(s)"),
        "{}",
        early.description
    );
    let due = at(d(6, 1));
    assert_eq!(due.severity, Severity::Warning);
    assert!(
        due.description.contains("scheduled for today"),
        "{}",
        due.description
    );
    let late = at(d(6, 5));
    assert_eq!(late.severity, Severity::Warning);
    assert!(
        late.description.contains("overdue by 4 day(s)"),
        "{}",
        late.description
    );
}

// @spec APP-FOLLOW-005
#[test]
fn a_reminder_is_named_for_the_entry_its_target_and_product() {
    let plant = viburnum();
    let plants = HashMap::from([(42, &plant)]);
    let mut unnamed = app(Some(8), None, Some(d(5, 15)));
    unnamed.product_name = None;
    let log = [app(Some(7), Some(42), Some(d(5, 15))), unnamed];
    let recs = reminders(&log, &plants, d(5, 15));

    assert_eq!(recs[0].id, "application_followup_7");
    assert_eq!(recs[0].category, RecommendationCategory::ApplicationTiming);
    assert_eq!(
        recs[0].title,
        "Follow-up: Insecticide on Viburnum (Acelepryn)"
    );
    assert_eq!(recs[1].id, "application_followup_8");
    assert_eq!(
        recs[1].title,
        "Follow-up: Insecticide on Lawn (application)"
    );
}

// @spec APP-FOLLOW-006
#[test]
fn a_reminder_shows_its_dates_and_says_how_to_clear_it() {
    let mut unnamed = app(Some(2), None, Some(d(5, 15)));
    unnamed.product_name = None;
    let log = [app(Some(1), None, Some(d(5, 15))), unnamed];
    let recs = reminders(&log, &HashMap::new(), d(5, 15));

    let points: Vec<(&str, &str, &str)> = recs[0]
        .data_points
        .iter()
        .map(|p| (p.label.as_str(), p.value.as_str(), p.source.as_str()))
        .collect();
    let manual = crate::models::DataSource::Manual.as_str();
    assert_eq!(
        points,
        vec![
            ("Target", "Lawn", manual),
            ("Original", "May 1, 2026", manual),
            ("Follow-up", "May 15, 2026", manual),
            ("Product", "Acelepryn", manual),
        ]
    );
    assert_eq!(
        recs[0].suggested_action.as_deref(),
        Some("Re-apply Insecticide to Lawn and log it under Applications so this reminder clears.")
    );
    let labels: Vec<&str> = recs[1]
        .data_points
        .iter()
        .map(|p| p.label.as_str())
        .collect();
    assert_eq!(
        labels,
        vec!["Target", "Original", "Follow-up"],
        "no product named"
    );
}

// @spec APP-FOLLOW-007
#[test]
fn a_reminder_needs_nothing_from_the_shelf_and_states_no_amount() {
    let log = [app(Some(1), None, Some(d(5, 15)))];
    let rec = reminders(&log, &HashMap::new(), d(5, 15)).remove(0);
    assert!(rec.needs.is_empty());
    assert!(rec.feeding.is_none());
    let mut text = vec![
        rec.title.clone(),
        rec.description.clone(),
        rec.explanation.clone(),
        rec.suggested_action.clone().unwrap_or_default(),
    ];
    text.extend(
        rec.data_points
            .iter()
            .map(|p| format!("{} {}", p.label, p.value)),
    );
    for t in text {
        let lower = t.to_lowercase();
        for amount in [" lb", "rate", "per 1,000", "/1000", " oz", "sq ft"] {
            assert!(!lower.contains(amount), "{amount} in {t:?}");
        }
    }
}
