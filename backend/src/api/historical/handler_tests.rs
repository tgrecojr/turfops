//! `GET /historical` over test parquet: the hourly series, the GDD series and errors.

use super::*;
use crate::datasources::weather::fixtures::{self, GoldRow, SilverRow};
use crate::logic::data_sync::test_support::{app_state, eastern};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Timelike;

async fn historical(state: AppState, range: Option<&str>) -> Result<HistoricalData, StatusCode> {
    let query = HistoricalQuery {
        range: range.map(str::to_string),
    };
    get_historical(State(state), Query(query))
        .await
        .map(|Json(h)| h)
        .map_err(|e| e.into_response().status())
}

fn values(points: &[TimeSeriesPoint]) -> Vec<f64> {
    points.iter().map(|p| p.value).collect()
}

// @spec WX-VIEW-API-002
#[tokio::test]
async fn the_default_range_is_the_last_7_days_of_hours_ascending_without_zero_fill() {
    let now = Utc::now().naive_utc().with_nanosecond(0).unwrap();
    let hour = |h: i64, soil: Option<f64>, rain: f64| SilverRow {
        at: now - Duration::hours(h),
        soil_temp_10: soil,
        soil_moisture_10: Some(0.3),
        air_temp_c: Some(10.0),
        rh_pct: Some(60.0),
        precip_mm: Some(rain),
        ..Default::default()
    };
    let silver = fixtures::silver(&[
        hour(1, Some(20.0), 1.0),
        hour(2, None, -1.0),
        hour(3, Some(10.0), 0.0),
        hour(24 * 8, Some(30.0), 9.0),
    ]);
    let gold = fixtures::gold(&[GoldRow::day(eastern().today())]);
    let state = app_state(Some(fixtures::client(&silver.path, &gold.path)), None);

    let h = historical(state, None).await.unwrap();
    assert_eq!(h.range, "7d");
    let times: Vec<_> = h.ambient_temp_f.iter().map(|p| p.timestamp).collect();
    let expected: Vec<_> = [3, 2, 1]
        .map(|n| (now - Duration::hours(n)).and_utc())
        .to_vec();
    assert_eq!(times, expected, "ascending, the 8-day-old hour excluded");
    assert_eq!(
        values(&h.soil_temp_10_f),
        vec![50.0, 68.0],
        "the missing hour is left out"
    );
    assert_eq!(values(&h.ambient_temp_f), vec![50.0, 50.0, 50.0]);
    assert_eq!(values(&h.humidity_percent), vec![60.0, 60.0, 60.0]);
    assert_eq!(values(&h.soil_moisture_10), vec![0.3, 0.3, 0.3]);
    assert_eq!(
        values(&h.precipitation_mm),
        vec![0.0, 1.0],
        "negative rain is dropped"
    );
}

// @spec WX-VIEW-API-004
#[tokio::test]
async fn the_gdd_series_runs_from_jan_1_and_resets_each_new_year() {
    let start = eastern().local_date(Utc::now() - Duration::days(7));
    let jan1 = NaiveDate::from_ymd_opt(start.year(), 1, 1).unwrap();
    let today = eastern().today();
    let gdd_day = |day: NaiveDate, gdd: f64| GoldRow {
        gdd50: Some(gdd),
        ..GoldRow::day(day)
    };
    let mut rows = vec![gdd_day(jan1 - Duration::days(1), 1000.0)];
    let before_range = if jan1 < start { 100.0 } else { 0.0 };
    if jan1 < start {
        rows.push(gdd_day(jan1, 100.0));
    }
    rows.push(gdd_day(start, 5.0));
    rows.push(gdd_day(today, 5.0));
    let gold = fixtures::gold(&rows);
    let silver = fixtures::silver(&[SilverRow::default()]);
    let state = app_state(Some(fixtures::client(&silver.path, &gold.path)), None);

    let h = historical(state, Some("7d")).await.unwrap();
    let first = &h.gdd_accumulation[0];
    assert_eq!(
        first.timestamp,
        start.and_hms_opt(12, 0, 0).unwrap().and_utc()
    );
    assert_eq!(
        first.value,
        before_range + 5.0,
        "from Jan 1, not the year before"
    );
    assert_eq!(
        h.gdd_accumulation.len(),
        2,
        "only days in the range are shown"
    );
    if today.year() == start.year() {
        assert_eq!(h.gdd_accumulation[1].value, before_range + 10.0);
    }

    // The accumulation the handler runs restarts at each Jan 1.
    let across = gdd::accumulate_daily_gdd(&[
        (
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            0.0,
            0.0,
            4.0,
        ),
        (NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), 0.0, 0.0, 1.0),
    ]);
    assert_eq!(across[1].cumulative_gdd_base50, 1.0);
}

// @spec WX-VIEW-API-006
#[tokio::test]
async fn another_range_is_a_400_and_no_lake_client_a_503() {
    let gold = fixtures::gold(&[GoldRow::day(eastern().today())]);
    let silver = fixtures::silver(&[SilverRow::default()]);
    let state = app_state(Some(fixtures::client(&silver.path, &gold.path)), None);
    for range in ["1y", "14d", ""] {
        assert_eq!(
            historical(state.clone(), Some(range)).await.unwrap_err(),
            StatusCode::BAD_REQUEST,
            "{range:?}"
        );
    }
    for range in ["7d", "30d", "90d"] {
        assert!(
            historical(state.clone(), Some(range)).await.is_ok(),
            "{range}"
        );
    }
    assert_eq!(
        historical(app_state(None, None), Some("7d"))
            .await
            .unwrap_err(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}
