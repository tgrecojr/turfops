//! The station's history: which years count, freeze dates, GDD dates, and how a
//! boundary's past dates are found and summarized.

use super::*;
use crate::api::timing::climate_start;
use crate::logic::timing::climatology::{
    date_stat, fall_freeze_history, first_fall_freeze, gdd_reached, history_years,
    last_spring_freeze, spring_freeze_history,
};
use crate::logic::timing::evaluate::typical;
use crate::logic::timing::series::{Crossing, Direction, SeasonDay};
use crate::logic::timing::windows::Boundary;
use crate::models::seasonal_plan::WindowConfidence;

// @spec TIME-ENG-015
#[test]
fn history_is_up_to_15_complete_years_with_300_soil_days() {
    let today = date(2026, 9, 20);
    // Twenty years on the station; the record read starts 15 years back.
    let mut days: Vec<ClimateDay> = date(2006, 1, 1)
        .iter_days()
        .take_while(|d| *d < today)
        .map(|d| ClimateDay {
            date: d,
            soil_temp_5_f: Some(55.0),
            air_min_f: Some(40.0),
            air_avg_f: Some(50.0),
            gdd50: Some(0.0),
        })
        .collect();
    // 2015 keeps exactly 300 soil days; 2016 only 299.
    for (year, keep) in [(2015, 300), (2016, 299)] {
        for (i, day) in days
            .iter_mut()
            .filter(|d| d.date.year() == year)
            .enumerate()
        {
            if i >= keep {
                day.soil_temp_5_f = None;
            }
        }
    }
    let start = climate_start(today).unwrap();
    let record: Vec<ClimateDay> = days.into_iter().filter(|d| d.date >= start).collect();

    let years = history_years(&record, 2026);
    let expected: Vec<i32> = (2011..2026).filter(|&y| y != 2016).collect();
    assert_eq!(years, expected);
    assert!(years.len() <= 15);
    assert!(!years.contains(&2026), "the current year never counts");
}

fn day(on: NaiveDate, air_min_f: Option<f64>) -> ClimateDay {
    ClimateDay {
        date: on,
        soil_temp_5_f: Some(50.0),
        air_min_f,
        air_avg_f: Some(50.0),
        gdd50: Some(0.0),
    }
}

/// A year at 45°F minimums, with freezes (`min` ≤ 32) on the listed days.
fn year_with_freezes(year: i32, freezes: &[(NaiveDate, f64)]) -> Vec<ClimateDay> {
    date(year, 1, 1)
        .iter_days()
        .take_while(|d| d.year() == year)
        .map(|d| {
            let min = freezes
                .iter()
                .find(|(f, _)| *f == d)
                .map_or(45.0, |(_, m)| *m);
            day(d, Some(min))
        })
        .collect()
}

// @spec TIME-ENG-016
#[test]
fn freezes_are_minimums_at_or_below_32_split_at_july() {
    let days = year_with_freezes(
        2024,
        &[
            (date(2024, 3, 2), 30.0),
            (date(2024, 4, 20), 32.0), // exactly 32 is a freeze
            (date(2024, 5, 1), 32.5),  // not a freeze
            (date(2024, 7, 15), 31.0), // July: neither a spring nor a fall freeze
            (date(2024, 10, 14), 32.0),
            (date(2024, 11, 2), 25.0),
        ],
    );
    assert_eq!(last_spring_freeze(&days, 2024), Some(date(2024, 4, 20)));
    assert_eq!(first_fall_freeze(&days, 2024), Some(date(2024, 10, 14)));

    // A freeze on Aug 1 is the first fall freeze; one on Jun 30 the last spring one.
    let edges = year_with_freezes(2024, &[(date(2024, 6, 30), 31.0), (date(2024, 8, 1), 31.0)]);
    assert_eq!(last_spring_freeze(&edges, 2024), Some(date(2024, 6, 30)));
    assert_eq!(first_fall_freeze(&edges, 2024), Some(date(2024, 8, 1)));
}

