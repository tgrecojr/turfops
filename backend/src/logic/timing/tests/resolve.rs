//! Resolving a boundary for the current season: observed, tentative, forecast, or its
//! typical date — and when station data is fresh enough to judge.

use super::*;
use crate::logic::timing::evaluate::{resolve, typical, Resolved};
use crate::logic::timing::series::{Crossing, Direction, SeasonDay};
use crate::logic::timing::windows::Boundary;

const FALL_70: Boundary = Boundary::Soil(Crossing {
    threshold_f: 70.0,
    direction: Direction::Falling,
    scan_from: SeasonDay::new(0, 8, 1),
    scan_to: SeasonDay::new(0, 12, 31),
});

const SPRING_55: Boundary = Boundary::Soil(Crossing {
    threshold_f: 55.0,
    direction: Direction::Rising,
    scan_from: SeasonDay::new(0, 2, 1),
    scan_to: SeasonDay::new(0, 7, 31),
});

/// Resolve `boundary` for 2026 against the synthetic station through `through`
/// (2026 running `bias` °F off the curve), as seen on `today`.
fn resolved(boundary: &Boundary, through: NaiveDate, bias: f64, today: NaiveDate) -> Resolved {
    let days = station(through, bias);
    let data = SeasonData::build(&days, &[], 2026);
    resolve(boundary, &data.season(&days, today), 2026)
}

fn median(boundary: &Boundary) -> NaiveDate {
    let days = station(date(2026, 9, 19), 0.0);
    let data = SeasonData::build(&days, &[], 2026);
    typical(boundary, &data.season(&days, date(2026, 9, 20)), 2026)
        .unwrap()
        .median
}

// @spec TIME-ENG-021
#[test]
fn data_is_fresh_up_to_five_days_after_the_last_soil_day() {
    let days = station(date(2026, 9, 10), 0.0);
    let data = SeasonData::build(&days, &[], 2026);
    assert!(data.season(&days, date(2026, 9, 15)).data_fresh);
    assert!(!data.season(&days, date(2026, 9, 16)).data_fresh);

    let no_soil: Vec<ClimateDay> = days
        .into_iter()
        .map(|d| ClimateDay {
            soil_temp_5_f: None,
            ..d
        })
        .collect();
    let data = SeasonData::build(&no_soil, &[], 2026);
    assert!(!data.season(&no_soil, date(2026, 9, 11)).data_fresh);
}

// @spec TIME-ENG-022
#[test]
fn a_held_crossing_is_observed_and_passed() {
    let r = resolved(&FALL_70, date(2026, 9, 19), 0.0, date(2026, 9, 20));
    assert_eq!(r.source, DateSource::Observed);
    assert!(r.passed);
    assert!(within(r.date, date(2026, 9, 7), 2));
    assert_eq!(r.days_held, None);
}

// @spec TIME-ENG-023
#[test]
fn a_crossing_not_yet_held_is_tentative_with_its_days_held() {
    let last = date(2026, 9, 9);
    let r = resolved(&FALL_70, last, 0.0, date(2026, 9, 10));
    assert_eq!(r.source, DateSource::Tentative);
    assert!(r.passed);
    let since = r.date.unwrap();
    let held = (last - since).num_days() as u32 + 1;
    assert!(held < 5);
    assert_eq!(r.days_held, Some(held));
}

// @spec TIME-ENG-024
#[test]
fn a_crossing_seen_only_in_the_forecast_is_dated_no_earlier_than_today() {
    let days = station(date(2026, 9, 2), 0.0);
    let data = SeasonData::build(&days, &[], 2026);
    let mut smoothed = data.smoothed.clone();
    smoothed.extend((3..=6).map(|day| SoilPoint {
        date: date(2026, 9, day),
        temp_f: 65.0,
        is_forecast: true,
    }));
    let today = date(2026, 9, 4);
    let season = Season {
        smoothed: &smoothed,
        ..data.season(&days, today)
    };
    let r = resolve(&FALL_70, &season, 2026);
    assert_eq!(r.source, DateSource::Forecast);
    assert!(!r.passed);
    assert_eq!(r.date, Some(today), "the run starts Sep 3, before today");
}

// @spec TIME-ENG-025
#[test]
fn stale_data_trusts_only_a_confirmed_crossing() {
    // Data ends Sep 15, ten days ago: the crossing was held before it stopped.
    let held = resolved(&FALL_70, date(2026, 9, 15), 0.0, date(2026, 9, 25));
    assert_eq!(held.source, DateSource::Observed);

    // Data ends Sep 9 mid-run: nobody knows whether it held — the calendar decides.
    let in_progress = resolved(&FALL_70, date(2026, 9, 9), 0.0, date(2026, 9, 20));
    assert_eq!(in_progress.source, DateSource::Typical);
    assert_eq!(in_progress.date, Some(median(&FALL_70)));
}

