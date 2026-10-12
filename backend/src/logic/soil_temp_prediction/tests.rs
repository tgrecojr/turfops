use super::*;
use chrono::NaiveDate;

#[test]
fn linear_regression_perfect_fit() {
    let xs = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let ys = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
    let (slope, intercept, r2) = linear_regression(&xs, &ys).unwrap();
    assert!((slope - 2.0).abs() < 0.001);
    assert!(intercept.abs() < 0.001);
    assert!((r2 - 1.0).abs() < 0.001);
}

#[test]
fn linear_regression_with_intercept() {
    let xs = vec![0.0, 1.0, 2.0, 3.0];
    let ys = vec![5.0, 7.0, 9.0, 11.0]; // y = 2x + 5
    let (slope, intercept, r2) = linear_regression(&xs, &ys).unwrap();
    assert!((slope - 2.0).abs() < 0.001);
    assert!((intercept - 5.0).abs() < 0.001);
    assert!((r2 - 1.0).abs() < 0.001);
}

#[test]
fn linear_regression_too_few_points() {
    let xs = vec![1.0];
    let ys = vec![2.0];
    assert!(linear_regression(&xs, &ys).is_none());
}

#[test]
fn linear_regression_mismatched_lengths() {
    let xs = vec![1.0, 2.0];
    let ys = vec![1.0];
    assert!(linear_regression(&xs, &ys).is_none());
}

// @spec WX-OUTLOOK-MODEL-001
#[test]
fn fit_model_insufficient_data() {
    // Nine days: fewer than 10 aligned pairs at every lag.
    let pairs: Vec<(NaiveDate, f64, f64)> = (0..9)
        .map(|i| {
            let d = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap() + chrono::Duration::days(i);
            (d, 50.0 + i as f64, 48.0 + i as f64)
        })
        .collect();
    assert!(fit_model(&pairs).is_none());
}

#[test]
fn fit_model_selects_best_lag() {
    // Create data where soil temp follows air temp with a 2-day lag
    let mut pairs: Vec<(NaiveDate, f64, f64)> = Vec::new();
    for i in 0..30 {
        let date = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap() + chrono::Duration::days(i);
        let air_temp = 40.0 + (i as f64) * 0.5;
        // Soil temp follows air temp from 2 days ago (approx)
        let soil_temp = if i >= 2 {
            40.0 + ((i - 2) as f64) * 0.5 + 2.0
        } else {
            42.0
        };
        pairs.push((date, air_temp, soil_temp));
    }

    let model = fit_model(&pairs).unwrap();
    assert!(model.r_squared > 0.8, "r2={}", model.r_squared);
    // The best lag should be around 2 (might be 1 or 3 due to noise)
    assert!(model.lag_days <= 4, "lag={}", model.lag_days);
}

#[test]
fn fit_model_rejects_poor_fit() {
    // Random-ish data with no correlation
    let pairs: Vec<(NaiveDate, f64, f64)> = (0..30)
        .map(|i| {
            let d = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap() + chrono::Duration::days(i);
            let air = if i % 2 == 0 { 80.0 } else { 30.0 };
            let soil = if i % 3 == 0 { 70.0 } else { 35.0 };
            (d, air, soil)
        })
        .collect();
    // Should return None due to poor R-squared
    assert!(fit_model(&pairs).is_none());
}

#[test]
fn predict_soil_temps_basic() {
    let model = SoilTempModel {
        slope: 0.8,
        intercept: 10.0,
        lag_days: 0,
        r_squared: 0.9,
        training_window_days: 30,
        fitted_at: Utc::now(),
    };

    let recent: Vec<(NaiveDate, f64)> = vec![];
    let forecast: Vec<(NaiveDate, f64)> = vec![
        (NaiveDate::from_ymd_opt(2026, 3, 17).unwrap(), 50.0),
        (NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(), 55.0),
    ];

    let preds = predict_soil_temps(
        &model,
        &recent,
        &forecast,
        NaiveDate::from_ymd_opt(2026, 3, 16).unwrap(),
    );
    assert_eq!(preds.len(), 2);
    // 0.8 * 50 + 10 = 50
    assert!((preds[0].predicted_soil_temp_f - 50.0).abs() < 0.1);
    // 0.8 * 55 + 10 = 54
    assert!((preds[1].predicted_soil_temp_f - 54.0).abs() < 0.1);
}

