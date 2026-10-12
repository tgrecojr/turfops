//! Which 10 cm activities appear, their pre-emergent / spring nitrogen / fall feeding
//! windows, and their date-based status.

use super::test_support::*;
use super::*;

fn has(plan: &SeasonalPlan, id: &str) -> bool {
    plan.activities.iter().any(|a| a.id == id)
}

// @spec PLAN-ACT-001
#[test]
fn an_activity_whose_opening_threshold_never_crossed_is_left_off() {
    let cases: [(&str, &[&str]); 5] = [
        ("soil_50f_rising", &["pre_emergent", "spring_herbicide"]),
        ("soil_55f_rising", &["spring_nitrogen"]),
        ("soil_60f_rising", &["grub_preventative"]),
        (
            "soil_65f_falling",
            &[
                "core_aeration",
                "fall_overseeding",
                "early_fall_fert",
                "mid_fall_fert",
                "fall_herbicide",
            ],
        ),
        ("soil_45f_falling", &["winterizer"]),
    ];
    for (missing, gone) in cases {
        let p = plan_with(&history(&[missing]), &[]);
        for id in gone {
            assert!(!has(&p, id), "{id} without {missing}");
        }
        assert_eq!(p.activities.len(), 10 - gone.len(), "only those: {missing}");
    }
}

// @spec PLAN-ACT-002
#[test]
fn pre_emergent_runs_from_the_50_to_the_60_degree_rising_median() {
    // Medians: 50°F rising Apr 6, 60°F rising May 12.
    assert_eq!(
        window(&plan(&[]), "pre_emergent"),
        (d(2026, 4, 6), d(2026, 5, 12))
    );
    let thin = plan_with(&history(&["soil_60f_rising"]), &[]);
    assert_eq!(
        window(&thin, "pre_emergent"),
        (d(2026, 4, 6), d(2026, 5, 6)),
        "30 days when 60°F never crossed"
    );
}

// @spec PLAN-ACT-003
#[test]
fn spring_nitrogen_runs_from_the_55_to_the_65_degree_rising_median() {
    // Medians: 55°F rising Apr 23, 65°F rising May 27.
    assert_eq!(
        window(&plan(&[]), "spring_nitrogen"),
        (d(2026, 4, 23), d(2026, 5, 27))
    );
    let thin = plan_with(&history(&["soil_65f_rising"]), &[]);
    assert_eq!(
        window(&thin, "spring_nitrogen"),
        (d(2026, 4, 23), d(2026, 5, 14)),
        "21 days when 65°F never crossed"
    );
}

// @spec PLAN-ACT-009
#[test]
fn fall_feedings_follow_the_65_and_45_degree_falling_medians() {
    // Medians: 65°F falling Sep 25, 45°F falling Nov 22.
    let p = plan(&[]);
    assert_eq!(
        window(&p, "early_fall_fert"),
        (d(2026, 9, 25), d(2026, 10, 16))
    );
    assert_eq!(
        window(&p, "mid_fall_fert"),
        (d(2026, 10, 16), d(2026, 11, 6))
    );
    assert_eq!(window(&p, "winterizer"), (d(2026, 11, 22), d(2026, 12, 6)));
}

fn pre_emergent_on(today: NaiveDate, applications: &[Application]) -> ActivityStatus {
    let p = build_seasonal_plan(
        PLAN_YEAR,
        &history(&[]),
        &SETTLED,
        applications,
        GrassType::TallFescue,
        today,
    );
    activity(&p, "pre_emergent").status
}

// @spec PLAN-ACT-013
#[test]
fn status_follows_the_log_then_the_window_on_the_given_day() {
    // The pre-emergent window is Apr 6 – May 12.
    use ActivityStatus::*;
    assert_eq!(pre_emergent_on(d(2026, 4, 5), &[]), Upcoming);
    assert_eq!(pre_emergent_on(d(2026, 4, 6), &[]), Active);
    assert_eq!(pre_emergent_on(d(2026, 5, 12), &[]), Active);
    assert_eq!(pre_emergent_on(d(2026, 5, 13), &[]), Missed);
    let logged = [applied(ApplicationType::PreEmergent, d(2026, 3, 20))];
    assert_eq!(pre_emergent_on(d(2026, 5, 13), &logged), Completed);
    assert_eq!(pre_emergent_on(d(2026, 1, 15), &logged), Completed);
}
