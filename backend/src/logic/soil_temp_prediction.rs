use crate::models::soil_temp_prediction::*;
use chrono::{NaiveDate, Utc};

/// Standard least-squares linear regression.
/// Returns (slope, intercept, r_squared) or None if insufficient data.
pub fn linear_regression(xs: &[f64], ys: &[f64]) -> Option<(f64, f64, f64)> {
    let n = xs.len();
    if n < 2 || n != ys.len() {
        return None;
    }

    let n_f = n as f64;
    let sum_x: f64 = xs.iter().sum();
    let sum_y: f64 = ys.iter().sum();
    let sum_xy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
    let sum_x2: f64 = xs.iter().map(|x| x * x).sum();
    let sum_y2: f64 = ys.iter().map(|y| y * y).sum();

    let denom = n_f * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-12 {
        return None;
    }

    let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n_f;

    // R-squared
    let ss_tot = sum_y2 - (sum_y * sum_y) / n_f;
    let ss_res: f64 = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| {
            let pred = slope * x + intercept;
            (y - pred) * (y - pred)
        })
        .sum();

    let r_squared = if ss_tot.abs() < 1e-12 {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some((slope, intercept, r_squared))
}

/// Fit a lagged linear regression model: soil_temp = slope * lagged_air_temp + intercept.
/// Tries lags 0-7 days, picks best R-squared.
/// `daily_pairs` is (date, avg_air_temp_f, avg_soil_temp_10_f).
/// Returns None if <14 data points or best R-squared < 0.3.
pub fn fit_model(daily_pairs: &[(NaiveDate, f64, f64)]) -> Option<SoilTempModel> {
    if daily_pairs.len() < 14 {
        return None;
    }

    let mut best: Option<(f64, f64, f64, u32)> = None; // (slope, intercept, r2, lag)

    for lag in 0..=7u32 {
        let lag_usize = lag as usize;
        if daily_pairs.len() <= lag_usize {
            continue;
        }

        // Build aligned pairs: air_temp[i] predicts soil_temp[i + lag]
        let mut xs = Vec::new();
        let mut ys = Vec::new();

        for i in 0..daily_pairs.len() - lag_usize {
            let (_, air_temp, _) = daily_pairs[i];
            let (_, _, soil_temp) = daily_pairs[i + lag_usize];
            xs.push(air_temp);
            ys.push(soil_temp);
        }

        if xs.len() < 10 {
            continue;
        }

        if let Some((slope, intercept, r2)) = linear_regression(&xs, &ys) {
            if best.is_none() || r2 > best.unwrap().2 {
                best = Some((slope, intercept, r2, lag));
            }
        }
    }

    let (slope, intercept, r_squared, lag_days) = best?;

    if r_squared < 0.3 {
        return None;
    }

    Some(SoilTempModel {
        slope,
        intercept,
        lag_days,
        r_squared,
        training_window_days: daily_pairs.len() as u32,
        fitted_at: Utc::now(),
    })
}

/// Produce soil temperature predictions using the fitted model.
/// `recent_daily_air` = recent actual daily avg air temps (date, temp_f), most recent last.
/// `forecast_daily_air` = OWM forecast daily avg air temps (date, temp_f), chronological.
/// The model uses air temp from `lag_days` before each prediction date.
pub fn predict_soil_temps(
    model: &SoilTempModel,
    recent_daily_air: &[(NaiveDate, f64)],
    forecast_daily_air: &[(NaiveDate, f64)],
) -> Vec<SoilTempPrediction> {
    let lag = model.lag_days as i64;
    let mut predictions = Vec::new();

    for (forecast_date, _forecast_temp) in forecast_daily_air {
        // The air temp that drives this prediction is from `lag_days` ago
        let driver_date = *forecast_date - chrono::Duration::days(lag);

        // Look for driver_date in actual data first, then forecast
        let (air_temp, source) = if let Some((_, temp)) =
            recent_daily_air.iter().find(|(d, _)| *d == driver_date)
        {
            (*temp, format!("Actual air temp from {}", driver_date))
        } else if let Some((_, temp)) = forecast_daily_air.iter().find(|(d, _)| *d == driver_date) {
            (*temp, format!("Forecast air temp from {}", driver_date))
        } else {
            // If lag=0, use forecast for the same day
            if lag == 0 {
                (
                    *_forecast_temp,
                    format!("Forecast air temp for {}", forecast_date),
                )
            } else {
                continue; // Can't make this prediction
            }
        };

        let predicted_soil_temp = model.slope * air_temp + model.intercept;

        // Confidence based on model quality + forecast distance
        let days_out = (*forecast_date - Utc::now().date_naive()).num_days();
        let confidence = compute_confidence(model.r_squared, days_out);

        predictions.push(SoilTempPrediction {
            date: *forecast_date,
            predicted_soil_temp_f: (predicted_soil_temp * 10.0).round() / 10.0,
            confidence,
            air_temp_used_f: air_temp,
            source_description: source,
        });
    }

    predictions
}

fn compute_confidence(r_squared: f64, days_out: i64) -> PredictionConfidence {
    if r_squared >= 0.7 && days_out <= 2 {
        PredictionConfidence::High
    } else if r_squared >= 0.5 && days_out <= 3 {
        PredictionConfidence::Medium
    } else {
        PredictionConfidence::Low
    }
}