#[test]
fn predict_soil_temps_with_lag() {
    let model = SoilTempModel {
        slope: 0.8,
        intercept: 10.0,
        lag_days: 2,
        r_squared: 0.85,
        training_window_days: 30,
        fitted_at: Utc::now(),
    };

    let recent: Vec<(NaiveDate, f64)> = vec![
        (NaiveDate::from_ymd_opt(2026, 3, 14).unwrap(), 45.0),
        (NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(), 48.0),
        (NaiveDate::from_ymd_opt(2026, 3, 16).unwrap(), 50.0),
    ];
    let forecast: Vec<(NaiveDate, f64)> = vec![
        (NaiveDate::from_ymd_opt(2026, 3, 17).unwrap(), 55.0),
        (NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(), 58.0),
    ];

    let preds = predict_soil_temps(
        &model,
        &recent,
        &forecast,
        NaiveDate::from_ymd_opt(2026, 3, 16).unwrap(),
    );
    // 3/17 prediction uses air from 3/15 (lag=2): 0.8*48 + 10 = 48.4
    assert!(!preds.is_empty());
    assert!((preds[0].predicted_soil_temp_f - 48.4).abs() < 0.1);
}

#[test]
fn threshold_crossing_rising() {
    let predictions = vec![
        SoilTempPrediction {
            date: NaiveDate::from_ymd_opt(2026, 3, 17).unwrap(),
            predicted_soil_temp_f: 48.0,
            confidence: PredictionConfidence::High,
            air_temp_used_f: 50.0,
            source_description: "test".to_string(),
        },
        SoilTempPrediction {
            date: NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(),
            predicted_soil_temp_f: 51.0,
            confidence: PredictionConfidence::Medium,
            air_temp_used_f: 55.0,
            source_description: "test".to_string(),
        },
    ];

    let today = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap();
    let crossings = predict_threshold_crossings(46.0, &predictions, today);

    // Pre-emergent / crabgrass are the timing windows' business, not this model's.
    assert!(crossings.iter().all(|c| c.threshold_temp_f != 50.0));
    assert!(crossings.iter().all(|c| c.threshold_temp_f != 55.0));
}

// @spec WX-OUTLOOK-CROSS-001, WX-OUTLOOK-CROSS-002
#[test]
fn thresholds_are_only_announced_in_their_direction() {
    let day = |d: u32, temp: f64| SoilTempPrediction {
        date: NaiveDate::from_ymd_opt(2026, 10, d).unwrap(),
        predicted_soil_temp_f: temp,
        confidence: PredictionConfidence::High,
        air_temp_used_f: temp - 5.0,
        source_description: "test".to_string(),
    };
    let today = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();

    // Cooling through 60°F in October is not a grub-control window.
    let cooling = predict_threshold_crossings(62.0, &[day(2, 61.0), day(3, 58.0)], today);
    assert!(cooling
        .iter()
        .all(|c| c.threshold_name != "Grub Control Window"));

    // Warming through it is.
    let warming = predict_threshold_crossings(58.0, &[day(2, 59.0), day(3, 61.0)], today);
    let grub = warming
        .iter()
        .find(|c| c.threshold_name == "Grub Control Window")
        .expect("rising through 60°F");
    assert_eq!(grub.direction, CrossingDirection::Rising);
}

