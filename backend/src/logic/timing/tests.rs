//! Scenario tests over a synthetic station: every year follows the same cosine soil
//! curve (peak 77°F on Jul 19, trough 33°F in mid-January), with air tracking soil.
//! On that curve the smoothed 5 cm soil cools through 70°F about Sep 7, 65°F about
//! Sep 23 and 55°F about Oct 20; warms through 45°F about Mar 24, 50°F about Apr 7 and
//! 55°F about Apr 21; and the first fall freeze lands about Nov 5.

use super::*;
use crate::models::timing::{DateSource, TimingWindow, WindowId};
use crate::models::ApplicationType;

mod confirmation;

pub(super) fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn soil_curve(d: NaiveDate) -> f64 {
    let phase = 2.0 * std::f64::consts::PI * (d.ordinal() as f64 - 200.0) / 365.0;
    55.0 + 22.0 * phase.cos()
}

/// Daily record from 2020 through `through`; 2026 runs `bias_2026` °F off the curve.
pub(super) fn station(through: NaiveDate, bias_2026: f64) -> Vec<ClimateDay> {
    date(2020, 1, 1)
        .iter_days()
        .take_while(|d| *d <= through)
        .map(|d| {
            let soil = soil_curve(d) + if d.year() == 2026 { bias_2026 } else { 0.0 };
            let air_avg = soil - 5.0;
            ClimateDay {
                date: d,
                soil_temp_5_f: Some(soil),
                air_min_f: Some(soil - 16.5),
                air_avg_f: Some(air_avg),
                gdd50: Some((air_avg - 50.0).max(0.0)),
            }
        })
        .collect()
}

fn run(today: NaiveDate, days: &[ClimateDay], history: &[Application]) -> Assessment {
    assess(&Inputs {
        today,
        days,
        forecast: &[],
        grass: GrassType::TallFescue,
        history,
    })
}

fn window(assessment: &Assessment, id: WindowId) -> &TimingWindow {
    assessment.windows.iter().find(|w| w.id == id).unwrap()
}

pub(super) fn applied(kind: ApplicationType, on: NaiveDate) -> Application {
    Application {
        id: None,
        lawn_profile_id: 1,
        application_type: kind,
        product_name: None,
        application_date: on,
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        weather_snapshot: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: chrono::Utc::now(),
    }
}

pub(super) fn within(actual: Option<NaiveDate>, expected: NaiveDate, days: i64) -> bool {
    actual.is_some_and(|d| (d - expected).num_days().abs() <= days)
}

#[test]
fn mid_september_has_both_fall_windows_in_their_ideal_stretch() {
    let today = date(2026, 9, 20);
    let result = run(today, &station(date(2026, 9, 19), 0.0), &[]);

    assert_eq!(
        result.history_years,
        vec![2020, 2021, 2022, 2023, 2024, 2025]
    );
    assert!(result.data_fresh);
    assert_eq!(result.soil.as_of, Some(date(2026, 9, 19)));
    assert!(within(
        result.freeze.first_fall.as_ref().map(|s| s.median),
        date(2026, 11, 5),
        2
    ));

    let seeding = window(&result, WindowId::FallSeeding);
    assert_eq!(seeding.state, WindowState::Ideal);
    assert_eq!(seeding.opens.date, Some(date(2026, 8, 15))); // already ≤75°F at scan start
    assert_eq!(seeding.opens.source, DateSource::Observed);
    // Comfortable cutoff: 45 days before the typical first freeze.
    assert!(within(
        seeding.ideal_until.as_ref().unwrap().date,
        date(2026, 9, 21),
        2
    ));
    // Last chance is the freeze buffer (≈Oct 6), which beats soil reaching 55°F (≈Oct 20).
    assert!(within(seeding.closes.date, date(2026, 10, 6), 2));

    let pre_em = window(&result, WindowId::FallPreEmergent);
    assert_eq!(pre_em.state, WindowState::Ideal);
    assert!(within(pre_em.opens.date, date(2026, 9, 7), 2));
    assert!(within(
        pre_em.opens.typical.as_ref().map(|s| s.median),
        date(2026, 9, 7),
        2
    ));
}

