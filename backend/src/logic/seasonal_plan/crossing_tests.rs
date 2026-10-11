//! Crossing detection, aggregation, voting and the settled judgment.

use super::*;
use crate::models::seasonal_plan::{DailySoilTempAvg, ThresholdCrossing};

fn d(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// Daily 10 cm means: each segment is (first day, last day, °F), every calendar day present.
fn series(segments: &[(NaiveDate, NaiveDate, f64)]) -> Vec<DailySoilTempAvg> {
    segments
        .iter()
        .flat_map(|&(from, to, temp)| {
            from.iter_days()
                .take_while(move |day| *day <= to)
                .map(move |date| DailySoilTempAvg {
                    date,
                    avg_temp_f: temp,
                })
        })
        .collect()
}

fn without(mut rows: Vec<DailySoilTempAvg>, missing: NaiveDate) -> Vec<DailySoilTempAvg> {
    rows.retain(|r| r.date != missing);
    rows
}

fn crossing<'a>(crossings: &'a [ThresholdCrossing], name: &str) -> Option<&'a ThresholdCrossing> {
    let mut found = crossings.iter().filter(|c| c.threshold_name == name);
    let first = found.next();
    assert!(found.next().is_none(), "{name} crossed more than once");
    first
}

fn dated(crossings: &[ThresholdCrossing], name: &str) -> Option<NaiveDate> {
    crossing(crossings, name).map(|c| c.crossing_date)
}

// With 40°F and 60°F days, the 7-row trailing mean reaches 50°F on the 4th 60°F day and
// stays there exactly as many days as the 60°F block is long.

