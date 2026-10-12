//! The regression's predictions, its purity and its confidence / quality labels.

use super::*;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 4, day).unwrap()
}

fn model(slope: f64, lag_days: u32, r_squared: f64) -> SoilTempModel {
    SoilTempModel {
        slope,
        intercept: 0.0,
        lag_days,
        r_squared,
        training_window_days: 30,
        fitted_at: Utc::now(),
    }
}

// @spec WX-OUTLOOK-MODEL-002
#[test]
fn a_day_without_a_lagged_driver_is_skipped_and_the_rest_keep_outlook_order() {
    let history = [(d(2), 48.0), (d(3), 51.0)];
    // Out of date order on purpose; Apr 9's driver (Apr 7) is nowhere.
    let outlook = [(d(5), 60.0), (d(4), 55.0), (d(9), 70.0), (d(6), 57.0)];
    let preds = predict_soil_temps(&model(0.333, 2, 0.9), &history, &outlook, d(3));

    let dates: Vec<NaiveDate> = preds.iter().map(|p| p.date).collect();
    assert_eq!(dates, vec![d(5), d(4), d(6)]);
    // Apr 5 ← Apr 3 (history), Apr 4 ← Apr 2 (history), Apr 6 ← Apr 4 (outlook).
    let used: Vec<f64> = preds.iter().map(|p| p.air_temp_used_f).collect();
    assert_eq!(used, vec![51.0, 48.0, 55.0]);
    // 0.333 × 51 = 16.983 → 17.0; 0.333 × 48 = 15.984 → 16.0; 0.333 × 55 = 18.315 → 18.3
    let values: Vec<f64> = preds.iter().map(|p| p.predicted_soil_temp_f).collect();
    assert_eq!(values, vec![17.0, 16.0, 18.3]);
}

// @spec WX-OUTLOOK-MODEL-003
#[test]
fn the_regression_depends_only_on_the_series_it_is_given() {
    // The same functions serve any depth: here a 5 cm-like series that tracks air
    // closely and a 10 cm-like series that lags it a day.
    let air = |i: i64| 50.0 + 8.0 * ((i as f64) * 0.7).sin();
    let day = |i: i64| d(1) + chrono::Duration::days(i);
    let five_cm: Vec<(NaiveDate, f64, f64)> =
        (0..20).map(|i| (day(i), air(i), air(i) + 1.0)).collect();
    let ten_cm: Vec<(NaiveDate, f64, f64)> = (0..20)
        .map(|i| (day(i), air(i), 0.5 * air(i - 1) + 25.0))
        .collect();

    let a = fit_model(&five_cm).unwrap();
    let b = fit_model(&five_cm).unwrap();
    assert_eq!(
        (a.slope, a.intercept, a.lag_days),
        (b.slope, b.intercept, b.lag_days)
    );
    assert_eq!(a.lag_days, 0);
    assert_eq!(fit_model(&ten_cm).unwrap().lag_days, 1);

    let outlook = [(day(20), 55.0), (day(21), 58.0)];
    let history: Vec<(NaiveDate, f64)> = five_cm.iter().map(|(d, a, _)| (*d, *a)).collect();
    let first = predict_soil_temps(&a, &history, &outlook, day(19));
    let again = predict_soil_temps(&a, &history, &outlook, day(19));
    let values = |p: &[SoilTempPrediction]| -> Vec<f64> {
        p.iter().map(|p| p.predicted_soil_temp_f).collect()
    };
    assert_eq!(values(&first), values(&again));
    assert_eq!(values(&first), vec![56.0, 59.0]);
}

// @spec WX-OUTLOOK-MODEL-006
#[test]
fn confidence_falls_with_fit_and_days_out() {
    use PredictionConfidence::*;
    assert_eq!(compute_confidence(0.7, 2), High);
    assert_eq!(compute_confidence(0.7, 3), Medium);
    assert_eq!(compute_confidence(0.69, 1), Medium);
    assert_eq!(compute_confidence(0.5, 3), Medium);
    assert_eq!(compute_confidence(0.5, 4), Low);
    assert_eq!(compute_confidence(0.49, 0), Low);
}

// @spec WX-OUTLOOK-MODEL-006
#[test]
fn the_model_is_labelled_good_fair_or_poor() {
    let quality = |r2| SoilTempModelInfo::from(&model(1.0, 0, r2)).quality;
    assert_eq!(quality(0.7), "Good");
    assert_eq!(quality(0.69), "Fair");
    assert_eq!(quality(0.5), "Fair");
    assert_eq!(quality(0.49), "Poor");
}
