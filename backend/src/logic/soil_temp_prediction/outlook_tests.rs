//! The regression's date alignment, the continuous anchored outlook, and crossings past
//! the forecast.

use super::*;
use chrono::Duration;

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

/// Air that climbs and dips, with soil = 0.8 × the air of two calendar days earlier + 10.
fn lagged_pairs(skip: &[NaiveDate]) -> Vec<(NaiveDate, f64, f64)> {
    let air = |date: NaiveDate| 50.0 + 8.0 * ((date - d(3, 1)).num_days() as f64 / 3.0).sin();
    (0..30)
        .map(|i| d(3, 1) + Duration::days(i))
        .filter(|date| !skip.contains(date))
        .map(|date| (date, air(date), 0.8 * air(date - Duration::days(2)) + 10.0))
        .collect()
}

// @spec WX-OUTLOOK-MODEL-001
#[test]
fn the_lag_pairs_days_by_date_not_by_position() {
    // Three missing days shift every later row's position; pairing by position would
    // read the wrong air for most of the month.
    let pairs = lagged_pairs(&[d(3, 10), d(3, 11), d(3, 19)]);
    let model = fit_model(&pairs).expect("a model");
    assert_eq!(model.lag_days, 2);
    assert!(model.r_squared > 0.99, "R² {}", model.r_squared);
    assert!((model.slope - 0.8).abs() < 1e-6, "slope {}", model.slope);
}

// @spec WX-OUTLOOK-MODEL-001
#[test]
fn ten_date_aligned_pairs_are_enough() {
    let pairs: Vec<_> = lagged_pairs(&[]).into_iter().take(12).collect();
    // 12 days at lag 2 give 10 aligned pairs.
    assert!(fit_model(&pairs).is_some());
}

// @spec WX-OUTLOOK-MODEL-008
#[test]
fn the_outlook_reads_through_a_gap_between_station_and_forecast_air() {
    let pairs = lagged_pairs(&[]); // through Mar 30
                                   // The forecast starts Apr 2: Mar 31 and Apr 1 are a hole in the air series.
    let forecast = [(d(4, 2), 60.0), (d(4, 3), 62.0), (d(4, 4), 63.0)];
    let outlook = build_forecast(&pairs, &forecast, 55.0, d(4, 1)).expect("an outlook");
    let dates: Vec<NaiveDate> = outlook.predictions.iter().map(|p| p.date).collect();
    assert_eq!(
        dates.first(),
        Some(&d(3, 31)),
        "every day after the last observation"
    );
    assert!(
        dates.contains(&d(4, 2)),
        "Apr 2 is driven by interpolated Mar 31 air"
    );
    assert_eq!(dates.last(), Some(&d(4, 4)));
}

// @spec WX-OUTLOOK-MODEL-008
#[test]
fn the_outlook_continues_from_the_last_measured_soil() {
    let mut pairs = lagged_pairs(&[]);
    let model = fit_model(&pairs).unwrap();
    // The station read 3 °F warmer than the fit on the last day.
    let last = pairs.last_mut().unwrap();
    last.2 += 3.0;
    let observed_last = last.2;
    let forecast = [(d(4, 1), 60.0), (d(4, 2), 60.0)];
    let outlook = build_forecast(&pairs, &forecast, observed_last, d(3, 31)).unwrap();
    let fitted_mar31 = model.slope * pairs[pairs.len() - 2].1 + model.intercept;
    let first = &outlook.predictions[0];
    assert_eq!(first.date, d(3, 31));
    assert!(
        (first.predicted_soil_temp_f - (fitted_mar31 + 3.0)).abs() < 0.2,
        "shifted by the error on the last observed day: {} vs {}",
        first.predicted_soil_temp_f,
        fitted_mar31 + 3.0
    );
}

// @spec WX-OUTLOOK-MODEL-008
#[test]
fn short_air_gaps_are_filled_and_long_ones_are_not() {
    let observed = [(d(3, 1), 40.0), (d(3, 5), 48.0)];
    let air = continuous_air(&observed, &[(d(3, 20), 60.0)]);
    let filled: Vec<NaiveDate> = air.iter().map(|(date, _)| *date).collect();
    assert!(filled.contains(&d(3, 3)), "a 4-day gap is interpolated");
    assert_eq!(
        air.iter().find(|(date, _)| *date == d(3, 3)).unwrap().1,
        44.0
    );
    assert!(!filled.contains(&d(3, 10)), "a 15-day gap is not");
    let station_wins = continuous_air(&[(d(3, 1), 40.0)], &[(d(3, 1), 70.0)]);
    assert_eq!(station_wins, vec![(d(3, 1), 40.0)]);
}

// @spec WX-OUTLOOK-CROSS-005
#[test]
fn an_approach_past_the_forecast_gets_no_date() {
    let p = |date: NaiveDate, temp: f64| SoilTempPrediction {
        date,
        predicted_soil_temp_f: temp,
        confidence: PredictionConfidence::Low,
        air_temp_used_f: temp,
        source_description: String::new(),
    };
    // Soil at 56 °F creeping up 0.5 °F over three days: 60 °F is weeks away.
    let preds = [p(d(5, 2), 56.2), p(d(5, 3), 56.4), p(d(5, 4), 56.5)];
    let crossings = predict_threshold_crossings(56.0, &preds, d(5, 1));
    let grub = crossings
        .iter()
        .find(|c| c.threshold_temp_f == 60.0)
        .expect("approaching");
    assert_eq!(grub.estimated_crossing_date, None);
    assert!(grub.beyond_forecast);
}
