use crate::models::soil_temp_prediction::*;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

/// Fewest date-aligned (air, soil) pairs a lag needs before it is considered.
const MIN_ALIGNED_PAIRS: usize = 10;
/// Fewest daily (air, soil) pairs the 10 cm outlook trains on.
pub const MIN_TRAINING_PAIRS: usize = 14;

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
/// Tries lags 0-7 days, pairing each day's soil with the air of the calendar day `lag`
/// days earlier, and keeps the best R². A day whose driver is missing from the record
/// is left out rather than paired with a neighbour's air.
/// `daily_pairs` is (date, avg_air_temp_f, avg_soil_temp_f).
/// Returns None if no lag has 10 date-aligned pairs or the best R² is below 0.3.
// @spec WX-OUTLOOK-MODEL-001
pub fn fit_model(daily_pairs: &[(NaiveDate, f64, f64)]) -> Option<SoilTempModel> {
    let air_by_date: HashMap<NaiveDate, f64> =
        daily_pairs.iter().map(|(d, air, _)| (*d, *air)).collect();
    let mut best: Option<(f64, f64, f64, u32)> = None; // (slope, intercept, r2, lag)

    for lag in 0..=7u32 {
        let (xs, ys): (Vec<f64>, Vec<f64>) = daily_pairs
            .iter()
            .filter_map(|(date, _, soil)| {
                let driver = *date - chrono::Duration::days(lag as i64);
                air_by_date.get(&driver).map(|air| (*air, *soil))
            })
            .unzip();

        if xs.len() < MIN_ALIGNED_PAIRS {
            continue;
        }

        if let Some((slope, intercept, r2)) = linear_regression(&xs, &ys) {
            if best.is_none_or(|b| r2 > b.2) {
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
/// `today` is the lawn's local date; each prediction's confidence depends on its days out.
// @spec WX-OUTLOOK-MODEL-007
pub fn predict_soil_temps(
    model: &SoilTempModel,
    recent_daily_air: &[(NaiveDate, f64)],
    forecast_daily_air: &[(NaiveDate, f64)],
    today: NaiveDate,
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
        let days_out = (*forecast_date - today).num_days();
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

/// A 10 cm threshold worth announcing, and the direction in which it means something.
/// Pre-emergent, crabgrass and seeding are deliberately absent: those are judged on the
/// 5 cm timing windows (logic/timing), and a second, 10 cm answer next to the Timing
/// widget — labelled "Pre-Emergent Window" while soil *falls* through 50°F in October —
/// only contradicted it.
pub struct AgronomicThreshold {
    pub name: &'static str,
    pub temp_f: f64,
    /// `None` = meaningful both ways.
    pub direction: Option<CrossingDirection>,
}

pub const THRESHOLDS: &[AgronomicThreshold] = &[
    AgronomicThreshold {
        name: "Winterizer / Dormancy",
        temp_f: 45.0,
        direction: Some(CrossingDirection::Falling),
    },
    AgronomicThreshold {
        name: "Grub Control Window",
        temp_f: 60.0,
        direction: Some(CrossingDirection::Rising),
    },
    AgronomicThreshold {
        name: "Active Growth Peak",
        temp_f: 65.0,
        direction: None,
    },
    AgronomicThreshold {
        name: "Heat Stress Risk",
        temp_f: 75.0,
        direction: Some(CrossingDirection::Rising),
    },
];

/// Detect which agronomic thresholds will be crossed based on current soil temp + predictions.
// @spec WX-OUTLOOK-CROSS-005
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
                if threshold
                    .direction
                    .is_some_and(|wanted| wanted != direction)
                {
                    break; // crossed the wrong way: nothing to announce
                }

                let days_until = (pred.date - today).num_days();

                crossings.push(ThresholdPrediction {
                    threshold_name: threshold.name.to_string(),
                    threshold_temp_f: threshold.temp_f,
                    estimated_crossing_date: Some(pred.date),
                    beyond_forecast: false,
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
            && threshold.direction != Some(CrossingDirection::Falling)
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
                // A straight line past the forecast is a direction, not a date.
                let beyond_forecast = est_date > last_pred.date;

                crossings.push(ThresholdPrediction {
                    threshold_name: threshold.name.to_string(),
                    threshold_temp_f: threshold.temp_f,
                    estimated_crossing_date: (!beyond_forecast).then_some(est_date),
                    beyond_forecast,
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

/// Build the 10 cm outlook from the gold daily (air, soil) pairs and the forecast's
/// full-day mean air: one continuous air series, predictions anchored to the last
/// observed soil, and crossings walked from the live 10 cm reading through the days
/// after today.
// @spec WX-OUTLOOK-MODEL-008
pub fn build_forecast(
    daily_pairs: &[(NaiveDate, f64, f64)],
    forecast_daily_air: &[(NaiveDate, f64)],
    current_soil_temp_f: f64,
    today: NaiveDate,
) -> Option<SoilTempForecast> {
    let model = fit_model(daily_pairs)?;
    let &(last_date, _, last_soil) = daily_pairs.last()?;
    let station_air: Vec<(NaiveDate, f64)> =
        daily_pairs.iter().map(|(d, air, _)| (*d, *air)).collect();
    let air = continuous_air(&station_air, forecast_daily_air);
    let predictions = anchored_outlook(&model, &air, (last_date, last_soil), today);

    if predictions.is_empty() {
        return None;
    }

    // The live reading is today's soil; estimates for days the lake has not reported
    // yet are history to it.
    let ahead: Vec<SoilTempPrediction> = predictions
        .iter()
        .filter(|p| p.date > today)
        .cloned()
        .collect();
    let threshold_crossings = predict_threshold_crossings(current_soil_temp_f, &ahead, today);

    Some(SoilTempForecast {
        predictions,
        threshold_crossings,
        model_info: SoilTempModelInfo::from(&model),
        generated_at: Utc::now(),
    })
}

mod series;
pub use series::{anchored_outlook, continuous_air};

#[cfg(test)]
mod outlook_tests;
#[cfg(test)]
mod tests;
