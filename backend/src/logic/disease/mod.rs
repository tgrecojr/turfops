//! Per-disease turf risk models. Every model is a pure function over a daily weather
//! series (observed lake days + forecast days) — no IO, mirroring the rules engine.
//! Each disease keeps its own published/native score; only the Low/Moderate/High/Severe
//! tier is comparable across diseases, so no blended score is ever computed.

mod brown_patch;
mod dollar_spot;
mod gray_leaf_spot;
mod history;
mod management;
mod pythium;
pub mod recommendations;
mod red_thread;
pub mod weather_days;

pub use history::context_from_history;

use crate::models::{
    celsius_to_fahrenheit, DailyRisk, DailyWeather, DiseaseContext, DiseaseRisk, FactorStatus,
    RiskScale, RiskTier,
};
use chrono::{Duration, NaiveDate};

/// Observed days shown before today.
pub const HISTORY_DAYS: i64 = 6;
/// Observed days fetched, so trailing windows are full on the first displayed day.
pub const LOOKBACK_DAYS: i64 = 12;
/// Forecast days shown after today.
pub const FORECAST_DAYS: i64 = 4;
/// A day needs this many hours of data to drive a headline or a trailing window.
pub const MIN_USABLE_HOURS: f64 = 12.0;

/// Run every disease model. `series` must be ascending by date; diseases with no
/// usable data in the window are omitted.
pub fn assess_all(
    series: &[DailyWeather],
    today: NaiveDate,
    ctx: &DiseaseContext,
) -> Vec<DiseaseRisk> {
    [
        brown_patch::assess(series, today, ctx),
        dollar_spot::assess(series, today, ctx),
        pythium::assess(series, today, ctx),
        gray_leaf_spot::assess(series, today, ctx),
        red_thread::assess(series, today, ctx),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn is_usable(day: &DailyWeather) -> bool {
    day.hours_covered >= MIN_USABLE_HOURS
}

/// Mean of `field` over the usable days in the `window`-day span ending at `idx`
/// (the day at `idx` always counts). `None` with fewer than `min_days` contributing.
fn trailing_mean(
    series: &[DailyWeather],
    idx: usize,
    window: i64,
    min_days: usize,
    field: impl Fn(&DailyWeather) -> f64,
) -> Option<f64> {
    let end = series[idx].date;
    let start = end - Duration::days(window - 1);
    let values: Vec<f64> = series[..=idx]
        .iter()
        .enumerate()
        .filter(|(i, d)| d.date >= start && (*i == idx || is_usable(d)))
        .map(|(_, d)| field(d))
        .collect();
    if values.len() < min_days {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Daily scores across the display window plus the series index of the headline day.
struct ScoredWindow {
    daily: Vec<DailyRisk>,
    headline_idx: usize,
    headline_value: f64,
}

/// Score each day in `[today - HISTORY_DAYS, today + FORECAST_DAYS]` with `value_at`.
/// The headline is the latest usable day up to today: today itself once enough of it
/// is covered, otherwise the most recent complete day.
fn score_window(
    series: &[DailyWeather],
    today: NaiveDate,
    scale: &RiskScale,
    value_at: impl Fn(usize) -> Option<f64>,
) -> Option<ScoredWindow> {
    let first = today - Duration::days(HISTORY_DAYS);
    let last = today + Duration::days(FORECAST_DAYS);
    let mut daily = Vec::new();
    let mut headline: Option<(usize, f64)> = None;

    for (idx, day) in series.iter().enumerate() {
        if day.date < first || day.date > last {
            continue;
        }
        let Some(value) = value_at(idx) else { continue };
        daily.push(DailyRisk {
            date: day.date,
            value,
            tier: scale.tier_for(value),
            is_forecast: day.is_forecast,
            partial: !is_usable(day),
        });
        if day.date <= today && is_usable(day) {
            headline = Some((idx, value));
        }
    }

    let (headline_idx, headline_value) = headline?;
    Some(ScoredWindow {
        daily,
        headline_idx,
        headline_value,
    })
}

/// Raise every tier at or above Moderate by one step (history-driven amplifiers never
/// manufacture risk when the weather says Low).
fn amplify(tier: RiskTier) -> RiskTier {
    if tier >= RiskTier::Moderate {
        tier.raised()
    } else {
        tier
    }
}

/// Piecewise-linear suitability: 0 at/below `lo`, 1 across `[opt_lo, opt_hi]`,
/// 0 at/above `hi`.
fn trapezoid(x: f64, lo: f64, opt_lo: f64, opt_hi: f64, hi: f64) -> f64 {
    if x <= lo || x >= hi {
        0.0
    } else if x < opt_lo {
        (x - lo) / (opt_lo - lo)
    } else if x <= opt_hi {
        1.0
    } else {
        (hi - x) / (hi - opt_hi)
    }
}

/// Linear ramp: 0 at/below `lo`, 1 at/above `hi`.
fn ramp(x: f64, lo: f64, hi: f64) -> f64 {
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// Factor status for a 0–1 suitability value.
fn suitability_status(s: f64) -> FactorStatus {
    if s >= 0.75 {
        FactorStatus::Favorable
    } else if s > 0.0 {
        FactorStatus::Marginal
    } else {
        FactorStatus::Unfavorable
    }
}

fn fmt_temp_f(temp_c: f64) -> String {
    format!("{:.0}°F", celsius_to_fahrenheit(temp_c))
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A complete observed day with neutral moisture values.
    pub fn day(date: NaiveDate, mean_c: f64, min_c: f64, max_c: f64, rh: f64) -> DailyWeather {
        DailyWeather {
            date,
            temp_mean_c: mean_c,
            temp_min_c: min_c,
            temp_max_c: max_c,
            rh_mean: rh,
            hours_rh90: 0.0,
            leaf_wetness_hours: 0.0,
            precip_mm: 0.0,
            dew_point_max_c: weather_days::dew_point_c(mean_c, rh),
            hours_covered: 24.0,
            is_forecast: false,
        }
    }

    pub fn date(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    /// `n` identical days ending on `end`.
    pub fn run_of(
        end: NaiveDate,
        n: i64,
        make: impl Fn(NaiveDate) -> DailyWeather,
    ) -> Vec<DailyWeather> {
        (0..n)
            .rev()
            .map(|back| make(end - Duration::days(back)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn trapezoid_shape() {
        assert_eq!(trapezoid(10.0, 18.0, 25.0, 30.0, 35.0), 0.0);
        assert!((trapezoid(21.5, 18.0, 25.0, 30.0, 35.0) - 0.5).abs() < 1e-9);
        assert_eq!(trapezoid(27.0, 18.0, 25.0, 30.0, 35.0), 1.0);
        assert!((trapezoid(32.5, 18.0, 25.0, 30.0, 35.0) - 0.5).abs() < 1e-9);
        assert_eq!(trapezoid(40.0, 18.0, 25.0, 30.0, 35.0), 0.0);
    }

    #[test]
    fn trailing_mean_skips_partial_days_but_keeps_current() {
        let mut series = run_of(date(9, 20), 5, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        series[2].temp_mean_c = 100.0;
        series[2].hours_covered = 3.0;
        series[4].hours_covered = 3.0;
        series[4].temp_mean_c = 30.0;
        let mean = trailing_mean(&series, 4, 5, 3, |d| d.temp_mean_c).unwrap();
        assert!((mean - 22.5).abs() < 1e-9);
    }

    #[test]
    fn headline_falls_back_to_last_complete_day() {
        let today = date(9, 20);
        let mut series = run_of(today, 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        series[2].hours_covered = 4.0;
        let scale = RiskScale {
            min: 0.0,
            max: 100.0,
            unit: "%".into(),
            moderate_at: 25.0,
            high_at: 50.0,
            severe_at: 75.0,
        };
        let scored = score_window(&series, today, &scale, |i| Some(i as f64)).unwrap();
        assert_eq!(scored.headline_idx, 1);
        assert_eq!(scored.daily.len(), 3);
        assert!(scored.daily[2].partial);
    }

    #[test]
    fn assess_all_covers_every_disease_with_full_data() {
        let today = date(7, 20);
        let series = run_of(today, 10, |d| day(d, 26.0, 21.0, 32.0, 88.0));
        let risks = assess_all(&series, today, &DiseaseContext::default());
        assert_eq!(risks.len(), 5);
    }

    #[test]
    fn assess_all_is_empty_without_data() {
        assert!(assess_all(&[], date(7, 20), &DiseaseContext::default()).is_empty());
    }
}