/// Agronomic thresholds to watch for
pub struct AgronomicThreshold {
    pub name: &'static str,
    pub temp_f: f64,
}

pub const THRESHOLDS: &[AgronomicThreshold] = &[
    AgronomicThreshold {
        name: "Winterizer / Dormancy",
        temp_f: 45.0,
    },
    AgronomicThreshold {
        name: "Pre-Emergent Window",
        temp_f: 50.0,
    },
    AgronomicThreshold {
        name: "Crabgrass Germination",
        temp_f: 55.0,
    },
    AgronomicThreshold {
        name: "Grub Control Window",
        temp_f: 60.0,
    },
    AgronomicThreshold {
        name: "Active Growth Peak",
        temp_f: 65.0,
    },
    AgronomicThreshold {
        name: "Heat Stress Risk",
        temp_f: 75.0,
    },
];

/// Detect which agronomic thresholds will be crossed based on current soil temp + predictions.
pub fn predict_threshold_crossings(
    current_soil_temp_f: f64,
    predictions: &[SoilTempPrediction],
    today: NaiveDate,
) -> Vec<ThresholdPrediction> {
    let mut crossings = Vec::new();

    for threshold in THRESHOLDS {
        let currently_below = current_soil_temp_f < threshold.temp_f;

        // Look for crossing in predictions
        let mut prev_temp = current_soil_temp_f;
        for pred in predictions {
            let crosses_up =
                prev_temp < threshold.temp_f && pred.predicted_soil_temp_f >= threshold.temp_f;
            let crosses_down =
                prev_temp >= threshold.temp_f && pred.predicted_soil_temp_f < threshold.temp_f;

            if crosses_up || crosses_down {
                let direction = if crosses_up {
                    CrossingDirection::Rising
                } else {
                    CrossingDirection::Falling
                };

                let days_until = (pred.date - today).num_days();

                crossings.push(ThresholdPrediction {
                    threshold_name: threshold.name.to_string(),
                    threshold_temp_f: threshold.temp_f,
                    estimated_crossing_date: pred.date,
                    days_until_crossing: days_until,
                    confidence: pred.confidence,
                    direction,
                });
                break; // Only first crossing per threshold
            }
            prev_temp = pred.predicted_soil_temp_f;
        }

        // Also flag if currently near a threshold and trending toward it
        if predictions.is_empty() {
            continue;
        }
        let last_pred = predictions.last().unwrap();
        let approaching = currently_below
            && last_pred.predicted_soil_temp_f > current_soil_temp_f
            && (threshold.temp_f - current_soil_temp_f) < 5.0
            && !crossings
                .iter()
                .any(|c| c.threshold_temp_f == threshold.temp_f);

        if approaching {
            // Approximate crossing date by linear interpolation
            let temp_diff = last_pred.predicted_soil_temp_f - current_soil_temp_f;
            let needed = threshold.temp_f - current_soil_temp_f;
            if temp_diff > 0.0 {
                let forecast_days = (last_pred.date - today).num_days() as f64;
                let est_days = (needed / temp_diff * forecast_days).ceil() as i64;
                let est_date = today + chrono::Duration::days(est_days);

                crossings.push(ThresholdPrediction {
                    threshold_name: threshold.name.to_string(),
                    threshold_temp_f: threshold.temp_f,
                    estimated_crossing_date: est_date,
                    days_until_crossing: est_days,
                    confidence: PredictionConfidence::Low,
                    direction: CrossingDirection::Rising,
                });
            }
        }
    }

    crossings.sort_by_key(|c| c.days_until_crossing);
    crossings
}

/// Build a complete SoilTempForecast from paired data, recent air temps, and forecast.
pub fn build_forecast(
    daily_pairs: &[(NaiveDate, f64, f64)],
    recent_daily_air: &[(NaiveDate, f64)],
    forecast_daily_air: &[(NaiveDate, f64)],
    current_soil_temp_f: f64,
) -> Option<SoilTempForecast> {
    let model = fit_model(daily_pairs)?;
    let predictions = predict_soil_temps(&model, recent_daily_air, forecast_daily_air);

    if predictions.is_empty() {
        return None;
    }

    let today = Utc::now().date_naive();
    let threshold_crossings = predict_threshold_crossings(current_soil_temp_f, &predictions, today);

    Some(SoilTempForecast {
        predictions,
        threshold_crossings,
        model_info: SoilTempModelInfo::from(&model),
        generated_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn fit_model_insufficient_data() {
        let pairs: Vec<(NaiveDate, f64, f64)> = (0..10)
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

        let preds = predict_soil_temps(&model, &recent, &forecast);
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

        let preds = predict_soil_temps(&model, &recent, &forecast);
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

        // Should detect crossing 50°F (Pre-Emergent Window)
        let pre_emergent = crossings.iter().find(|c| c.threshold_temp_f == 50.0);
        assert!(pre_emergent.is_some());
        let c = pre_emergent.unwrap();
        assert_eq!(c.direction, CrossingDirection::Rising);
        assert_eq!(c.days_until_crossing, 2); // March 18
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
        let recent: Vec<(NaiveDate, f64)> = vec![];
        let forecast: Vec<(NaiveDate, f64)> = vec![];
        assert!(build_forecast(&pairs, &recent, &forecast, 50.0).is_none());
    }
}
