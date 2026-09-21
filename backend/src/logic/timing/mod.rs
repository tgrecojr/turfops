//! Seeding and pre-emergent timing windows.
//!
//! Every window is a pair of boundaries (plus an optional ideal stretch) defined on the
//! 5-day mean soil temperature at 5 cm, the station's freeze climatology, or running
//! GDD. Each boundary gets a *typical* date — found per historical year, then summarized
//! as median/spread — and a standing for the current season: observed in station data,
//! tentative, forecast, or still on its typical date. Pure functions; the handler in
//! `api/timing.rs` supplies the data.

pub mod climatology;
pub mod companions;
pub mod context;
pub mod evaluate;
pub mod log;
pub mod plan;
pub mod recommendations;
pub mod series;
pub mod text;
pub mod windows;

#[cfg(test)]
mod tests;

use crate::datasources::weather::ClimateDay;
use crate::logic::soil_temp_prediction::{fit_model, predict_soil_temps};
use crate::models::timing::{FreezeDates, SoilNow, TimingWindow, WindowState};
use crate::models::{Application, DailyForecast, GrassType};
use chrono::{Datelike, Duration, NaiveDate};
use evaluate::{Season, WindowViews};
use series::SoilPoint;
use std::collections::BTreeMap;

/// Years of station history requested for the typical dates.
pub const HISTORY_YEARS: i32 = 15;
/// Soil depth the windows are defined on.
pub const SOIL_DEPTH_CM: u32 = 5;
/// Days of paired air/soil data used to fit the soil forecast.
const FORECAST_TRAINING_DAYS: i64 = 30;
/// Longest hole in the air series that is bridged by interpolation.
const MAX_AIR_GAP_DAYS: i64 = 7;
/// Station soil data older than this cannot tell us what has happened this season.
const STALE_AFTER_DAYS: i64 = 5;

pub struct Inputs<'a> {
    pub today: NaiveDate,
    /// Ascending daily climate record, `HISTORY_YEARS` back through the latest day.
    pub days: &'a [ClimateDay],
    pub forecast: &'a [DailyForecast],
    pub grass: GrassType,
    /// Applications from at least the previous July onward.
    pub history: &'a [Application],
}

pub struct Assessment {
    pub windows: Vec<TimingWindow>,
    pub soil: SoilNow,
    pub freeze: FreezeDates,
    pub history_years: Vec<i32>,
    pub smoothed: Vec<SoilPoint>,
    pub data_fresh: bool,
}

fn observed_points(days: &[ClimateDay]) -> Vec<SoilPoint> {
    days.iter()
        .filter_map(|d| {
            Some(SoilPoint {
                date: d.date,
                temp_f: d.soil_temp_5_f?,
                is_forecast: false,
            })
        })
        .collect()
}

