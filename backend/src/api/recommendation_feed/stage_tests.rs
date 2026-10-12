//! The stages after the sources: what the nitrogen guard, the shelf and the owner's
//! answers do to the assembled feed, and in which order.

use super::fixtures::{application, plant_fertilizing_on, profile};
use super::{assemble, Stages};
use crate::logic::plant_maintenance::generate_plant_maintenance_recommendations;
use crate::models::{
    Application, ApplicationType, EnvironmentalSummary, InventoryState, ProductCategory,
    ProductNeed, Recommendation, RecommendationCategory, RecommendationState, Severity,
};
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 5).unwrap()
}

fn rec(id: &str, category: RecommendationCategory, severity: Severity) -> Recommendation {
    Recommendation::new(id, category, severity, id, id)
}

fn feeding(id: &str, severity: Severity) -> Recommendation {
    rec(id, RecommendationCategory::Fertilizer, severity)
        .with_feeding(1.0)
        .with_need(
            ProductNeed::new("lawn fertilizer", ProductCategory::Fertilizer).requiring_nitrogen(),
        )
}

fn block() -> Recommendation {
    rec(
        "fertilizer_block",
        RecommendationCategory::Fertilizer,
        Severity::Warning,
    )
}

fn answer(severity: Severity, dismissed: bool, addressed: bool) -> RecommendationState {
    RecommendationState {
        dismissed,
        addressed,
        severity: Some(severity),
        updated_at: Utc::now(),
    }
}

fn run(
    recs: Vec<Recommendation>,
    history: &[Application],
    answers: &HashMap<String, RecommendationState>,
) -> Vec<Recommendation> {
    let profile = profile();
    assemble(
        recs,
        &Stages {
            profile: &profile,
            history,
            today: today(),
            products: Some(&[]),
            answers,
            now: Utc::now(),
        },
    )
}

fn find<'a>(feed: &'a [Recommendation], id: &str) -> &'a Recommendation {
    feed.iter().find(|r| r.id == id).expect(id)
}

// @spec FEED-PIPE-005
#[test]
fn a_plant_feeding_is_not_held_or_budgeted_by_the_nitrogen_guard() {
    let plant = crate::models::plant::Plant {
        id: Some(12),
        ..plant_fertilizing_on(1, today())
    };
    let plant_recs = generate_plant_maintenance_recommendations(
        &[plant],
        &[],
        &EnvironmentalSummary::default(),
        crate::clock::Moment {
            now: chrono::Utc::now(),
            today: today(),
        },
    );
    assert_eq!(plant_recs.len(), 1, "the fertilizing task is due");
    let id = plant_recs[0].id.clone();
    let before = plant_recs[0].clone();
    assert!(
        before.feeding.is_none(),
        "plants are outside the lawn budget"
    );

    let mut recs = vec![block()];
    recs.extend(plant_recs);
    let feed = run(recs, &[], &HashMap::new());
    let after = find(&feed, &id);
    assert_eq!(after.severity, before.severity, "not put on hold");
    assert_eq!(after.title, before.title);
    assert!(!after
        .data_points
        .iter()
        .any(|d| d.label == "N budget remaining"));
}

// @spec FEED-PIPE-007
#[test]
fn the_shelf_reads_needs_after_the_guard_has_cleared_them() {
    // 4.6 lb N already down this year: the tall fescue target is met, so the guard
    // clears the feeding's nitrogen need before the shelf is asked about it.
    let fed = Application {
        rate_per_1000sqft: Some(10.0),
        nitrogen_pct: Some(46.0),
        ..application(
            1,
            ApplicationType::Fertilizer,
            today() - chrono::Duration::days(60),
            None,
        )
    };
    let feed = run(
        vec![feeding("fall_fert_early", Severity::Advisory)],
        &[fed],
        &HashMap::new(),
    );
    let rec = find(&feed, "fall_fert_early");
    assert!(rec.title.contains("target reached"), "{}", rec.title);
    assert!(rec.needs.is_empty());
    assert!(
        rec.inventory.is_none(),
        "no \"need to buy\" for a skipped feeding"
    );

    // Without the guard clearing it, the same empty shelf is a "need to buy" verdict —
    // and an answered rec is enriched too, so the shelf runs before answers apply.
    let answers = HashMap::from([(
        "fall_fert_early".to_string(),
        answer(Severity::Advisory, true, false),
    )]);
    let feed = run(
        vec![feeding("fall_fert_early", Severity::Advisory)],
        &[],
        &answers,
    );
    let rec = find(&feed, "fall_fert_early");
    assert!(rec.dismissed);
    assert_eq!(
        rec.inventory.as_ref().map(|s| s.state),
        Some(InventoryState::NotOnHand)
    );
}

// @spec FEED-PIPE-008
#[test]
fn an_answer_is_compared_with_the_severity_after_the_guard() {
    // Answered while on hold (Info). The source still says Advisory.
    let answers = HashMap::from([(
        "fall_fert_early".to_string(),
        answer(Severity::Info, true, false),
    )]);

    // Still blocked: the guard lowers it to Info again, so the answer holds.
    let held = run(
        vec![block(), feeding("fall_fert_early", Severity::Advisory)],
        &[],
        &answers,
    );
    assert!(!find(&held, "fall_fert_early").is_active());

    // Block lifted: it is Advisory again, above the answer, so it returns.
    let lifted = run(
        vec![feeding("fall_fert_early", Severity::Advisory)],
        &[],
        &answers,
    );
    let rec = find(&lifted, "fall_fert_early");
    assert_eq!(rec.severity, Severity::Advisory);
    assert!(rec.is_active());
}

// @spec FEED-STATE-009
#[test]
fn a_holding_answer_copies_its_flags_and_one_that_lapsed_leaves_the_rec_active() {
    let answers = HashMap::from([
        (
            "aeration".to_string(),
            answer(Severity::Advisory, false, true),
        ),
        (
            "disease_brown_patch".to_string(),
            answer(Severity::Info, true, false),
        ),
    ]);
    let feed = run(
        vec![
            rec(
                "aeration",
                RecommendationCategory::Aeration,
                Severity::Advisory,
            ),
            rec(
                "disease_brown_patch",
                RecommendationCategory::DiseasePressure,
                Severity::Critical,
            ),
        ],
        &[],
        &answers,
    );
    let aeration = find(&feed, "aeration");
    assert!(aeration.addressed && !aeration.dismissed);
    let escalated = find(&feed, "disease_brown_patch");
    assert!(escalated.is_active(), "escalated past the answer");
    assert!(!escalated.dismissed && !escalated.addressed);
}
