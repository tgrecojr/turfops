//! When the 10 cm activities count as done, and when fall herbicide steps aside for seed.

use super::test_support::*;
use super::*;

// @spec PLAN-ACT-004
#[test]
fn spring_nitrogen_is_completed_only_by_a_spring_feeding() {
    let fert = |m, day| applied(ApplicationType::Fertilizer, d(PLAN_YEAR, m, day));
    assert!(
        !completed(&plan(&[on_a_plant(fert(4, 10))]), "spring_nitrogen"),
        "plant fertilizer"
    );
    assert!(
        !completed(&plan(&[analysis(fert(4, 10), 0.0)]), "spring_nitrogen"),
        "potash only"
    );
    assert!(
        !completed(&plan(&[fert(6, 2)]), "spring_nitrogen"),
        "June is past spring"
    );
    assert!(completed(
        &plan(&[analysis(fert(4, 10), 10.0)]),
        "spring_nitrogen"
    ));
    assert!(
        completed(&plan(&[fert(5, 31)]), "spring_nitrogen"),
        "unrecorded N counts"
    );
}

// @spec PLAN-ACT-010
#[test]
fn fall_feedings_are_completed_only_by_a_feeding_in_their_month() {
    let fert = |m, day| applied(ApplicationType::Fertilizer, d(PLAN_YEAR, m, day));
    let potash = plan(&[analysis(fert(9, 10), 0.0), on_a_plant(fert(10, 10))]);
    assert!(
        !completed(&potash, "early_fall_fert"),
        "0-0-x is not a feeding"
    );
    assert!(
        !completed(&potash, "mid_fall_fert"),
        "plant fertilizer is not a feeding"
    );
    let fed = plan(&[fert(9, 10), fert(10, 10), fert(12, 1)]);
    for id in ["early_fall_fert", "mid_fall_fert", "winterizer"] {
        assert!(completed(&fed, id), "{id}");
    }
    assert!(!completed(&plan(&[fert(10, 31)]), "winterizer"));
}

// @spec PLAN-ACT-020, PLAN-ACT-005, PLAN-ACT-006, PLAN-ACT-007, PLAN-ACT-008, PLAN-ACT-011
#[test]
fn only_turf_applications_complete_an_activity() {
    let cases = [
        (
            "spring_herbicide",
            ApplicationType::PostEmergent,
            d(PLAN_YEAR, 4, 15),
        ),
        (
            "grub_preventative",
            ApplicationType::Insecticide,
            d(PLAN_YEAR, 6, 1),
        ),
        (
            "grub_preventative",
            ApplicationType::GrubControl,
            d(PLAN_YEAR, 6, 1),
        ),
        (
            "core_aeration",
            ApplicationType::Aeration,
            d(PLAN_YEAR, 9, 20),
        ),
        (
            "fall_overseeding",
            ApplicationType::Overseed,
            d(PLAN_YEAR, 9, 30),
        ),
        (
            "fall_herbicide",
            ApplicationType::PostEmergent,
            d(PLAN_YEAR, 10, 1),
        ),
    ];
    for (id, kind, on) in cases {
        let turf = applied(kind, on);
        assert!(
            !completed(&plan(&[on_a_plant(turf.clone())]), id),
            "{id}: a plant {kind:?}"
        );
        assert!(completed(&plan(&[turf]), id), "{id}: a turf {kind:?}");
    }
    let june = applied(ApplicationType::PostEmergent, d(PLAN_YEAR, 6, 1));
    assert!(!completed(
        &plan(std::slice::from_ref(&june)),
        "spring_herbicide"
    ));
    assert!(!completed(&plan(&[june]), "fall_herbicide"));
}

// @spec PLAN-ACT-018
#[test]
fn fall_herbicide_hides_only_for_seed_down_in_the_sixty_days_before_it_opens() {
    let has = |apps: &[Application]| {
        plan(apps)
            .activities
            .iter()
            .any(|a| a.id == "fall_herbicide")
    };
    let seeded = |m, day| applied(ApplicationType::Overseed, d(PLAN_YEAR, m, day));
    // The window opens on the 65°F falling median, Sep 25.
    assert!(!has(&[seeded(9, 1)]), "seeded 24 days before it opens");
    assert!(!has(&[seeded(7, 27)]), "seeded 60 days before it opens");
    assert!(has(&[seeded(7, 26)]), "seeded 61 days before it opens");
    assert!(has(&[seeded(9, 25)]), "seeded the day it opens");
    assert!(has(&[seeded(10, 10)]), "seeded after it opens");
    let sprayed = applied(ApplicationType::PostEmergent, d(PLAN_YEAR, 10, 1));
    assert!(
        has(&[seeded(9, 1), sprayed]),
        "already done: keep it on the plan"
    );
}

// @spec PLAN-ACT-019
#[test]
fn the_fallback_pre_emergent_is_completed_only_by_a_spring_application() {
    let pre = |m| applied(ApplicationType::PreEmergent, d(PLAN_YEAR, m, 15));
    assert!(
        !completed(&plan(&[pre(9)]), "pre_emergent"),
        "a fall pre-emergent"
    );
    assert!(completed(&plan(&[pre(3)]), "pre_emergent"));
    assert!(completed(&plan(&[pre(6)]), "pre_emergent"));
}