#[test]
fn threshold_crossing_falling() {
    let predictions = vec![
        SoilTempPrediction {
            date: NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            predicted_soil_temp_f: 63.0,
            confidence: PredictionConfidence::High,
            air_temp_used_f: 55.0,
            source_description: "test".to_string(),
        },
        SoilTempPrediction {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            predicted_soil_temp_f: 59.0,
            confidence: PredictionConfidence::Medium,
            air_temp_used_f: 50.0,
            source_description: "test".to_string(),
        },
    ];

    let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
    let crossings = predict_threshold_crossings(66.0, &predictions, today);

    let grub = crossings.iter().find(|c| c.threshold_temp_f == 65.0);
    assert!(grub.is_some());
    assert_eq!(grub.unwrap().direction, CrossingDirection::Falling);
}

#[test]
fn no_crossings_when_stable() {
    let predictions = vec![SoilTempPrediction {
        date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        predicted_soil_temp_f: 70.0,
        confidence: PredictionConfidence::High,
        air_temp_used_f: 80.0,
        source_description: "test".to_string(),
    }];

    let today = NaiveDate::from_ymd_opt(2026, 6, 14).unwrap();
    let crossings = predict_threshold_crossings(68.0, &predictions, today);

    // At 68-70°F, no thresholds are crossed
    let exact_crossings: Vec<_> = crossings
        .iter()
        .filter(|c| c.confidence != PredictionConfidence::Low)
        .collect();
    assert!(exact_crossings.is_empty());
}

// @spec WX-OUTLOOK-MODEL-006
#[test]
fn confidence_levels() {
    assert_eq!(compute_confidence(0.9, 1), PredictionConfidence::High);
    assert_eq!(compute_confidence(0.9, 3), PredictionConfidence::Medium);
    assert_eq!(compute_confidence(0.9, 5), PredictionConfidence::Low);
    assert_eq!(compute_confidence(0.4, 1), PredictionConfidence::Low);
    assert_eq!(compute_confidence(0.6, 2), PredictionConfidence::Medium);
}

#[test]
fn build_forecast_returns_none_insufficient_data() {
    let pairs: Vec<(NaiveDate, f64, f64)> = vec![];
    let forecast: Vec<(NaiveDate, f64)> = vec![];
    assert!(build_forecast(
        &pairs,
        &forecast,
        50.0,
        NaiveDate::from_ymd_opt(2026, 3, 16).unwrap()
    )
    .is_none());
}

// @spec WX-OUTLOOK-MODEL-007
#[test]
fn confidence_counts_days_out_from_the_given_today() {
    let model = SoilTempModel {
        slope: 1.0,
        intercept: 0.0,
        lag_days: 0,
        r_squared: 0.9,
        training_window_days: 30,
        fitted_at: Utc::now(),
    };
    let d = |day| NaiveDate::from_ymd_opt(2025, 6, day).unwrap();
    let forecast = vec![(d(2), 70.0), (d(5), 70.0)];
    let preds = predict_soil_temps(&model, &[], &forecast, d(1));
    assert_eq!(preds[0].confidence, PredictionConfidence::High); // 1 day out
    assert_eq!(preds[1].confidence, PredictionConfidence::Low); // 4 days out
}

// @spec WX-OUTLOOK-MODEL-007
#[test]
fn crossings_count_days_from_the_given_today() {
    let d = |day| NaiveDate::from_ymd_opt(2025, 6, day).unwrap();
    // Soil tracks air exactly over the training month.
    let pairs: Vec<(NaiveDate, f64, f64)> = (0..20)
        .map(|i| {
            (
                d(1) - chrono::Duration::days(20 - i),
                50.0 + i as f64,
                50.0 + i as f64,
            )
        })
        .collect();
    let forecast = vec![(d(2), 74.0), (d(3), 76.0)];
    let outlook = build_forecast(&pairs, &forecast, 70.0, d(1)).unwrap();
    let heat = outlook
        .threshold_crossings
        .iter()
        .find(|c| c.threshold_temp_f == 75.0)
        .expect("75 °F crossing");
    assert_eq!(heat.estimated_crossing_date, Some(d(3)));
    assert_eq!(heat.days_until_crossing, 2);
}
