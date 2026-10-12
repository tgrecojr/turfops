//! The guard's budget point, target-reached and cap edits, and the hold.

use super::*;
use crate::logic::rules::test_fixtures::{app, date, profile};
use crate::models::{
    ApplicationType, GrassType, ProductCategory, ProductNeed, RecommendationCategory,
    RecommendationState,
};
use chrono::Utc;
use std::collections::HashMap;

fn today() -> NaiveDate {
    date(2026, 10, 5)
}

/// A lawn feeding of `lbs` N / 1000 sq ft (20% N).
fn fed(lbs: f64, on: NaiveDate) -> Application {
    app(ApplicationType::Fertilizer, on, Some(20.0), Some(lbs * 5.0))
}

/// A feeding as a rule builds one: `n` lb N/1000 sq ft on a 5000 sq ft lawn.
fn feeding(n: f64) -> Recommendation {
    Recommendation::new(
        "fall_fert_mid",
        RecommendationCategory::Fertilizer,
        Severity::Warning,
        "Main Fall Feeding",
        format!("Feed now: {}.", amount_text(n, Some(5000.0))),
    )
    .with_data_point(RATE_POINT, rate_text(n), DataSource::Agronomic.as_str())
    .with_need(
        ProductNeed::new("nitrogen fertilizer", ProductCategory::Fertilizer).requiring_nitrogen(),
    )
    .with_action(format!("Apply {}.", amount_text(n, Some(5000.0))))
    .with_feeding(n)
}

fn blocker(id: &str, severity: Severity) -> Recommendation {
    Recommendation::new(id, RecommendationCategory::General, severity, id, id)
}

fn guarded(
    grass: GrassType,
    history: &[Application],
    mut recs: Vec<Recommendation>,
) -> Recommendation {
    apply(&mut recs, &profile(grass), history, today());
    recs.remove(0)
}

fn budget_point(rec: &Recommendation) -> String {
    let points: Vec<_> = rec
        .data_points
        .iter()
        .filter(|d| d.label == "N budget remaining")
        .collect();
    assert_eq!(points.len(), 1);
    points[0].value.clone()
}

// @spec RULE-GUARD-003, RULE-BUDGET-001
#[test]
fn each_feeding_shows_this_years_remaining_lawn_nitrogen() {
    let history = [
        fed(1.0, date(2026, 4, 1)),
        fed(1.0, date(2026, 10, 20)), // not yet
        fed(1.0, date(2025, 9, 1)),   // last year
        // Not lawn nitrogen: plant fertilizer, and entries missing an analysis or a rate.
        app(
            ApplicationType::PlantFertilizer,
            date(2026, 5, 1),
            Some(20.0),
            Some(5.0),
        ),
        app(
            ApplicationType::Fertilizer,
            date(2026, 5, 1),
            None,
            Some(5.0),
        ),
        app(
            ApplicationType::Fertilizer,
            date(2026, 5, 1),
            Some(20.0),
            None,
        ),
    ];
    let rec = guarded(GrassType::TallFescue, &history, vec![feeding(0.75)]);
    assert_eq!(budget_point(&rec), "2.50 of 3.5 lb/1000 sqft");
    // Over the target: shown as 0, never negative.
    let over = [fed(1.5, date(2026, 4, 1)), fed(1.5, date(2026, 9, 1))];
    let rec = guarded(GrassType::FineFescue, &over, vec![feeding(0.75)]);
    assert_eq!(budget_point(&rec), "0.00 of 2.0 lb/1000 sqft");
}

// @spec RULE-GUARD-004
#[test]
fn a_spent_budget_withdraws_a_nitrogen_only_feeding() {
    let spent = [fed(1.0, date(2026, 4, 1)), fed(0.98, date(2026, 9, 1))];
    let rec = guarded(GrassType::FineFescue, &spent, vec![feeding(0.75)]);
    assert_eq!(rec.severity, Severity::Info);
    assert_eq!(
        rec.title,
        "Main Fall Feeding — annual nitrogen target reached"
    );
    assert!(
        rec.description
            .contains("applied 1.98 lb N/1000 sqft this year against a target of 2.0"),
        "{}",
        rec.description
    );
    assert!(rec
        .description
        .contains("Skip this feeding, or use a product without nitrogen"));
    assert!(!rec.description.contains("Feed now"));
    assert!(rec.suggested_action.is_none());
    assert!(rec.needs.is_empty());
    // 0.06 lb left is still room: not withdrawn.
    let nearly = [fed(1.0, date(2026, 4, 1)), fed(0.94, date(2026, 9, 1))];
    let rec = guarded(GrassType::FineFescue, &nearly, vec![feeding(0.75)]);
    assert!(!rec.title.contains("target reached"));
    assert!(!rec.needs.is_empty());
}

