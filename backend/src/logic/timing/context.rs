use super::climatology::FREEZE_F;
use super::series::SoilPoint;
use crate::datasources::weather::ClimateDay;
use crate::models::timing::{SeasonContext, SoilSeriesDay};
use crate::models::DailyForecast;
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashMap;

/// Share of a comparison window that needs data, this year and in each past year.
const MIN_WINDOW_COVERAGE: f64 = 0.7;
/// Past years needed before an anomaly or a typical-year value is reported.
const MIN_BASELINE_YEARS: usize = 3;
/// Anomalies inside this band (°F) read as "near normal".
const NEAR_NORMAL_F: f64 = 1.0;
/// Forecast highs at or above this stress seedlings and volatilize some herbicides.
const HOT_DAY_F: f64 = 90.0;

/// Same month/day in another year (Feb 29 → Feb 28).
fn in_year(date: NaiveDate, year: i32) -> NaiveDate {
    date.with_year(year)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, 2, 28).unwrap_or(date))
}

fn window_mean(values: &HashMap<NaiveDate, f64>, end: NaiveDate, window_days: i64) -> Option<f64> {
    let found: Vec<f64> = (0..window_days)
        .filter_map(|back| values.get(&(end - Duration::days(back))).copied())
        .collect();
    let covered = found.len() as f64 / window_days as f64;
    (covered >= MIN_WINDOW_COVERAGE).then(|| found.iter().sum::<f64>() / found.len() as f64)
}

/// Mean over the `window_days` ending at `end`, minus the mean of the same calendar
/// window across `history_years`.
pub fn anomaly(
    values: &HashMap<NaiveDate, f64>,
    end: NaiveDate,
    window_days: i64,
    history_years: &[i32],
) -> Option<f64> {
    let current = window_mean(values, end, window_days)?;
    let past: Vec<f64> = history_years
        .iter()
        .filter_map(|&y| window_mean(values, in_year(end, y), window_days))
        .collect();
    (past.len() >= MIN_BASELINE_YEARS)
        .then(|| current - past.iter().sum::<f64>() / past.len() as f64)
}

fn describe(air: Option<f64>, soil: Option<f64>) -> Option<String> {
    let anomaly = soil.or(air)?;
    let subject = if soil.is_some() {
        "5 cm soil over the last week"
    } else {
        "Air temperature over the last 30 days"
    };
    Some(if anomaly.abs() < NEAR_NORMAL_F {
        format!("{subject} is running close to normal for this station.")
    } else if anomaly > 0.0 {
        format!(
            "{subject} is running {anomaly:.1}°F warmer than normal: spring windows tend to \
             open earlier, fall windows later."
        )
    } else {
        format!(
            "{subject} is running {:.1}°F cooler than normal: spring windows tend to open \
             later, fall windows earlier.",
            anomaly.abs()
        )
    })
}

fn forecast_alerts(forecast: &[DailyForecast]) -> Vec<String> {
    let mut alerts = Vec::new();
    if let Some(day) = forecast.iter().find(|d| d.low_temp_f <= FREEZE_F) {
        alerts.push(format!(
            "Freeze in the forecast: low of {:.0}°F on {}.",
            day.low_temp_f,
            day.date.format("%b %-d")
        ));
    }
    let hot = forecast
        .iter()
        .filter(|d| d.high_temp_f >= HOT_DAY_F)
        .count();
    if hot > 0 {
        alerts.push(format!(
            "{hot} forecast day(s) at {HOT_DAY_F:.0}°F or hotter — new seed will need extra water."
        ));
    }
    alerts
}

pub fn build_context(
    days: &[ClimateDay],
    history_years: &[i32],
    forecast: &[DailyForecast],
) -> SeasonContext {
    let series = |pick: fn(&ClimateDay) -> Option<f64>| -> HashMap<NaiveDate, f64> {
        days.iter()
            .filter_map(|d| Some((d.date, pick(d)?)))
            .collect()
    };
    let latest = |pick: fn(&ClimateDay) -> Option<f64>| {
        days.iter()
            .rev()
            .find(|d| pick(d).is_some())
            .map(|d| d.date)
    };

    let air = latest(|d| d.air_avg_f)
        .and_then(|end| anomaly(&series(|d| d.air_avg_f), end, 30, history_years));
    let soil = latest(|d| d.soil_temp_5_f)
        .and_then(|end| anomaly(&series(|d| d.soil_temp_5_f), end, 7, history_years));

    SeasonContext {
        air_anomaly_30d_f: air,
        soil_anomaly_7d_f: soil,
        summary: describe(air, soil),
        forecast_alerts: forecast_alerts(forecast),
    }
}

