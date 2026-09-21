use super::*;

fn sample_config() -> OpenWeatherMapConfig {
    OpenWeatherMapConfig {
        api_key: "test_key".to_string(),
        latitude: 39.8561,
        longitude: -75.7872,
        enabled: true,
    }
}

#[test]
fn client_creation() {
    let client = OpenWeatherMapClient::new(sample_config());
    assert!(client.config.enabled);
}

fn point(utc: &str, temp_f: f64) -> ForecastPoint {
    ForecastPoint {
        timestamp: utc.parse().unwrap(),
        temp_f,
        feels_like_f: temp_f,
        humidity_percent: 50.0,
        precipitation_mm: 0.0,
        precipitation_prob: 0.0,
        wind_speed_mph: 5.0,
        wind_gust_mph: None,
        cloud_cover_percent: 0.0,
        weather_condition: WeatherCondition::default(),
    }
}

#[test]
fn days_are_local_and_partial_days_are_marked() {
    let client = OpenWeatherMapClient::new(sample_config());
    const EDT: i32 = -4 * 3600;
    // Fetched mid-afternoon Sep 21 (EDT): two points left today, then a full Sep 22.
    let mut hourly = vec![
        point("2026-09-21T21:00:00Z", 78.0), // 5 pm Sep 21 local
        point("2026-09-22T00:00:00Z", 70.0), // 8 pm Sep 21 local — still the 21st
    ];
    for (i, temp) in [62.0, 58.0, 57.0, 66.0, 76.0, 80.0, 77.0, 69.0]
        .iter()
        .enumerate()
    {
        let hour = 3 + 3 * i as i64; // 03:00Z Sep 22 (11 pm local Sep 21)… 00:00Z Sep 23
        let ts: DateTime<Utc> = "2026-09-22T00:00:00Z".parse().unwrap();
        let mut p = point("2026-09-22T00:00:00Z", *temp);
        p.timestamp = ts + chrono::Duration::hours(hour);
        hourly.push(p);
    }

    let days = client.aggregate_daily(&hourly, EDT);
    let sep21 = &days[0];
    assert_eq!(sep21.date, NaiveDate::from_ymd_opt(2026, 9, 21).unwrap());
    assert_eq!(sep21.hours_covered, 9); // 5 pm, 8 pm, 11 pm
    assert!(!sep21.is_full_day());

    let sep22 = &days[1];
    assert_eq!(sep22.date, NaiveDate::from_ymd_opt(2026, 9, 22).unwrap());
    // The afternoon high and the dawn low of the same local day land together.
    assert_eq!(sep22.high_temp_f, 80.0);
    assert_eq!(sep22.low_temp_f, 57.0);
    assert_eq!(sep22.hours_covered, 21);
    assert!(sep22.is_full_day());
}
