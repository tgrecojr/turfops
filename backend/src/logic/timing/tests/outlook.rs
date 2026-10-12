//! The season's soil series: observed days, the 5 cm outlook, and smoothing over both.

use super::*;

/// Air that jumps around from day to day (so only lag 0 explains the soil), with soil a
/// fixed 5°F above it.
fn noisy_station(start: NaiveDate, through: NaiveDate) -> Vec<ClimateDay> {
    start
        .iter_days()
        .take_while(|d| *d <= through)
        .enumerate()
        .map(|(i, d)| {
            let air = 60.0 + 10.0 * (i as f64 * 1.7).sin();
            ClimateDay {
                date: d,
                soil_temp_5_f: Some(air + 5.0),
                air_min_f: Some(air - 8.0),
                air_avg_f: Some(air),
                gdd50: Some((air - 50.0).max(0.0)),
            }
        })
        .collect()
}

fn forecast_at(from: NaiveDate, temps: &[f64]) -> Vec<DailyForecast> {
    temps
        .iter()
        .enumerate()
        .map(|(i, &avg)| forecast_day(from + Duration::days(i as i64), avg))
        .collect()
}

// @spec TIME-ENG-006
#[test]
fn forecast_soil_is_appended_after_the_station_and_smoothed_with_it() {
    let last = date(2026, 9, 19);
    let mut days = station(last, 0.0);
    // A hole in the station record: Sep 1–2 missing.
    for day in days
        .iter_mut()
        .filter(|d| d.date == date(2026, 9, 1) || d.date == date(2026, 9, 2))
    {
        day.soil_temp_5_f = None;
    }
    let forecast = forecast_at(date(2026, 9, 20), &[50.0, 50.0, 50.0]);
    let raw: Vec<SoilPoint> = observed_points(&days)
        .into_iter()
        .chain(forecast_points(&days, &forecast))
        .collect();
    let data = SeasonData::build(&days, &forecast, 2026);

    // Forecast days come after the last observed one, flagged as forecast.
    let outlook: Vec<&SoilPoint> = data.smoothed.iter().filter(|p| p.is_forecast).collect();
    assert_eq!(outlook.first().unwrap().date, date(2026, 9, 20));
    assert!(data
        .smoothed
        .iter()
        .filter(|p| !p.is_forecast)
        .all(|p| p.date <= last));

    // The first forecast day's mean is the trailing 5 days of observed + forecast soil.
    let window: Vec<f64> = raw
        .iter()
        .filter(|p| p.date > date(2026, 9, 15) && p.date <= date(2026, 9, 20))
        .map(|p| p.temp_f)
        .collect();
    assert_eq!(window.len(), 5);
    let expected = window.iter().sum::<f64>() / 5.0;
    assert!((outlook[0].temp_f - expected).abs() < 1e-9);

    // Sep 3–5 have only 3 of their 5 days: a gap. Sep 6 has 4 of 5: a value.
    let has = |d: NaiveDate| data.smoothed.iter().any(|p| p.date == d);
    for day in 1..=5 {
        assert!(!has(date(2026, 9, day)), "Sep {day}");
    }
    assert!(has(date(2026, 8, 31)) && has(date(2026, 9, 6)));
}

// @spec TIME-ENG-007
#[test]
fn the_outlook_continues_from_the_last_soil_observation() {
    let last = date(2026, 9, 19);
    let mut days = noisy_station(date(2026, 6, 1), last);
    // The last observed day reads 3°F above the station's usual soil-over-air offset.
    days.last_mut().unwrap().soil_temp_5_f = Some(days.last().unwrap().air_avg_f.unwrap() + 8.0);
    let forecast = forecast_at(date(2026, 9, 20), &[55.0, 58.0, 61.0]);

    let outlook = forecast_points(&days, &forecast);
    let dates: Vec<NaiveDate> = outlook.iter().map(|p| p.date).collect();
    assert_eq!(
        dates,
        vec![date(2026, 9, 20), date(2026, 9, 21), date(2026, 9, 22)],
        "only days after the last observation"
    );
    // Air + 5°F from the fit, shifted by the ≈3°F error on the last observed day.
    for (point, air) in outlook.iter().zip([55.0, 58.0, 61.0]) {
        assert!(point.is_forecast);
        assert!(
            (point.temp_f - (air + 8.0)).abs() < 0.6,
            "{} → {}",
            point.date,
            point.temp_f
        );
    }
}

// @spec TIME-ENG-007
#[test]
fn the_outlook_fits_on_the_last_30_days_and_prefers_station_air() {
    let last = date(2026, 9, 19);
    let days = noisy_station(date(2026, 6, 1), last);
    let forecast = forecast_at(date(2026, 9, 18), &[55.0, 57.0, 59.0, 61.0]);
    let baseline = forecast_points(&days, &forecast);
    assert_eq!(baseline.len(), 2);

    // Soil older than the 30-day training window has nothing to do with air: no effect.
    let mut old_noise = days.clone();
    for day in old_noise
        .iter_mut()
        .filter(|d| d.date < last - Duration::days(30))
    {
        day.soil_temp_5_f = Some(100.0 - day.soil_temp_5_f.unwrap());
    }
    assert_eq!(forecast_points(&old_noise, &forecast), baseline);

    // On Sep 18–19 the station has air of its own: the forecast's value is not used.
    let mut overlapping = forecast.clone();
    overlapping[0] = forecast_day(date(2026, 9, 18), 20.0);
    overlapping[1] = forecast_day(date(2026, 9, 19), 95.0);
    assert_eq!(forecast_points(&days, &overlapping), baseline);
}

// @spec TIME-ENG-008
#[test]
fn no_observation_forecast_or_fit_means_no_outlook() {
    let last = date(2026, 9, 19);
    let days = noisy_station(date(2026, 6, 1), last);
    let forecast = forecast_at(date(2026, 9, 20), &[55.0, 58.0]);
    assert!(!forecast_points(&days, &forecast).is_empty());

    assert!(forecast_points(&days, &[]).is_empty(), "no forecast");

    let mut no_soil = days.clone();
    for day in &mut no_soil {
        day.soil_temp_5_f = None;
    }
    assert!(forecast_points(&no_soil, &forecast).is_empty(), "no soil");

    // Too few paired days for a regression.
    let short = noisy_station(date(2026, 9, 15), last);
    assert!(forecast_points(&short, &forecast).is_empty(), "no fit");

    let data = SeasonData::build(&short, &forecast, 2026);
    assert!(data.smoothed.iter().all(|p| !p.is_forecast));
}