// @spec RULE-GUARD-006
#[test]
fn less_room_than_the_largest_feeding_limits_this_one() {
    // 0.75 left: more than this 0.5 feeding, less than the largest (1.0).
    let history = [fed(2.75, date(2026, 4, 1))];
    let rec = guarded(GrassType::TallFescue, &history, vec![feeding(0.5)]);
    assert_eq!(rec.severity, Severity::Warning);
    assert!(
        rec.description
            .contains("Keep this application at or under 0.75 lb N/1000 sqft"),
        "{}",
        rec.description
    );
    assert!(rec.description.contains(&amount_text(0.5, Some(5000.0))));
    // 0.5 left under a 1.0 feeding: the amount itself is limited.
    let history = [fed(3.0, date(2026, 4, 1))];
    let rec = guarded(GrassType::TallFescue, &history, vec![feeding(1.0)]);
    assert_eq!(rec.severity, Severity::Warning);
    assert!(
        rec.description.contains("at or under 0.50"),
        "{}",
        rec.description
    );
    // More than a full feeding's worth left: no limit.
    let history = [fed(2.0, date(2026, 4, 1))];
    let rec = guarded(GrassType::TallFescue, &history, vec![feeding(1.0)]);
    assert!(
        !rec.description.contains("at or under"),
        "{}",
        rec.description
    );
}

fn held(blocker: Recommendation) -> Recommendation {
    guarded(GrassType::TallFescue, &[], vec![feeding(1.0), blocker])
}

fn is_held(rec: &Recommendation) -> bool {
    rec.severity == Severity::Info
        && rec.title == "On hold — Main Fall Feeding"
        && rec
            .description
            .starts_with("Hold off for now: current conditions rule out fertilizer")
}

// @spec RULE-GUARD-008
#[test]
fn a_fertilizer_block_or_real_heat_puts_feedings_on_hold() {
    assert!(is_held(&held(blocker("fertilizer_block", Severity::Info))));
    assert!(is_held(&held(blocker(
        "fertilizer_block",
        Severity::Critical
    ))));
    assert!(is_held(&held(blocker(
        "heat_stress_forecast",
        Severity::Warning
    ))));
    assert!(is_held(&held(blocker(
        "heat_stress_forecast",
        Severity::Critical
    ))));
    let warm = held(blocker("heat_stress_forecast", Severity::Advisory));
    assert!(!is_held(&warm));
    assert_eq!(warm.severity, Severity::Warning);
}

// @spec RULE-GUARD-009
#[test]
fn a_held_feeding_past_the_target_carries_both_edits() {
    let spent = [fed(2.0, date(2026, 4, 1))];
    let rec = guarded(
        GrassType::FineFescue,
        &spent,
        vec![
            feeding(0.75),
            blocker("fertilizer_block", Severity::Warning),
        ],
    );
    assert_eq!(rec.severity, Severity::Info);
    assert_eq!(
        rec.title,
        "On hold — Main Fall Feeding — annual nitrogen target reached"
    );
    assert!(rec.description.starts_with("Hold off for now"));
    assert!(rec.description.contains("Skip this feeding"));
    assert!(rec.needs.is_empty() && rec.suggested_action.is_none());
}

// @spec RULE-GUARD-010
#[test]
fn an_answered_blocker_still_holds_feedings() {
    let mut dismissed = blocker("fertilizer_block", Severity::Warning);
    dismissed.dismissed = true;
    assert!(is_held(&held(dismissed)));
    let mut addressed = blocker("heat_stress_forecast", Severity::Warning);
    addressed.addressed = true;
    assert!(is_held(&held(addressed)));

    // Through the feed: the owner's answer hides the block, the feeding stays on hold.
    let answers = HashMap::from([(
        "fertilizer_block".to_string(),
        RecommendationState {
            dismissed: true,
            addressed: false,
            severity: Some(Severity::Warning),
            updated_at: Utc::now(),
        },
    )]);
    let profile = profile(GrassType::TallFescue);
    let feed = crate::api::recommendation_feed::assemble(
        vec![feeding(1.0), blocker("fertilizer_block", Severity::Warning)],
        &crate::api::recommendation_feed::Stages {
            profile: &profile,
            history: &[],
            today: today(),
            products: None,
            answers: &answers,
            now: Utc::now(),
        },
    );
    let block = feed.iter().find(|r| r.id == "fertilizer_block").unwrap();
    assert!(block.dismissed);
    assert!(is_held(
        feed.iter().find(|r| r.id == "fall_fert_mid").unwrap()
    ));
}