/// The chart: every day of this calendar year with the smoothed soil temperature for
/// this year (observed, then forecast), last year, and the mean of the history years.
pub fn build_series(
    smoothed: &[SoilPoint],
    today: NaiveDate,
    history_years: &[i32],
) -> Vec<SoilSeriesDay> {
    let by_date: HashMap<NaiveDate, &SoilPoint> = smoothed.iter().map(|p| (p.date, p)).collect();
    let year = today.year();
    let round = |v: f64| (v * 10.0).round() / 10.0;

    NaiveDate::from_ymd_opt(year, 1, 1)
        .into_iter()
        .flat_map(|jan1| jan1.iter_days())
        .take_while(|d| d.year() == year)
        .map(|date| {
            let point = by_date.get(&date);
            let past: Vec<f64> = history_years
                .iter()
                .filter_map(|&y| by_date.get(&in_year(date, y)).map(|p| p.temp_f))
                .collect();
            SoilSeriesDay {
                date,
                this_year: point.filter(|p| !p.is_forecast).map(|p| round(p.temp_f)),
                forecast: point.filter(|p| p.is_forecast).map(|p| round(p.temp_f)),
                last_year: by_date
                    .get(&in_year(date, year - 1))
                    .map(|p| round(p.temp_f)),
                typical: (past.len() >= MIN_BASELINE_YEARS)
                    .then(|| round(past.iter().sum::<f64>() / past.len() as f64)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// Every September day of each year at `base + per_year[year]`.
    fn septembers(temps: &[(i32, f64)]) -> HashMap<NaiveDate, f64> {
        temps
            .iter()
            .flat_map(|&(year, temp)| (1..=30).map(move |day| (date(year, 9, day), temp)))
            .collect()
    }

    #[test]
    fn anomaly_compares_to_the_same_dates_in_past_years() {
        let values = septembers(&[(2023, 64.0), (2024, 66.0), (2025, 65.0), (2026, 68.0)]);
        let result = anomaly(&values, date(2026, 9, 20), 7, &[2023, 2024, 2025]).unwrap();
        assert!((result - 3.0).abs() < 1e-9);
    }

    #[test]
    fn anomaly_needs_three_baseline_years_and_current_coverage() {
        let values = septembers(&[(2024, 66.0), (2025, 65.0), (2026, 68.0)]);
        assert_eq!(anomaly(&values, date(2026, 9, 20), 7, &[2024, 2025]), None);
        // Only 3 of the 7 days ending Sep 3 exist.
        assert_eq!(
            anomaly(&values, date(2026, 9, 3), 7, &[2023, 2024, 2025]),
            None
        );
    }

    #[test]
    fn summary_prefers_soil_and_names_the_direction() {
        let warm = describe(Some(-2.0), Some(2.4)).unwrap();
        assert!(warm.starts_with("5 cm soil") && warm.contains("2.4°F warmer"));
        assert!(describe(Some(-2.0), None).unwrap().contains("2.0°F cooler"));
        assert!(describe(Some(0.4), None)
            .unwrap()
            .contains("close to normal"));
        assert_eq!(describe(None, None), None);
    }

    #[test]
    fn series_separates_observed_forecast_and_history() {
        let point = |d: NaiveDate, temp_f: f64, is_forecast: bool| SoilPoint {
            date: d,
            temp_f,
            is_forecast,
        };
        let smoothed = vec![
            point(date(2023, 9, 20), 64.0, false),
            point(date(2024, 9, 20), 66.0, false),
            point(date(2025, 9, 20), 65.04, false),
            point(date(2026, 9, 20), 68.0, false),
            point(date(2026, 9, 21), 67.0, true),
        ];
        let series = build_series(&smoothed, date(2026, 9, 20), &[2023, 2024, 2025]);
        assert_eq!(series.len(), 365);

        let sep20 = series.iter().find(|d| d.date == date(2026, 9, 20)).unwrap();
        assert_eq!(sep20.this_year, Some(68.0));
        assert_eq!(sep20.forecast, None);
        assert_eq!(sep20.last_year, Some(65.0));
        assert_eq!(sep20.typical, Some(65.0));

        let sep21 = series.iter().find(|d| d.date == date(2026, 9, 21)).unwrap();
        assert_eq!(sep21.this_year, None);
        assert_eq!(sep21.forecast, Some(67.0));
        assert_eq!(sep21.typical, None);
    }
}