/// Remove the air minimum from `n` days starting at `from`.
fn without_air(mut days: Vec<ClimateDay>, from: NaiveDate, n: i64) -> Vec<ClimateDay> {
    for d in days
        .iter_mut()
        .filter(|d| d.date >= from && d.date < from + Duration::days(n))
    {
        d.air_min_f = None;
    }
    days
}

// @spec TIME-ENG-017
#[test]
fn a_freeze_counts_only_with_90_percent_of_its_side_of_the_season_on_record() {
    let fall = date(2024, 11, 3); // Sep 15 → Nov 3 is 50 days
    let spring = date(2024, 4, 11); // Apr 11 → May 31 is 51 days
    let days = year_with_freezes(2024, &[(spring, 30.0), (fall, 30.0)]);

    // 5 of 50 days missing before the fall freeze (90%): sound. 6 missing: not.
    let ok = without_air(days.clone(), date(2024, 10, 1), 5);
    assert_eq!(fall_freeze_history(&ok, &[2024]), vec![(2024, fall)]);
    let holed = without_air(days.clone(), date(2024, 10, 1), 6);
    assert!(fall_freeze_history(&holed, &[2024]).is_empty());
    // A hole after the fall freeze does not matter.
    let after = without_air(days.clone(), date(2024, 11, 10), 30);
    assert_eq!(fall_freeze_history(&after, &[2024]), vec![(2024, fall)]);

    // 5 of 51 days missing after the spring freeze (≥ 90%): sound. 6 missing: not.
    let ok = without_air(days.clone(), date(2024, 5, 1), 5);
    assert_eq!(spring_freeze_history(&ok, &[2024]), vec![(2024, spring)]);
    let holed = without_air(days.clone(), date(2024, 5, 1), 6);
    assert!(spring_freeze_history(&holed, &[2024]).is_empty());
    let before = without_air(days, date(2024, 2, 1), 30);
    assert_eq!(
        spring_freeze_history(&before, &[2024]),
        vec![(2024, spring)]
    );
}

// @spec TIME-ENG-018
#[test]
fn a_gdd_target_is_reached_on_the_day_the_running_total_gets_there() {
    // 5 GDD a day from Jan 1: 200 on Feb 9 (day 40).
    let mut days: Vec<ClimateDay> = date(2024, 1, 1)
        .iter_days()
        .take(60)
        .map(|d| ClimateDay {
            gdd50: Some(5.0),
            ..day(d, Some(40.0))
        })
        .collect();
    assert_eq!(gdd_reached(&days, 2024, 200.0), Some(date(2024, 2, 9)));

    // 8 of 40 elapsed days without a value (80% covered): still counts; the total
    // reaches 200 eight days later (Feb 17, 40 of 48 days = 83%).
    for d in days.iter_mut().take(8) {
        d.gdd50 = None;
    }
    assert_eq!(gdd_reached(&days, 2024, 200.0), Some(date(2024, 2, 17)));

    // 11 missing: 40 of 51 elapsed days (78%) — not trusted.
    for d in days.iter_mut().take(11) {
        d.gdd50 = None;
    }
    assert_eq!(gdd_reached(&days, 2024, 200.0), None);
}

const FALL_70: Crossing = Crossing {
    threshold_f: 70.0,
    direction: Direction::Falling,
    scan_from: SeasonDay::new(0, 8, 1),
    scan_to: SeasonDay::new(0, 12, 31),
};

