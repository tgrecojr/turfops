//! The outlook's air series and anchoring, shared by the 10 cm outlook and the timing
//! windows' 5 cm outlook so both read the record the same way.

use super::predict_soil_temps;
use crate::models::soil_temp_prediction::{SoilTempModel, SoilTempPrediction};
use chrono::{Duration, NaiveDate};
use std::collections::BTreeMap;

/// Longest hole in the air series that is bridged by interpolation.
pub const MAX_AIR_GAP_DAYS: i64 = 7;

/// One daily mean air series: station values, then the forecast, with any hole of up to
/// `MAX_AIR_GAP_DAYS` between or inside them filled by linear interpolation. Station
/// values win over the forecast for the same day. The lake trails real time by a day or
/// more while the forecast starts today, and the soil model reads air from `lag` days
/// back — without the fill, the outlook loses every day whose driver falls in that hole.
// @spec WX-OUTLOOK-MODEL-008
pub fn continuous_air(
    observed: &[(NaiveDate, f64)],
    forecast: &[(NaiveDate, f64)],
) -> Vec<(NaiveDate, f64)> {
    let mut known: BTreeMap<NaiveDate, f64> = forecast.iter().copied().collect();
    known.extend(observed.iter().copied());

    let mut filled = Vec::with_capacity(known.len());
    let mut previous: Option<(NaiveDate, f64)> = None;
    for (&date, &temp) in &known {
        if let Some((prev_date, prev_temp)) = previous {
            let gap = (date - prev_date).num_days();
            if gap > 1 && gap <= MAX_AIR_GAP_DAYS {
                filled.extend((1..gap).map(|step| {
                    let share = step as f64 / gap as f64;
                    (
                        prev_date + Duration::days(step),
                        prev_temp + (temp - prev_temp) * share,
                    )
                }));
            }
        }
        filled.push((date, temp));
        previous = Some((date, temp));
    }
    filled
}

/// Predictions for every day after `last_soil`'s date that `air` can drive, shifted by
/// the model's error on that last observed day so the outlook continues from the
/// measured soil temperature instead of jumping to the fitted line. `today` sets each
/// prediction's confidence.
pub fn anchored_outlook(
    model: &SoilTempModel,
    air: &[(NaiveDate, f64)],
    last_soil: (NaiveDate, f64),
    today: NaiveDate,
) -> Vec<SoilTempPrediction> {
    let (last_date, observed) = last_soil;
    let (history, outlook): (Vec<_>, Vec<_>) = air.iter().partition(|(d, _)| *d < last_date);
    // `outlook` starts on the last observed day so the model's error there is known.
    let predictions = predict_soil_temps(model, &history, &outlook, today);
    let anchor = predictions
        .iter()
        .find(|p| p.date == last_date)
        .map_or(0.0, |fitted| observed - fitted.predicted_soil_temp_f);

    predictions
        .into_iter()
        .filter(|p| p.date > last_date)
        .map(|mut p| {
            p.predicted_soil_temp_f = ((p.predicted_soil_temp_f + anchor) * 10.0).round() / 10.0;
            p
        })
        .collect()
}
