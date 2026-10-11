//! Current-season statuses from the rules, estimates, and the plan's `today`.

use super::test_support::*;
use super::*;

fn statuses(plan: &SeasonalPlan) -> Vec<(String, ActivityStatus, bool)> {
    plan.activities
        .iter()
        .map(|a| (a.id.clone(), a.status, a.status_estimated))
        .collect()
}

fn find(plan: &SeasonalPlan, id: &str) -> (ActivityStatus, bool) {
    let a = activity(plan, id);
    (a.status, a.status_estimated)
}

// @spec PLAN-ACT-021
#[test]
fn covered_activities_take_their_status_from_the_rules() {
    let mut p = plan(&[]);
    let before = find(&p, "pre_emergent");
    let states = [
        ("spring_nitrogen", SeasonState::Done),
        ("spring_herbicide", SeasonState::Passed),
        ("grub_preventative", SeasonState::Due),
        ("early_fall_fert", SeasonState::NotNeeded),
        ("mid_fall_fert", SeasonState::NotYet),
        ("winterizer", SeasonState::NotYet),
        ("fall_herbicide", SeasonState::Due),
    ];
    apply_rule_states(&mut p.activities, Some(&states));
    assert_eq!(
        find(&p, "spring_nitrogen"),
        (ActivityStatus::Completed, false)
    );
    assert_eq!(
        find(&p, "spring_herbicide"),
        (ActivityStatus::Missed, false)
    );
    assert_eq!(
        find(&p, "grub_preventative"),
        (ActivityStatus::Active, false)
    );
    assert_eq!(
        find(&p, "early_fall_fert"),
        (ActivityStatus::Skipped, false)
    );
    assert_eq!(find(&p, "mid_fall_fert"), (ActivityStatus::Upcoming, false));
    assert_eq!(find(&p, "fall_herbicide"), (ActivityStatus::Active, false));
    assert_eq!(
        find(&p, "pre_emergent"),
        before,
        "not rule-covered: untouched"
    );
}

// @spec PLAN-ACT-022
#[test]
fn an_unknown_state_keeps_the_typical_date_status_as_an_estimate() {
    // Judged on Jan 15: every 10 cm window is still ahead.
    let mut p = plan(&[]);
    apply_rule_states(
        &mut p.activities,
        Some(&[("grub_preventative", SeasonState::Unknown)]),
    );
    assert_eq!(
        find(&p, "grub_preventative"),
        (ActivityStatus::Upcoming, true)
    );
    assert!(!find(&p, "spring_nitrogen").1, "no state given: left alone");
}

// @spec PLAN-ACT-022
#[test]
fn without_a_feed_every_covered_activity_is_an_estimate() {
    let mut p = plan(&[]);
    apply_rule_states(&mut p.activities, None);
    for (id, _, estimated) in statuses(&p) {
        let covered = [
            "spring_nitrogen",
            "spring_herbicide",
            "grub_preventative",
            "early_fall_fert",
            "mid_fall_fert",
            "winterizer",
            "fall_herbicide",
        ]
        .contains(&id.as_str());
        assert_eq!(estimated, covered, "{id}");
    }
}

// @spec PLAN-ACT-023
#[test]
fn statuses_are_five_and_estimates_default_off() {
    assert_eq!(
        serde_json::to_value(ActivityStatus::Skipped).unwrap(),
        "Skipped"
    );
    let p = plan(&[]);
    assert!(p.activities.iter().all(|a| !a.status_estimated));
    let json = serde_json::to_value(&p.activities[0]).unwrap();
    assert_eq!(json["status_estimated"], false);
}

// @spec PLAN-MERGE-008
#[test]
fn the_plan_carries_the_date_its_statuses_were_judged_on() {
    let p = plan(&[]);
    assert_eq!(p.today, d(PLAN_YEAR, 1, 15));
    assert_eq!(serde_json::to_value(&p).unwrap()["today"], "2026-01-15");
}
