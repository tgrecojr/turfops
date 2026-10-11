//! The 10 cm windows, their historical ranges, and which years vote for them.

use super::test_support::*;
use super::*;

// @spec PLAN-ACT-005, PLAN-ACT-006, PLAN-ACT-007, PLAN-ACT-008, PLAN-ACT-011
#[test]
fn ten_cm_windows_open_and_close_on_their_medians() {
    let p = plan(&[]);
    assert_eq!(
        window(&p, "spring_herbicide"),
        (d(2026, 3, 30), d(2026, 4, 30))
    );
    assert_eq!(
        window(&p, "grub_preventative"),
        (d(2026, 5, 15), d(2026, 6, 27))
    );
    assert_eq!(
        window(&p, "core_aeration"),
        (d(2026, 9, 25), d(2026, 11, 7))
    );
    assert_eq!(
        window(&p, "fall_overseeding"),
        (d(2026, 9, 25), d(2026, 11, 7))
    );
    assert_eq!(
        window(&p, "fall_herbicide"),
        (d(2026, 9, 25), d(2026, 10, 27))
    );

    let thin = plan_with(
        &history(&[
            "soil_55f_rising",
            "soil_75f_rising",
            "soil_50f_falling",
            "soil_55f_falling",
        ]),
        &[],
    );
    assert_eq!(
        window(&thin, "spring_herbicide"),
        (d(2026, 3, 30), d(2026, 4, 27))
    );
    assert_eq!(
        window(&thin, "grub_preventative"),
        (d(2026, 5, 15), d(2026, 7, 4))
    );
    assert_eq!(
        window(&thin, "core_aeration"),
        (d(2026, 9, 25), d(2026, 10, 25))
    );
    assert_eq!(
        window(&thin, "fall_overseeding"),
        (d(2026, 9, 25), d(2026, 10, 23))
    );
    assert_eq!(
        window(&thin, "fall_herbicide"),
        (d(2026, 9, 25), d(2026, 10, 23))
    );
}

// @spec PLAN-ACT-017
#[test]
fn spring_herbicide_edges_keep_their_month_and_day() {
    // 50°F median Apr 6 − 7 = Mar 30; 55°F median Apr 23 + 7 = Apr 30.
    let (start, end) = window(&plan(&[]), "spring_herbicide");
    assert_eq!(start, d(2026, 3, 30));
    assert_eq!(end, d(2026, 4, 30));
}

// @spec PLAN-ACT-012
#[test]
fn every_window_carries_its_historical_range() {
    let p = plan(&[]);
    assert_eq!(
        range(&p, "pre_emergent"),
        (Some(d(2026, 4, 4)), Some(d(2026, 5, 14)))
    );
    assert_eq!(
        range(&p, "spring_nitrogen"),
        (Some(d(2026, 4, 21)), Some(d(2026, 5, 29)))
    );
    assert_eq!(
        range(&p, "spring_herbicide"),
        (Some(d(2026, 3, 28)), Some(d(2026, 5, 2)))
    );
    assert_eq!(
        range(&p, "early_fall_fert"),
        (Some(d(2026, 9, 23)), Some(d(2026, 10, 18)))
    );
    assert_eq!(
        range(&p, "mid_fall_fert"),
        (Some(d(2026, 10, 14)), Some(d(2026, 11, 8)))
    );
    assert_eq!(
        range(&p, "winterizer"),
        (Some(d(2026, 11, 20)), Some(d(2026, 12, 8)))
    );
    let thin = plan_with(&history(&["soil_55f_falling"]), &[]);
    assert_eq!(range(&thin, "fall_herbicide"), (Some(d(2026, 9, 23)), None));
}

fn every_year_on(years: impl Iterator<Item = i32>, month: u32, day: u32) -> Vec<ThresholdCrossing> {
    years
        .map(|year| ThresholdCrossing {
            year,
            threshold_name: "soil_50f_rising".into(),
            crossing_date: d(year, month, day),
            avg_soil_temp_f: 50.0,
        })
        .collect()
}

// @spec PLAN-CROSS-010
#[test]
fn only_settled_years_before_the_plan_year_vote_ten_at_most() {
    let mut crossings = every_year_on(2015..=2024, 4, 10);
    // Too old (11th and 12th), unsettled, and the plan year itself: none may vote.
    crossings.extend(every_year_on([2013, 2014, 2025, 2026].into_iter(), 3, 1));
    let settled: Vec<i32> = (2013..=2024).collect();
    let p = build_seasonal_plan(
        2026,
        &crossings,
        &settled,
        &[],
        GrassType::TallFescue,
        d(2026, 1, 15),
    );
    assert_eq!(p.data_years_used, 10);
    assert_eq!(range(&p, "pre_emergent").0, Some(d(2026, 4, 10)));
}

// @spec PLAN-CROSS-010
#[test]
fn a_settled_year_with_no_crossings_does_not_take_a_vote() {
    let mut crossings = every_year_on((2015..=2024).filter(|y| *y != 2020), 4, 10);
    crossings.extend(every_year_on([2014].into_iter(), 3, 1));
    let settled: Vec<i32> = (2014..=2024).collect();
    let p = build_seasonal_plan(
        2026,
        &crossings,
        &settled,
        &[],
        GrassType::TallFescue,
        d(2026, 1, 15),
    );
    assert_eq!(p.data_years_used, 10);
    assert_eq!(
        range(&p, "pre_emergent").0,
        Some(d(2026, 3, 1)),
        "2014 votes in 2020's place"
    );
}
