//! The forecast's day and hour helpers.

use super::*;
use chrono::Duration;

fn day(date: NaiveDate, hours_covered: u32, high: f64) -> DailyForecast {
    DailyForecast {
        date,
        high_temp_f: high,
        low_temp_f: high - 20.0,
        avg_humidity: 50.0,
        total_precipitation_mm: 0.0,
        max_precipitation_prob: 0.0,
        dominant_condition: WeatherCondition::Clear,
        avg_wind_speed_mph: 0.0,
        max_wind_gust_mph: None,
        hours_covered,
    }
}

fn slot(hours_from_now: i64, precip_mm: f64, pop: f64) -> ForecastPoint {
    ForecastPoint {
        timestamp: Utc::now() + Duration::hours(hours_from_now),
        temp_f: 70.0,
        feels_like_f: 70.0,
        humidity_percent: 50.0,
        precipitation_mm: precip_mm,
        precipitation_prob: pop,
        wind_speed_mph: 0.0,
        wind_gust_mph: None,
        cloud_cover_percent: 0.0,
        weather_condition: WeatherCondition::Clear,
    }
}

fn forecast(hourly: Vec<ForecastPoint>, daily: Vec<DailyForecast>) -> WeatherForecast {
    WeatherForecast {
        fetched_at: Utc::now(),
        location: ForecastLocation {
            city: "Avondale".into(),
            country: "US".into(),
            latitude: 0.0,
            longitude: 0.0,
            utc_offset_seconds: 0,
        },
        hourly,
        daily_summary: daily,
    }
}

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, day).unwrap()
}

// @spec WX-SRC-OWM-003
#[test]
fn a_full_day_covers_at_least_18_hours_and_full_days_skips_the_rest() {
    let f = forecast(
        vec![],
        vec![
            day(d(1), 9, 70.0),
            day(d(2), 24, 80.0),
            day(d(3), 18, 75.0),
            day(d(4), 15, 90.0),
        ],
    );
    assert!(!f.daily_summary[3].is_full_day());
    assert!(f.daily_summary[2].is_full_day());
    let full: Vec<NaiveDate> = f.full_days().map(|d| d.date).collect();
    assert_eq!(full, vec![d(2), d(3)]);
}

// @spec WX-SRC-OWM-004
#[test]
fn next_hours_keeps_the_current_slot_and_stops_at_the_horizon() {
    let f = forecast(
        vec![slot(-1, 0.0, 0.0), slot(2, 0.0, 0.0), slot(5, 0.0, 0.0)],
        vec![],
    );
    assert_eq!(
        f.next_hours(chrono::Utc::now(), 3).len(),
        2,
        "the slot under way counts"
    );
    assert_eq!(f.next_hours(chrono::Utc::now(), 6).len(), 3);
}

// @spec WX-SRC-OWM-004
#[test]
fn rain_is_expected_on_enough_total_or_a_likely_slot() {
    let wet = forecast(
        vec![slot(-1, 2.0, 0.1), slot(2, 1.0, 0.1), slot(30, 9.0, 0.9)],
        vec![],
    );
    let rain = wet
        .rain_expected_within(chrono::Utc::now(), 24, 3.0)
        .expect("3 mm in 24 h");
    assert_eq!(rain.expected_mm, 3.0);
    assert!(
        wet.rain_expected_within(chrono::Utc::now(), 24, 3.5)
            .is_none(),
        "the 30 h slot is out"
    );

    let likely = forecast(vec![slot(1, 0.0, 0.5)], vec![]);
    assert!(
        likely
            .rain_expected_within(chrono::Utc::now(), 24, 5.0)
            .is_some(),
        "pop ≥ 0.5"
    );
    let unlikely = forecast(vec![slot(1, 0.0, 0.49)], vec![]);
    assert!(unlikely
        .rain_expected_within(chrono::Utc::now(), 24, 5.0)
        .is_none());
}

// @spec WX-SRC-OWM-004
#[test]
fn the_hottest_day_counts_partial_days_up_to_today_plus_n() {
    let f = forecast(
        vec![],
        vec![
            day(d(1), 6, 91.0),
            day(d(2), 24, 80.0),
            day(d(3), 24, 85.0),
            day(d(4), 24, 99.0),
        ],
    );
    assert_eq!(
        f.max_temp_next_days(d(1), 2),
        Some(91.0),
        "partial first day included"
    );
    assert_eq!(f.max_temp_next_days(d(1), 3), Some(99.0));
}
