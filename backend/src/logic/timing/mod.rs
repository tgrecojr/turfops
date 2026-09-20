//! Seeding and pre-emergent timing windows.
//!
//! Every window is a pair of boundaries (plus an optional ideal stretch) defined on the
//! 5-day mean soil temperature at 5 cm, the station's freeze climatology, or running
//! GDD. Each boundary gets a *typical* date — found per historical year, then summarized
//! as median/spread — and a standing for the current season: observed in station data,
//! tentative, forecast, or still on its typical date. Pure functions; the handler in
//! `api/timing.rs` supplies the data.

pub mod climatology;
pub mod context;
pub mod evaluate;
pub mod log;
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

/// Years of station history requested for the typical dates.
pub const HISTORY_YEARS: i32 = 15;
/// Soil depth the windows are defined on.
pub const SOIL_DEPTH_CM: u32 = 5;
/// Days of paired air/soil data used to fit the soil forecast.
const FORECAST_TRAINING_DAYS: i64 = 30;
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

/// Estimate 5 cm soil for the forecast days with the lagged air→soil regression, fit on
/// the most recent month. Empty when the fit is too weak or the forecast is missing.
fn forecast_points(days: &[ClimateDay], forecast: &[DailyForecast]) -> Vec<SoilPoint> {
    let Some(last_soil) = days.iter().rev().find(|d| d.soil_temp_5_f.is_some()) else {
        return Vec::new();
    };
    let training_start = last_soil.date - Duration::days(FORECAST_TRAINING_DAYS);
    let recent = days.iter().filter(|d| d.date >= training_start);
    let pairs: Vec<(NaiveDate, f64, f64)> = recent
        .clone()
        .filter_map(|d| Some((d.date, d.air_avg_f?, d.soil_temp_5_f?)))
        .collect();
    let Some(model) = fit_model(&pairs) else {
        return Vec::new();
    };

    let recent_air: Vec<(NaiveDate, f64)> = recent
        .filter_map(|d| Some((d.date, d.air_avg_f?)))
        .collect();
    let forecast_air: Vec<(NaiveDate, f64)> = forecast
        .iter()
        .filter(|f| f.date > last_soil.date)
        .map(|f| (f.date, (f.high_temp_f + f.low_temp_f) / 2.0))
        .collect();
    predict_soil_temps(&model, &recent_air, &forecast_air)
        .into_iter()
        .map(|p| SoilPoint {
            date: p.date,
            temp_f: p.predicted_soil_temp_f,
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

pub fn assess(inputs: &Inputs) -> Assessment {
    let today = inputs.today;
    let mut points = observed_points(inputs.days);
    let last_observed = points.last().map(|p| p.date);
    points.extend(forecast_points(inputs.days, inputs.forecast));
    let smoothed = series::smooth(&points);

    let history_years = climatology::history_years(inputs.days, today.year());
    let spring_freezes = climatology::spring_freeze_history(inputs.days, &history_years);
    let fall_freezes = climatology::fall_freeze_history(inputs.days, &history_years);
    let data_fresh = last_observed.is_some_and(|d| (today - d).num_days() <= STALE_AFTER_DAYS);

    let season = Season {
        today,
        days: inputs.days,
        smoothed: &smoothed,
        history_years: &history_years,
        fall_freezes: &fall_freezes,
        data_fresh,
    };
    let windows = windows::specs_for(inputs.grass)
        .iter()
        .map(|spec| evaluate_window(spec, &season, inputs.history))
        .collect();

    let latest = smoothed.iter().rev().find(|p| !p.is_forecast);
    Assessment {
        windows,
        soil: SoilNow {
            avg_5day_f: latest.map(|p| (p.temp_f * 10.0).round() / 10.0),
            as_of: latest.map(|p| p.date),
            depth_cm: SOIL_DEPTH_CM,
        },
        freeze: FreezeDates {
            last_spring: climatology::date_stat(&spring_freezes, today.year()),
            first_fall: climatology::date_stat(&fall_freezes, today.year()),
            last_spring_this_year: climatology::last_spring_freeze(inputs.days, today.year())
                .filter(|_| today.month() >= 6),
            first_fall_this_year: climatology::first_fall_freeze(inputs.days, today.year()),
        },
        history_years,
        smoothed,
        data_fresh,
    }
}
