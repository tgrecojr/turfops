//! Builds the daily weather series the disease models run on: observed lake days,
//! with today's remainder and the coming days filled in from the 3-hourly forecast.

use crate::clock::Clock;
use crate::models::{DailyWeather, ForecastPoint, AFTERNOON_HOURS, DAWN_HOURS};
use chrono::{NaiveDate, Timelike};
use std::collections::BTreeMap;

/// Hours each OpenWeatherMap forecast point stands for.
const FORECAST_STEP_HOURS: f64 = 3.0;
/// RH (%) at or above which a leaf is assumed wet.
const LEAF_WETNESS_RH: f64 = 90.0;

/// Magnus-formula dew point (°C) from air temp (°C) and RH (%).
pub fn dew_point_c(temp_c: f64, rh: f64) -> f64 {
    let gamma = (rh.max(1.0) / 100.0).ln() + 17.625 * temp_c / (243.04 + temp_c);
    243.04 * gamma / (17.625 - gamma)
}

fn fahrenheit_to_celsius(f: f64) -> f64 {
    (f - 32.0) * 5.0 / 9.0
}

/// Aggregate 3-hourly forecast points into the lawn's local calendar days, each point on
/// the local date (and hour) of its own time.
// @spec DIS-MODEL-DAY-003
/// Points at or before `after` (the newest lake observation) are dropped: those hours
/// are already observed.
pub fn forecast_days(
    points: &[ForecastPoint],
    clock: &Clock,
    after: Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<DailyWeather> {
    let mut by_day: BTreeMap<NaiveDate, Vec<&ForecastPoint>> = BTreeMap::new();
    // @spec DIS-MODEL-DAY-007
    for point in points
        .iter()
        .filter(|p| after.is_none_or(|t| p.timestamp > t))
    {
        by_day
            .entry(clock.local_date(point.timestamp))
            .or_default()
            .push(point);
    }

    // @spec DIS-MODEL-DAY-005
    let local_hour = |p: &&ForecastPoint| clock.local(p.timestamp).hour();
    by_day
        .into_iter()
        .map(|(date, pts)| {
            let n = pts.len() as f64;
            let temps: Vec<f64> = pts
                .iter()
                .map(|p| fahrenheit_to_celsius(p.temp_f))
                .collect();
            let humid = |p: &&&ForecastPoint| p.humidity_percent >= LEAF_WETNESS_RH;
            DailyWeather {
                date,
                temp_mean_c: temps.iter().sum::<f64>() / n,
                temp_min_c: temps.iter().copied().fold(f64::INFINITY, f64::min),
                temp_max_c: temps.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                rh_mean: pts.iter().map(|p| p.humidity_percent).sum::<f64>() / n,
                hours_rh90: pts.iter().filter(humid).count() as f64 * FORECAST_STEP_HOURS,
                leaf_wetness_hours: pts
                    .iter()
                    .filter(|p| humid(p) || p.precipitation_mm > 0.0)
                    .count() as f64
                    * FORECAST_STEP_HOURS,
                precip_mm: pts.iter().map(|p| p.precipitation_mm).sum(),
                dew_point_max_c: pts
                    .iter()
                    .zip(&temps)
                    .map(|(p, t)| dew_point_c(*t, p.humidity_percent))
                    .fold(f64::NEG_INFINITY, f64::max),
                hours_covered: n * FORECAST_STEP_HOURS,
                is_forecast: true,
                covers_dawn: pts.iter().any(|p| DAWN_HOURS.contains(&local_hour(p))),
                covers_afternoon: pts.iter().any(|p| AFTERNOON_HOURS.contains(&local_hour(p))),
            }
        })
        .collect()
}

/// Combine two partial views of the same day, weighting means by hours covered.
fn merge_day(a: &DailyWeather, b: &DailyWeather) -> DailyWeather {
    let total = a.hours_covered + b.hours_covered;
    let weighted = |x: f64, y: f64| (x * a.hours_covered + y * b.hours_covered) / total;
    DailyWeather {
        date: a.date,
        temp_mean_c: weighted(a.temp_mean_c, b.temp_mean_c),
        temp_min_c: a.temp_min_c.min(b.temp_min_c),
        temp_max_c: a.temp_max_c.max(b.temp_max_c),
        rh_mean: weighted(a.rh_mean, b.rh_mean),
        hours_rh90: a.hours_rh90 + b.hours_rh90,
        leaf_wetness_hours: a.leaf_wetness_hours + b.leaf_wetness_hours,
        precip_mm: a.precip_mm + b.precip_mm,
        dew_point_max_c: a.dew_point_max_c.max(b.dew_point_max_c),
        hours_covered: total.min(24.0),
        is_forecast: a.is_forecast || b.is_forecast,
        covers_dawn: a.covers_dawn || b.covers_dawn,
        covers_afternoon: a.covers_afternoon || b.covers_afternoon,
    }
}

// @spec DIS-MODEL-DAY-006
/// Merge observed and forecast days into one ascending series. A fully observed day
/// wins outright; a partly observed day (the lake lags, so usually today) is topped up
/// with whatever forecast points remain for it.
pub fn build_series(observed: Vec<DailyWeather>, forecast: Vec<DailyWeather>) -> Vec<DailyWeather> {
    let mut by_day: BTreeMap<NaiveDate, DailyWeather> =
        observed.into_iter().map(|d| (d.date, d)).collect();
    for day in forecast {
        let merged = match by_day.get(&day.date) {
            Some(obs) if obs.hours_covered >= 24.0 => continue,
            Some(obs) => merge_day(obs, &day),
            None => day,
        };
        by_day.insert(merged.date, merged);
    }
    by_day.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day};
    use crate::models::WeatherCondition;
    use chrono::{TimeZone, Utc};

    fn point(day: u32, hour: u32, temp_f: f64, rh: f64, precip: f64) -> ForecastPoint {
        ForecastPoint {
            timestamp: Utc.with_ymd_and_hms(2026, 9, day, hour, 0, 0).unwrap(),
            temp_f,
            feels_like_f: temp_f,
            humidity_percent: rh,
            precipitation_mm: precip,
            precipitation_prob: 0.0,
            wind_speed_mph: 0.0,
            wind_gust_mph: None,
            cloud_cover_percent: 0.0,
            weather_condition: WeatherCondition::Clear,
        }
    }

    #[test]
    fn dew_point_matches_known_values() {
        // Saturated air: dew point equals air temp.
        assert!((dew_point_c(20.0, 100.0) - 20.0).abs() < 0.01);
        // 25°C at 60% RH → ~16.7°C.
        assert!((dew_point_c(25.0, 60.0) - 16.7).abs() < 0.1);
    }

    fn eastern() -> Clock {
        Clock::Zone(chrono_tz::America::New_York)
    }

    // @spec DIS-MODEL-DAY-003, DIS-MODEL-DAY-004, DIS-MODEL-DAY-005
    #[test]
    fn forecast_points_bucket_by_local_date() {
        // 02:00 UTC on the 21st is 22:00 on the 20th at UTC-4.
        let points = [
            point(21, 2, 68.0, 95.0, 0.0),
            point(21, 12, 77.0, 60.0, 1.0),
        ];
        let days = forecast_days(&points, &eastern(), None);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date, date(9, 20));
        assert_eq!(days[0].hours_rh90, 3.0);
        assert!((days[0].temp_mean_c - 20.0).abs() < 1e-9);
        assert_eq!(days[1].date, date(9, 21));
        assert_eq!(days[1].hours_rh90, 0.0);
        assert_eq!(days[1].leaf_wetness_hours, 3.0);
        assert!(days[1].is_forecast);
        // 22:00 local is neither dawn nor afternoon; 08:00 local (12:00 UTC) is dawn.
        assert!(!days[0].covers_dawn && !days[0].covers_afternoon);
        assert!(days[1].covers_dawn && !days[1].covers_afternoon);
    }

    // @spec DIS-MODEL-DAY-003
    #[test]
    fn forecast_points_follow_daylight_saving() {
        // DST ends 2026-11-01 06:00 UTC. 04:00 UTC Nov 2 is 11 pm EST Nov 1 (a fixed EDT
        // offset would say midnight Nov 2); 12:00 UTC Nov 2 is 7 am EST, a dawn hour.
        let at = |d: u32, h: u32| {
            let mut p = point(1, 0, 50.0, 50.0, 0.0);
            p.timestamp = Utc.with_ymd_and_hms(2026, 11, d, h, 0, 0).unwrap();
            p
        };
        let days = forecast_days(&[at(2, 4), at(2, 12)], &eastern(), None);
        assert_eq!(days.len(), 2);
        assert_eq!(
            days[0].date,
            chrono::NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
        );
        assert_eq!(
            days[1].date,
            chrono::NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );
        assert!(days[1].covers_dawn);
    }

    // @spec DIS-MODEL-DAY-006
    #[test]
    fn partial_observed_day_is_topped_up_from_forecast() {
        let mut observed_today = day(date(9, 20), 18.0, 17.0, 19.0, 90.0);
        observed_today.hours_covered = 6.0;
        observed_today.hours_rh90 = 6.0;
        let mut forecast_today = day(date(9, 20), 24.0, 20.0, 28.0, 60.0);
        forecast_today.hours_covered = 12.0;
        forecast_today.is_forecast = true;

        let series = build_series(vec![observed_today], vec![forecast_today]);
        assert_eq!(series.len(), 1);
        let merged = &series[0];
        assert_eq!(merged.hours_covered, 18.0);
        assert!((merged.temp_mean_c - 22.0).abs() < 1e-9);
        assert_eq!(merged.temp_min_c, 17.0);
        assert_eq!(merged.temp_max_c, 28.0);
        assert!((merged.rh_mean - 70.0).abs() < 1e-9);
        assert_eq!(merged.hours_rh90, 6.0);
        assert!(merged.is_forecast);
    }

    #[test]
    fn complete_observed_day_ignores_forecast() {
        let observed = day(date(9, 19), 18.0, 17.0, 19.0, 90.0);
        let mut forecast = day(date(9, 19), 30.0, 25.0, 35.0, 40.0);
        forecast.is_forecast = true;
        let series = build_series(vec![observed.clone()], vec![forecast]);
        assert_eq!(series, vec![observed]);
    }

    // @spec DIS-MODEL-DAY-005
    #[test]
    fn each_forecast_point_is_three_hours_of_humidity_and_wetness() {
        // 15:00, 18:00 and 21:00 local on 9/20 (UTC-4).
        let points = [
            point(20, 19, 70.0, 90.0, 0.0),
            point(20, 22, 72.0, 89.9, 0.2),
            point(21, 1, 68.0, 70.0, 0.0),
        ];
        let days = forecast_days(&points, &eastern(), None);
        assert_eq!(days.len(), 1);
        let d = &days[0];
        assert_eq!(d.hours_covered, 9.0);
        assert_eq!(d.hours_rh90, 3.0, "RH of exactly 90% counts");
        assert_eq!(d.leaf_wetness_hours, 6.0, "humid or wet, not dry");
        assert!((d.precip_mm - 0.2).abs() < 1e-9);
    }

    // @spec DIS-MODEL-DAY-006
    #[test]
    fn observed_and_forecast_days_join_into_one_sorted_series() {
        let mut partial = day(date(9, 20), 20.0, 15.0, 25.0, 80.0);
        partial.hours_covered = 18.0;
        partial.precip_mm = 1.0;
        let mut rest_of_today = day(date(9, 20), 20.0, 14.0, 26.0, 80.0);
        rest_of_today.hours_covered = 9.0;
        rest_of_today.precip_mm = 2.0;
        rest_of_today.is_forecast = true;
        let mut tomorrow = day(date(9, 21), 22.0, 17.0, 27.0, 70.0);
        tomorrow.is_forecast = true;
        let full = day(date(9, 19), 18.0, 12.0, 24.0, 85.0);

        let series = build_series(
            vec![partial, full.clone()],
            vec![tomorrow.clone(), rest_of_today],
        );
        let dates: Vec<_> = series.iter().map(|d| d.date).collect();
        assert_eq!(dates, vec![date(9, 19), date(9, 20), date(9, 21)]);
        assert_eq!(series[0], full);
        assert_eq!(series[1].hours_covered, 24.0, "capped at 24");
        assert_eq!(series[1].precip_mm, 3.0);
        assert_eq!((series[1].temp_min_c, series[1].temp_max_c), (14.0, 26.0));
        assert_eq!(series[2], tomorrow);
    }

    // @spec DIS-MODEL-DAY-007
    #[test]
    fn hours_the_lake_has_reported_are_not_counted_again() {
        // Lake reported through 14:00 UTC; the 12:00 forecast point is already observed.
        let last_observed = Utc.with_ymd_and_hms(2026, 9, 21, 14, 0, 0).unwrap();
        let points = [
            point(21, 12, 70.0, 95.0, 0.0),
            point(21, 15, 72.0, 95.0, 0.0),
            point(21, 18, 74.0, 95.0, 0.0),
        ];
        let days = forecast_days(&points, &eastern(), Some(last_observed));
        let total: f64 = days.iter().map(|d| d.hours_covered).sum();
        assert_eq!(total, 6.0, "only the 15:00 and 18:00 points remain");
    }
}
