//! The display window, the headline day and trailing means.

use super::test_support::{date, day, run_of};
use super::*;
use crate::models::Disease;

fn percent_scale() -> RiskScale {
    RiskScale {
        min: 0.0,
        max: 100.0,
        unit: "%".into(),
        moderate_at: 25.0,
        high_at: 50.0,
        severe_at: 75.0,
    }
}

// @spec DIS-MODEL-DAY-008
#[test]
fn twelve_days_are_fetched_and_six_back_to_four_ahead_are_shown() {
    assert_eq!(LOOKBACK_DAYS, 12);
    assert_eq!(HISTORY_DAYS, 6);
    assert_eq!(FORECAST_DAYS, 4);

    // Twelve observed days before today, today, and six forecast days.
    let today = date(7, 20);
    let mut series = run_of(today + Duration::days(6), 19, |d| {
        day(d, 24.0, 19.0, 29.0, 85.0)
    });
    for d in series.iter_mut().filter(|d| d.date > today) {
        d.is_forecast = true;
    }
    let scored = score_window(&series, today, &percent_scale(), |_| Some(10.0)).unwrap();
    let shown: Vec<NaiveDate> = scored.daily.iter().map(|d| d.date).collect();
    let expected: Vec<NaiveDate> = (-6..=4).map(|n| today + Duration::days(n)).collect();
    assert_eq!(shown, expected);

    // The lookback leaves a full 5-day trailing window on the first displayed day.
    let risks = assess_all(&series, today, &DiseaseContext::default());
    let dollar_spot = risks
        .iter()
        .find(|r| r.disease == Disease::DollarSpot)
        .unwrap();
    assert_eq!(
        dollar_spot.daily[0].date,
        today - Duration::days(HISTORY_DAYS)
    );
}

// @spec DIS-MODEL-DAY-009
#[test]
fn a_trailing_mean_needs_the_models_minimum_of_usable_days() {
    let today = date(9, 20);
    let mut series = run_of(today, 5, |d| day(d, 20.0, 15.0, 25.0, 80.0));
    // Two partial days in the window: three usable days remain (today counts regardless).
    series[1].hours_covered = 4.0;
    series[2].hours_covered = 4.0;
    series[1].temp_mean_c = 99.0;
    series[2].temp_mean_c = 99.0;
    assert_eq!(
        trailing_mean(&series, 4, 5, 3, |d| d.temp_mean_c),
        Some(20.0)
    );
    assert_eq!(trailing_mean(&series, 4, 5, 4, |d| d.temp_mean_c), None);

    // The scored day itself counts even when partial.
    series[4].hours_covered = 4.0;
    series[4].temp_mean_c = 26.0;
    assert_eq!(
        trailing_mean(&series, 4, 5, 3, |d| d.temp_mean_c),
        Some((20.0 + 20.0 + 26.0) / 3.0)
    );

    // Days before the window never count.
    let long = run_of(today, 10, |d| day(d, 20.0, 15.0, 25.0, 80.0));
    assert_eq!(trailing_mean(&long, 9, 3, 4, |d| d.temp_mean_c), None);
}

// @spec DIS-MODEL-HEAD-001
#[test]
fn every_scored_day_records_value_tier_source_and_partiality() {
    let today = date(7, 20);
    let mut series = run_of(today + Duration::days(2), 5, |d| {
        day(d, 24.0, 19.0, 29.0, 85.0)
    });
    series[3].is_forecast = true;
    series[4].is_forecast = true;
    series[4].hours_covered = 6.0;
    // Day 1 has no value (e.g. too few days for its trailing window): it is not drawn.
    let values = [Some(10.0), None, Some(30.0), Some(60.0), Some(80.0)];
    let scored = score_window(&series, today, &percent_scale(), |i| values[i]).unwrap();

    assert_eq!(scored.daily.len(), 4);
    let first = &scored.daily[0];
    assert_eq!((first.value, first.tier), (10.0, RiskTier::Low));
    assert!(!first.is_forecast && !first.partial);
    assert_eq!(scored.daily[1].tier, RiskTier::Moderate);
    assert_eq!(scored.daily[2].tier, RiskTier::High);
    assert!(scored.daily[2].is_forecast && !scored.daily[2].partial);
    assert_eq!(scored.daily[3].tier, RiskTier::Severe);
    assert!(scored.daily[3].is_forecast && scored.daily[3].partial);
}

// @spec DIS-MODEL-HEAD-002
#[test]
fn the_headline_is_the_latest_usable_day_no_older_than_the_history() {
    let today = date(7, 20);
    let mut series = run_of(today + Duration::days(1), 9, |d| {
        day(d, 24.0, 19.0, 29.0, 85.0)
    });
    // Forecast tomorrow is usable but is never the headline; today is partial.
    series[8].is_forecast = true;
    series[7].hours_covered = 5.0;
    let scored = score_window(&series, today, &percent_scale(), |i| Some(i as f64)).unwrap();
    assert_eq!(series[scored.headline_idx].date, today - Duration::days(1));

    // Only the first displayed day (today - 6) is usable: it is the headline and the as-of.
    let mut sparse = run_of(today, 7, |d| day(d, 26.0, 21.0, 32.0, 90.0));
    for d in sparse.iter_mut().skip(1) {
        d.hours_covered = 5.0;
    }
    let risks = assess_all(&sparse, today, &DiseaseContext::default());
    let brown_patch = risks
        .iter()
        .find(|r| r.disease == Disease::BrownPatch)
        .unwrap();
    assert_eq!(brown_patch.as_of, today - Duration::days(HISTORY_DAYS));
}

// @spec DIS-MODEL-HEAD-003
#[test]
fn a_disease_with_no_usable_day_in_the_history_is_left_out() {
    let today = date(7, 20);
    // A usable day a week ago (outside the 6-day history), only partial days since.
    let mut series = run_of(today, 8, |d| day(d, 26.0, 21.0, 32.0, 90.0));
    for d in series.iter_mut().skip(1) {
        d.hours_covered = 5.0;
    }
    assert_eq!(series[0].date, today - Duration::days(7));
    let scored = score_window(&series, today, &percent_scale(), |_| Some(50.0));
    assert!(scored.is_none());
    assert!(assess_all(&series, today, &DiseaseContext::default()).is_empty());
}