// @spec TIME-ENG-026
#[test]
fn a_freeze_buffer_resolves_on_the_typical_freeze_only() {
    let boundary = Boundary::BeforeFirstFreeze(30);
    let expected = median(&boundary);
    assert!(within(Some(expected), date(2026, 10, 6), 2));

    // This year freezes on Sep 12 — that does not move the current season's date.
    let mut days = station(date(2026, 9, 19), 0.0);
    for d in days.iter_mut().filter(|d| d.date == date(2026, 9, 12)) {
        d.air_min_f = Some(25.0);
    }
    let data = SeasonData::build(&days, &[], 2026);
    let before = resolve(&boundary, &data.season(&days, date(2026, 9, 20)), 2026);
    assert_eq!(before.source, DateSource::Typical);
    assert_eq!(before.date, Some(expected));
    assert!(!before.passed);

    let on_the_day = resolve(&boundary, &data.season(&days, expected), 2026);
    assert!(on_the_day.passed);
    let day_before = resolve(
        &boundary,
        &data.season(&days, expected - Duration::days(1)),
        2026,
    );
    assert!(!day_before.passed);
}

// @spec TIME-ENG-027
#[test]
fn a_gdd_boundary_is_observed_once_reached_otherwise_typical() {
    let gdd = Boundary::Gdd(200.0);
    let reached = resolved(&gdd, date(2026, 8, 31), 0.0, date(2026, 9, 1));
    assert_eq!(reached.source, DateSource::Observed);
    assert!(reached.passed);

    let early = resolved(&gdd, date(2026, 3, 31), 0.0, date(2026, 4, 1));
    assert_eq!(early.source, DateSource::Typical);
    assert_eq!(early.date, Some(median(&gdd)));
    assert!(!early.passed);
}

// @spec TIME-ENG-029
#[test]
fn a_typical_date_stays_an_estimate_until_the_station_reports_past_it() {
    let m = median(&FALL_70);
    // 2026 runs 10°F warm: no crossing anywhere near the typical date.
    let ahead = resolved(&FALL_70, date(2026, 8, 19), 10.0, date(2026, 8, 20));
    assert_eq!((ahead.date, ahead.passed), (Some(m), false));
    assert_eq!(ahead.source, DateSource::Typical);

    // Past the median, but the fresh station record stops short of it.
    let lagging = resolved(&FALL_70, m - Duration::days(1), 10.0, m + Duration::days(2));
    assert_eq!((lagging.date, lagging.passed), (Some(m), false));
}

// @spec TIME-ENG-030
#[test]
fn a_typical_date_is_passed_once_the_scan_has_ended_or_the_data_is_stale() {
    // A spring so cold that soil never holds 55°F before the scan ends on Jul 31.
    let m = median(&SPRING_55);
    let scan_over = resolved(&SPRING_55, date(2026, 8, 9), -25.0, date(2026, 8, 10));
    assert_eq!((scan_over.date, scan_over.passed), (Some(m), true));

    let stale = resolved(&FALL_70, date(2026, 9, 1), 10.0, date(2026, 9, 20));
    assert_eq!((stale.date, stale.passed), (Some(median(&FALL_70)), true));
}

// @spec TIME-ENG-031
#[test]
fn a_typical_date_the_station_has_reported_past_is_overdue() {
    let m = median(&FALL_70);
    let today = m + Duration::days(3);
    let overdue = resolved(&FALL_70, today - Duration::days(1), 10.0, today);
    assert_eq!(overdue.date, None);
    assert!(!overdue.passed);
    assert_eq!(overdue.source, DateSource::Typical);
}

// @spec TIME-ENG-032
#[test]
fn a_boundary_with_no_history_is_undated() {
    // Only this year on record.
    let days: Vec<ClimateDay> = station(date(2026, 9, 19), 10.0)
        .into_iter()
        .filter(|d| d.date.year() == 2026)
        .collect();
    let data = SeasonData::build(&days, &[], 2026);
    assert!(data.history_years.is_empty());
    let season = data.season(&days, date(2026, 9, 20));
    for boundary in [
        FALL_70,
        Boundary::BeforeFirstFreeze(30),
        Boundary::Gdd(5000.0),
    ] {
        let r = resolve(&boundary, &season, 2026);
        assert_eq!((r.date, r.passed), (None, false), "{boundary:?}");
    }
}