// @spec PLAN-CROSS-003, PLAN-CROSS-005
#[test]
fn a_rising_crossing_is_dated_on_the_first_day_of_its_held_run() {
    let rows = series(&[
        (d(2025, 3, 1), d(2025, 3, 20), 40.0),
        (d(2025, 3, 21), d(2025, 4, 30), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    let c = crossing(&crossings, "soil_50f_rising").expect("50°F rising");
    assert_eq!(c.crossing_date, d(2025, 3, 24));
    assert!(
        (c.avg_soil_temp_f - 360.0 / 7.0).abs() < 1e-9,
        "carries that day's trailing mean"
    );
}

// @spec PLAN-CROSS-008, PLAN-CROSS-005
#[test]
fn a_run_that_does_not_hold_five_days_is_not_a_crossing() {
    // Four 60°F days: four trailing means at or above 50°F, then back below.
    let rows = series(&[
        (d(2025, 3, 1), d(2025, 3, 20), 40.0),
        (d(2025, 3, 21), d(2025, 3, 24), 60.0),
        (d(2025, 3, 25), d(2025, 4, 3), 40.0),
        (d(2025, 4, 4), d(2025, 5, 31), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_50f_rising"), Some(d(2025, 4, 7)));
}

// @spec PLAN-CROSS-008
#[test]
fn the_end_of_the_years_data_before_the_fifth_day_is_no_crossing() {
    let rows = series(&[
        (d(2025, 3, 1), d(2025, 3, 20), 40.0),
        (d(2025, 3, 21), d(2025, 3, 26), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_50f_rising"), None);
}

// @spec PLAN-CROSS-008
#[test]
fn a_missing_date_ends_a_run_and_the_search_continues() {
    // The run starts Mar 24; Mar 25 is missing, and a cold Mar 26 pulls the mean back
    // below 50°F, so the run ends after one day. The next run starts Apr 8.
    let rows = series(&[
        (d(2025, 3, 1), d(2025, 3, 20), 40.0),
        (d(2025, 3, 21), d(2025, 3, 25), 60.0),
        (d(2025, 3, 26), d(2025, 3, 26), 0.0),
        (d(2025, 3, 27), d(2025, 4, 4), 40.0),
        (d(2025, 4, 5), d(2025, 5, 31), 60.0),
    ]);
    let rows = without(rows, d(2025, 3, 25));
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_50f_rising"), Some(d(2025, 4, 8)));
}

// @spec PLAN-CROSS-009
#[test]
fn a_held_run_right_after_a_missing_date_drops_the_threshold_for_the_year() {
    // Mar 24 is missing; the mean first reaches 50°F on Mar 25, the day after the hole,
    // and holds. That dates the outage, not the soil: no 50°F crossing this year, even
    // though a later run (May) would hold too.
    let rows = series(&[
        (d(2025, 3, 1), d(2025, 3, 20), 40.0),
        (d(2025, 3, 21), d(2025, 4, 10), 60.0),
        (d(2025, 4, 11), d(2025, 4, 30), 40.0),
        (d(2025, 5, 1), d(2025, 5, 31), 60.0),
    ]);
    let rows = without(rows, d(2025, 3, 24));
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_50f_rising"), None);
}

// @spec PLAN-CROSS-003
#[test]
fn a_run_that_begins_the_years_data_is_not_a_crossing() {
    let rows = series(&[
        (d(2025, 3, 1), d(2025, 3, 10), 60.0),
        (d(2025, 3, 11), d(2025, 3, 31), 40.0),
        (d(2025, 4, 1), d(2025, 4, 30), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_50f_rising"), Some(d(2025, 4, 4)));
}

// @spec PLAN-CROSS-003
#[test]
fn a_rising_run_must_begin_by_june() {
    let june = series(&[
        (d(2025, 6, 1), d(2025, 6, 26), 40.0),
        (d(2025, 6, 27), d(2025, 7, 31), 60.0),
    ]);
    assert_eq!(
        dated(&find_threshold_crossings(2025, &june), "soil_50f_rising"),
        Some(d(2025, 6, 30))
    );
    let july = series(&[
        (d(2025, 6, 1), d(2025, 6, 27), 40.0),
        (d(2025, 6, 28), d(2025, 7, 31), 60.0),
    ]);
    assert_eq!(
        dated(&find_threshold_crossings(2025, &july), "soil_50f_rising"),
        None
    );
}

// With 70°F and 60°F days the trailing mean reaches 65°F on the 4th 60°F day.

// @spec PLAN-CROSS-004, PLAN-CROSS-005
#[test]
fn a_falling_crossing_is_dated_on_the_first_day_of_its_held_run() {
    let rows = series(&[
        (d(2025, 1, 1), d(2025, 9, 10), 70.0),
        (d(2025, 9, 11), d(2025, 12, 31), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    let c = crossing(&crossings, "soil_65f_falling").expect("65°F falling");
    assert_eq!(c.crossing_date, d(2025, 9, 14));
    assert!((c.avg_soil_temp_f - 450.0 / 7.0).abs() < 1e-9);
}

// @spec PLAN-CROSS-004, PLAN-CROSS-008
#[test]
fn a_brief_fall_cool_spell_is_not_a_falling_crossing() {
    let rows = series(&[
        (d(2025, 1, 1), d(2025, 9, 10), 70.0),
        (d(2025, 9, 11), d(2025, 9, 14), 60.0),
        (d(2025, 9, 15), d(2025, 9, 30), 70.0),
        (d(2025, 10, 1), d(2025, 12, 31), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_65f_falling"), Some(d(2025, 10, 4)));
}

// @spec PLAN-CROSS-004
#[test]
fn a_falling_run_that_begins_in_june_is_not_a_fall_crossing() {
    let rows = series(&[
        (d(2025, 1, 1), d(2025, 6, 20), 70.0),
        (d(2025, 6, 21), d(2025, 8, 31), 60.0),
    ]);
    let crossings = find_threshold_crossings(2025, &rows);
    assert_eq!(dated(&crossings, "soil_65f_falling"), None);
}

fn crossed(year: i32, month: u32, day: u32) -> ThresholdCrossing {
    ThresholdCrossing {
        year,
        threshold_name: "soil_50f_rising".into(),
        crossing_date: d(year, month, day),
        avg_soil_temp_f: 50.0,
    }
}

// @spec PLAN-CROSS-006
#[test]
fn aggregation_maps_crossings_by_month_and_day() {
    // Mar 1 is day 61 in a leap year and day 60 otherwise; by month and day it stays Mar 1.
    let agg = aggregate_crossings(&[crossed(2024, 3, 1)], "soil_50f_rising", 2025).unwrap();
    assert_eq!(agg.median, d(2025, 3, 1));
    let agg = aggregate_crossings(&[crossed(2023, 3, 1)], "soil_50f_rising", 2024).unwrap();
    assert_eq!(agg.median, d(2024, 3, 1));
    let agg = aggregate_crossings(&[crossed(2024, 2, 29)], "soil_50f_rising", 2025).unwrap();
    assert_eq!(agg.median, d(2025, 2, 28));
}

// @spec PLAN-CROSS-006
#[test]
fn aggregation_reports_median_at_n_over_two_with_the_spread() {
    let crossings = [
        crossed(2021, 4, 20),
        crossed(2022, 4, 1),
        crossed(2023, 4, 10),
        crossed(2024, 4, 5),
    ];
    let agg = aggregate_crossings(&crossings, "soil_50f_rising", 2026).unwrap();
    assert_eq!(
        (agg.earliest, agg.median, agg.latest),
        (d(2026, 4, 1), d(2026, 4, 10), d(2026, 4, 20))
    );
    assert_eq!(agg.sample_count, 4);
    assert!(aggregate_crossings(&crossings, "soil_55f_rising", 2026).is_none());
}

// @spec PLAN-CROSS-010
#[test]
fn only_the_ten_most_recent_settled_years_before_the_plan_year_vote() {
    let settled: Vec<i32> = (2010..=2026).rev().collect();
    assert_eq!(
        voting_years(&settled, 2026),
        (2016..=2025).collect::<Vec<_>>()
    );
    assert_eq!(
        voting_years(&settled, 2020),
        (2010..=2019).collect::<Vec<_>>()
    );
}

// @spec PLAN-CACHE-005
#[test]
fn a_year_is_settled_only_after_it_ends_and_the_lake_has_reported_december_31() {
    assert!(
        !year_settled(2025, d(2025, 12, 31), Some(d(2025, 12, 31))),
        "year not over"
    );
    assert!(
        !year_settled(2025, d(2026, 1, 1), Some(d(2025, 12, 30))),
        "lake still behind"
    );
    assert!(!year_settled(2025, d(2026, 1, 1), None), "no lake reading");
    assert!(year_settled(2025, d(2026, 1, 1), Some(d(2025, 12, 31))));
    assert!(year_settled(2024, d(2026, 3, 1), Some(d(2026, 2, 28))));
}

// @spec PLAN-CACHE-008, PLAN-CROSS-001
#[test]
fn a_year_too_thin_to_analyze_still_settles() {
    let rows = series(&[
        (d(2024, 3, 1), d(2024, 3, 20), 40.0),
        (d(2024, 3, 21), d(2024, 3, 29), 60.0),
    ]);
    let analysis = analyze_year(2024, &rows, d(2026, 1, 5), Some(d(2026, 1, 4)));
    assert!(
        analysis.crossings.is_empty(),
        "fewer than 30 daily means: no crossings"
    );
    assert!(
        analysis.settled,
        "a marker settles the year so it is not re-read on every load"
    );

    let this_year = analyze_year(2026, &rows, d(2026, 1, 5), Some(d(2026, 1, 4)));
    assert!(!this_year.settled);
}