#[test]
fn spring_windows_roll_to_next_year_after_summer() {
    let result = run(date(2026, 9, 20), &station(date(2026, 9, 19), 0.0), &[]);
    let spring = window(&result, WindowId::SpringPreEmergent);
    assert_eq!(spring.season_year, 2027);
    assert_eq!(spring.state, WindowState::NotYet);
    assert_eq!(spring.opens.source, DateSource::Typical);
    assert!(within(spring.opens.date, date(2027, 3, 24), 2));

    let dormant = window(&result, WindowId::DormantSeeding);
    assert_eq!(dormant.season_year, 2026);
    assert_eq!(dormant.state, WindowState::NotYet);
    assert!(dormant.closes.typical.as_ref().unwrap().median > date(2027, 1, 1));
}

#[test]
fn fall_seeding_moves_from_closing_to_closed() {
    let closing = run(date(2026, 9, 28), &station(date(2026, 9, 27), 0.0), &[]);
    let seeding = window(&closing, WindowId::FallSeeding);
    assert_eq!(seeding.state, WindowState::Closing);
    assert!(seeding
        .headline
        .starts_with("Closing — last chance around Oct"));

    let closed = run(date(2026, 10, 12), &station(date(2026, 10, 11), 0.0), &[]);
    assert_eq!(
        window(&closed, WindowId::FallSeeding).state,
        WindowState::Closed
    );
    // Pre-emergent runs on soil alone and is merely late at this point.
    assert_eq!(
        window(&closed, WindowId::FallPreEmergent).state,
        WindowState::Closing
    );
}

#[test]
fn logged_overseeding_completes_seeding_and_rules_out_fall_pre_emergent() {
    let history = [applied(ApplicationType::Overseed, date(2026, 9, 12))];
    let result = run(
        date(2026, 9, 20),
        &station(date(2026, 9, 19), 0.0),
        &history,
    );

    let seeding = window(&result, WindowId::FallSeeding);
    assert_eq!(seeding.state, WindowState::Done);
    assert_eq!(seeding.headline, "Done — logged Sep 12");

    let pre_em = window(&result, WindowId::FallPreEmergent);
    assert_eq!(pre_em.state, WindowState::Blocked);
    assert!(pre_em.conflict.as_ref().unwrap().contains("Sep 12"));
    assert!(pre_em.guidance.is_empty());
}

#[test]
fn warm_fall_is_reported_as_overdue_rather_than_given_a_date() {
    // 2026 runs 6°F warm: soil is still above 70°F a week past the typical crossing.
    let today = date(2026, 9, 14);
    let result = run(today, &station(date(2026, 9, 13), 6.0), &[]);
    let pre_em = window(&result, WindowId::FallPreEmergent);
    assert_eq!(pre_em.state, WindowState::NotYet);
    assert_eq!(pre_em.opens.date, None);
    assert!(!pre_em.opens.passed);
    assert!(pre_em.headline.contains("later than usual"));
}

#[test]
fn stale_station_data_falls_back_to_the_calendar() {
    let result = run(date(2026, 9, 20), &station(date(2026, 8, 1), 0.0), &[]);
    assert!(!result.data_fresh);
    let pre_em = window(&result, WindowId::FallPreEmergent);
    assert_eq!(pre_em.opens.source, DateSource::Typical);
    assert!(pre_em.opens.passed);
    assert_eq!(pre_em.state, WindowState::Ideal);
}