// @spec TIME-ENG-019
#[test]
fn past_years_are_dated_by_their_own_crossings_freezes_and_gdd() {
    let today = date(2026, 9, 20);
    let mut days = station(date(2026, 9, 19), 0.0);
    // 2023: the sensor is down Aug 20 → Oct 15, so the run after it dates the outage.
    for d in days
        .iter_mut()
        .filter(|d| d.date >= date(2023, 8, 20) && d.date <= date(2023, 10, 15))
    {
        d.soil_temp_5_f = None;
    }
    let data = SeasonData::build(&days, &[], 2026);
    let freezes = vec![(2021, date(2021, 11, 10)), (2022, date(2022, 10, 30))];
    let season = Season {
        fall_freezes: &freezes,
        ..data.season(&days, today)
    };

    let soil = typical(&Boundary::Soil(FALL_70), &season, 2026).unwrap();
    // 2020–2025 minus the outage year.
    assert_eq!(soil.sample_count, 5);

    let freeze = typical(&Boundary::BeforeFirstFreeze(30), &season, 2026).unwrap();
    assert_eq!(freeze.sample_count, 2);
    assert_eq!(freeze.earliest, date(2026, 9, 30)); // Oct 30 − 30 d
    assert_eq!(freeze.latest, date(2026, 10, 11)); // Nov 10 − 30 d

    let gdd = typical(&Boundary::Gdd(200.0), &season, 2026).unwrap();
    assert_eq!(gdd.sample_count, 6);

    // Earliest-of: only the years with every option on record, at the earlier date.
    let composite = Boundary::Earliest(vec![
        Boundary::BeforeFirstFreeze(30),
        Boundary::Soil(FALL_70),
    ]);
    let both = typical(&composite, &season, 2026).unwrap();
    assert_eq!(both.sample_count, 2);
    assert!(
        both.latest < date(2026, 9, 30),
        "soil (≈Sep 7) wins each year"
    );
}

// @spec TIME-ENG-020
#[test]
fn a_boundarys_past_dates_summarize_as_median_spread_and_confidence() {
    let on = |y, m, d| (y, date(y, m, d));
    // Even count: the two middle offsets (Sep 14, Sep 17) average down to Sep 15.
    let even = [
        on(2020, 9, 10),
        on(2021, 9, 14),
        on(2022, 9, 17),
        on(2023, 9, 30),
    ];
    let stat = date_stat(&even, 2026).unwrap();
    assert_eq!(stat.median, date(2026, 9, 15));
    assert_eq!(stat.confidence, WindowConfidence::Medium);

    // Ten years, Sep 1 … Sep 10 (leap years shifted to keep their offsets aligned).
    let ten: Vec<(i32, NaiveDate)> = (0..10)
        .map(|i| {
            let year = 2013 + i;
            let anchor = date(year, 1, 1);
            (year, anchor + Duration::days(243 + i as i64)) // offset 243 = Sep 1, 2026
        })
        .collect();
    let stat = date_stat(&ten, 2026).unwrap();
    assert_eq!(stat.earliest, date(2026, 9, 1));
    assert_eq!(stat.p10, date(2026, 9, 1)); // rank ⌈1.0⌉ = 1
    assert_eq!(stat.median, date(2026, 9, 5)); // (Sep 5 + Sep 6) / 2, rounded down
    assert_eq!(stat.p90, date(2026, 9, 9)); // rank ⌈9.0⌉ = 9
    assert_eq!(stat.latest, date(2026, 9, 10));
    assert_eq!(stat.sample_count, 10);
    assert_eq!(stat.confidence, WindowConfidence::High);

    let confidence = |n: usize| date_stat(&ten[..n], 2026).unwrap().confidence;
    assert_eq!(confidence(8), WindowConfidence::High);
    assert_eq!(confidence(7), WindowConfidence::Medium);
    assert_eq!(confidence(4), WindowConfidence::Medium);
    assert_eq!(confidence(3), WindowConfidence::Low);

    // Offsets run from Jan 1 of each event's season year: a January date in the next
    // calendar year sorts after December.
    let spill = [on(2022, 12, 20), (2022, date(2023, 1, 10)), on(2021, 12, 1)];
    let stat = date_stat(&spill, 2026).unwrap();
    assert_eq!(stat.latest, date(2027, 1, 10));
    assert_eq!(stat.median, date(2026, 12, 20));
}
