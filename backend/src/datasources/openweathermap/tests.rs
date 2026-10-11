use super::*;

fn eastern() -> Clock {
    Clock::Zone(chrono_tz::America::New_York)
}

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
    let client = OpenWeatherMapClient::new(sample_config(), eastern());
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

// @spec WX-SRC-OWM-002
#[test]
fn days_are_local_and_partial_days_are_marked() {
    let client = OpenWeatherMapClient::new(sample_config(), eastern());
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

    let days = client.aggregate_daily(&hourly);
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

// @spec WX-SRC-OWM-002
#[test]
fn days_follow_daylight_saving_not_a_fixed_offset() {
    // DST ends 2026-11-01 06:00 UTC. 04:30 UTC on Nov 2 is 11:30 pm EST on Nov 1; a fixed
    // EDT offset (what OWM's `city.timezone` reported at fetch time) would call it Nov 2.
    let client = OpenWeatherMapClient::new(sample_config(), eastern());
    let days = client.aggregate_daily(&[
        point("2026-11-01T22:00:00Z", 60.0), // 5 pm EST Nov 1
        point("2026-11-02T04:30:00Z", 50.0), // 11:30 pm EST Nov 1
    ]);
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].date, NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
}

const FAKE_KEY: &str = "owm-secret-key-0123456789";

/// A client whose requests fail at the network level (nothing listens on port 1), the
/// failure whose reqwest error carries the request URL.
fn unreachable_client() -> OpenWeatherMapClient {
    let mut config = sample_config();
    config.api_key = FAKE_KEY.to_string();
    OpenWeatherMapClient::new(config, eastern()).with_base_url("http://127.0.0.1:1")
}

// @spec WX-SRC-OWM-005
#[tokio::test]
async fn a_failed_forecast_fetch_never_carries_the_api_key() {
    let err = unreachable_client().fetch_forecast().await.unwrap_err();
    let text = format!("{err} {err:?}");
    assert!(!text.contains(FAKE_KEY), "API key leaked: {text}");
}

// @spec WX-SRC-OWM-005
#[tokio::test]
async fn a_failed_connection_test_never_carries_the_api_key() {
    let err = unreachable_client().test_connection().await.unwrap_err();
    let text = format!("{err} {err:?}");
    assert!(!text.contains(FAKE_KEY), "API key leaked: {text}");
}