#[test]
fn spring_pre_emergent_walks_through_its_stages() {
    let state_on = |month: u32, day: u32| {
        let today = date(2026, month, day);
        let result = run(today, &station(today.pred_opt().unwrap(), 0.0), &[]);
        window(&result, WindowId::SpringPreEmergent).state
    };
    assert_eq!(state_on(3, 1), WindowState::NotYet);
    assert_eq!(state_on(3, 20), WindowState::OpeningSoon);
    assert_eq!(state_on(3, 30), WindowState::Open); // 45–50°F: early but worthwhile
    assert_eq!(state_on(4, 12), WindowState::Ideal); // 50–55°F
    assert_eq!(state_on(5, 1), WindowState::Closed); // past 55°F sustained
}

#[test]
fn forecast_cooling_flags_the_window_as_opening_soon() {
    let today = date(2026, 9, 3);
    let days = station(date(2026, 9, 2), 0.0);
    // Air tracks soil − 5°F on this station, so a 58°F forecast means ≈63°F soil.
    let forecast: Vec<DailyForecast> = (0..5)
        .map(|i| DailyForecast {
            date: today + Duration::days(i),
            high_temp_f: 66.0,
            low_temp_f: 50.0,
            avg_humidity: 60.0,
            total_precipitation_mm: 0.0,
            max_precipitation_prob: 0.0,
            dominant_condition: crate::models::WeatherCondition::Clear,
            avg_wind_speed_mph: 5.0,
            max_wind_gust_mph: None,
            hours_covered: 24,
        })
        .collect();
    let result = assess(&Inputs {
        today,
        days: &days,
        forecast: &forecast,
        grass: GrassType::TallFescue,
        history: &[],
    });

    let pre_em = window(&result, WindowId::FallPreEmergent);
    assert_eq!(pre_em.opens.source, DateSource::Forecast);
    assert!(!pre_em.opens.passed);
    assert_eq!(pre_em.state, WindowState::OpeningSoon);
    assert!(result.smoothed.iter().any(|p| p.is_forecast));
}

fn forecast_day(date: NaiveDate, avg_f: f64) -> DailyForecast {
    DailyForecast {
        date,
        high_temp_f: avg_f + 8.0,
        low_temp_f: avg_f - 8.0,
        avg_humidity: 60.0,
        total_precipitation_mm: 0.0,
        max_precipitation_prob: 0.0,
        dominant_condition: crate::models::WeatherCondition::Clear,
        avg_wind_speed_mph: 5.0,
        max_wind_gust_mph: None,
        hours_covered: 24,
    }
}

#[test]
fn soil_outlook_is_continuous_across_the_hole_between_station_air_and_forecast() {
    // Soil follows air from 4 days earlier. Station soil ends Sep 19 but its air ends
    // Sep 16, and the forecast only starts Sep 21 — so the drivers for most outlook days
    // sit in a hole that has to be bridged.
    let mut days = station(date(2026, 9, 19), 0.0);
    for day in days.iter_mut() {
        day.air_avg_f = Some(soil_curve(day.date + Duration::days(4)) - 5.0);
    }
    for day in days.iter_mut().filter(|d| d.date > date(2026, 9, 16)) {
        day.air_avg_f = None;
    }
    let forecast: Vec<DailyForecast> = (21..=26)
        .map(|d| forecast_day(date(2026, 9, d), soil_curve(date(2026, 9, d + 4)) - 5.0))
        .collect();

    let result = assess(&Inputs {
        today: date(2026, 9, 21),
        days: &days,
        forecast: &forecast,
        grass: GrassType::TallFescue,
        history: &[],
    });

    let outlook: Vec<&SoilPoint> = result.smoothed.iter().filter(|p| p.is_forecast).collect();
    let dates: Vec<NaiveDate> = outlook.iter().map(|p| p.date).collect();
    let expected: Vec<NaiveDate> = (20..=26).map(|d| date(2026, 9, d)).collect();
    assert_eq!(dates, expected);

    // It continues from the last observation rather than jumping, and keeps cooling.
    let last_observed = result.smoothed.iter().rfind(|p| !p.is_forecast).unwrap();
    assert!((outlook[0].temp_f - last_observed.temp_f).abs() < 1.0);
    assert!(outlook.last().unwrap().temp_f < last_observed.temp_f);
}