/// Daily mean air temperature from `from` on: station values, then the forecast, with
/// any short hole between or inside them filled by linear interpolation. The lake trails
/// real time by a day or more while the forecast starts today, and the soil model reads
/// air from `lag` days back — without the fill, the outlook loses every day whose driver
/// falls in that hole.
fn continuous_air(
    days: &[ClimateDay],
    forecast: &[DailyForecast],
    from: NaiveDate,
) -> Vec<(NaiveDate, f64)> {
    let mut known: BTreeMap<NaiveDate, f64> = forecast
        .iter()
        .map(|f| (f.date, (f.high_temp_f + f.low_temp_f) / 2.0))
        .collect();
    // Station measurements win over the forecast for the same day.
    known.extend(
        days.iter()
            .filter(|d| d.date >= from)
            .filter_map(|d| Some((d.date, d.air_avg_f?))),
    );

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

/// Estimate 5 cm soil for each day after the last observation through the end of the
/// forecast, using the lagged air→soil regression fit on the most recent month. The
/// estimates are shifted by the model's error on the last observed day so the outlook
/// continues from the measured soil temperature instead of jumping to the fitted line.
/// Empty when the fit is too weak or there is no forecast.
fn forecast_points(days: &[ClimateDay], forecast: &[DailyForecast]) -> Vec<SoilPoint> {
    let Some(last_soil) = days.iter().rev().find(|d| d.soil_temp_5_f.is_some()) else {
        return Vec::new();
    };
    if forecast.is_empty() {
        return Vec::new();
    }
    let training_start = last_soil.date - Duration::days(FORECAST_TRAINING_DAYS);
    let pairs: Vec<(NaiveDate, f64, f64)> = days
        .iter()
        .filter(|d| d.date >= training_start)
        .filter_map(|d| Some((d.date, d.air_avg_f?, d.soil_temp_5_f?)))
        .collect();
    let Some(model) = fit_model(&pairs) else {
        return Vec::new();
    };

    let air = continuous_air(days, forecast, training_start);
    let (history, outlook): (Vec<_>, Vec<_>) = air.iter().partition(|(d, _)| *d < last_soil.date);
    // `outlook` starts on the last observed day so the model's error there is known.
    let predictions = predict_soil_temps(&model, &history, &outlook);
    let anchor = predictions
        .iter()
        .find(|p| p.date == last_soil.date)
        .zip(last_soil.soil_temp_5_f)
        .map_or(0.0, |(fitted, observed)| {
            observed - fitted.predicted_soil_temp_f
        });

    predictions
        .into_iter()
        .filter(|p| p.date > last_soil.date)
        .map(|p| SoilPoint {
            date: p.date,
            temp_f: p.predicted_soil_temp_f + anchor,
            is_forecast: true,
        })
        .collect()
}

fn evaluate_window(
    spec: &windows::WindowSpec,
    season: &Season,
    history: &[Application],
) -> TimingWindow {
    let season_year = windows::season_year(spec.id, season.today);
    let views = WindowViews::build(spec, season, season_year);
    let logged = log::check(spec.id, season_year, history);
    let state = if logged.done_on.is_some() {
        WindowState::Done
    } else if logged.conflict.is_some() {
        WindowState::Blocked
    } else {
        views.weather_state(season.today)
    };

    TimingWindow {
        id: spec.id,
        name: spec.name.to_string(),
        priority: spec.priority,
        season_year,
        state,
        headline: text::headline(state, &views, logged.done_on),
        detail: text::detail(state, &views),
        guidance: text::guidance(spec.id, state),
        opens: views.opens,
        ideal_from: views.ideal_from,
        ideal_until: views.ideal_until,
        closes: views.closes,
        done_on: logged.done_on,
        conflict: logged.conflict,
    }
}

/// The smoothed soil series and climatology that every evaluation shares.
pub struct SeasonData {
    pub smoothed: Vec<SoilPoint>,
    pub history_years: Vec<i32>,
    pub spring_freezes: Vec<(i32, NaiveDate)>,
    pub fall_freezes: Vec<(i32, NaiveDate)>,
    last_observed: Option<NaiveDate>,
}

impl SeasonData {
    pub fn build(days: &[ClimateDay], forecast: &[DailyForecast], current_year: i32) -> Self {
        let mut points = observed_points(days);
        let last_observed = points.last().map(|p| p.date);
        points.extend(forecast_points(days, forecast));
        let history_years = climatology::history_years(days, current_year);
        Self {
            smoothed: series::smooth(&points),
            spring_freezes: climatology::spring_freeze_history(days, &history_years),
            fall_freezes: climatology::fall_freeze_history(days, &history_years),
            history_years,
            last_observed,
        }
    }

    pub fn season<'a>(&'a self, days: &'a [ClimateDay], today: NaiveDate) -> Season<'a> {
        Season {
            today,
            days,
            smoothed: &self.smoothed,
            history_years: &self.history_years,
            fall_freezes: &self.fall_freezes,
            data_fresh: self
                .last_observed
                .is_some_and(|d| (today - d).num_days() <= STALE_AFTER_DAYS),
        }
    }
}

pub fn assess(inputs: &Inputs) -> Assessment {
    let today = inputs.today;
    let data = SeasonData::build(inputs.days, inputs.forecast, today.year());
    let season = data.season(inputs.days, today);
    let data_fresh = season.data_fresh;
    let windows = windows::specs_for(inputs.grass)
        .iter()
        .map(|spec| evaluate_window(spec, &season, inputs.history))
        .collect();

    let latest = data.smoothed.iter().rev().find(|p| !p.is_forecast);
    Assessment {
        windows,
        soil: SoilNow {
            avg_5day_f: latest.map(|p| (p.temp_f * 10.0).round() / 10.0),
            as_of: latest.map(|p| p.date),
            depth_cm: SOIL_DEPTH_CM,
        },
        freeze: FreezeDates {
            last_spring: climatology::date_stat(&data.spring_freezes, today.year()),
            first_fall: climatology::date_stat(&data.fall_freezes, today.year()),
            last_spring_this_year: climatology::last_spring_freeze(inputs.days, today.year())
                .filter(|_| today.month() >= 6),
            first_fall_this_year: climatology::first_fall_freeze(inputs.days, today.year()),
        },
        history_years: data.history_years,
        smoothed: data.smoothed,
        data_fresh,
    }
}
