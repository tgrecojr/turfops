//! Gold daily contracts: the 10 cm and air/soil series and the GDD rows.

use super::fixtures::{self, GoldRow};
use chrono::{NaiveDate, TimeZone, Utc};

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

/// Rows written out of date order: a full day, a sparse day, days missing a value.
fn days() -> Vec<GoldRow> {
    vec![
        GoldRow {
            soil_10_f: Some(61.0),
            air_avg_f: Some(66.0),
            ..GoldRow::day(d(9, 4))
        },
        GoldRow {
            soil_10_f: Some(60.0),
            air_avg_f: Some(65.0),
            ..GoldRow::day(d(9, 1))
        },
        GoldRow {
            hours_observed: 11,
            ..GoldRow::day(d(9, 2))
        },
        GoldRow {
            air_avg_f: None,
            soil_10_f: Some(59.0),
            ..GoldRow::day(d(9, 3))
        },
        GoldRow {
            soil_10_f: None,
            ..GoldRow::day(d(9, 5))
        },
    ]
}

// @spec WX-SRC-LAKE-007
#[tokio::test]
async fn gold_daily_series_keep_complete_days_in_date_order() {
    let gold = fixtures::gold(&days());
    let client = fixtures::client("unused.parquet", &gold.path);
    let (start, end) = (
        Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2026, 9, 30, 0, 0, 0).unwrap(),
    );

    let soil: Vec<(NaiveDate, f64)> = client
        .fetch_daily_soil_temp_averages(start, end)
        .await
        .unwrap()
        .into_iter()
        .map(|s| (s.date, s.avg_temp_f))
        .collect();
    // Sep 2 has 11 hours; Sep 5 has no 10 cm mean.
    assert_eq!(
        soil,
        vec![(d(9, 1), 60.0), (d(9, 3), 59.0), (d(9, 4), 61.0)]
    );

    let pairs = client
        .fetch_daily_paired_averages(start, end)
        .await
        .unwrap();
    // Sep 3 also lacks air.
    assert_eq!(pairs, vec![(d(9, 1), 65.0, 60.0), (d(9, 4), 66.0, 61.0)]);
}

// @spec WX-SRC-LAKE-008
#[tokio::test]
async fn gdd_rows_need_gdd50_and_the_year_to_date_sums_from_jan_1() {
    let gold = fixtures::gold(&[
        GoldRow {
            gdd50: Some(7.0),
            ..GoldRow::day(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
        },
        GoldRow {
            gdd50: Some(1.5),
            hours_observed: 6,
            ..GoldRow::day(d(1, 1))
        },
        GoldRow {
            gdd50: None,
            ..GoldRow::day(d(5, 1))
        },
        GoldRow {
            gdd50: Some(12.0),
            ..GoldRow::day(d(5, 2))
        },
        GoldRow {
            gdd50: Some(30.0),
            ..GoldRow::day(d(9, 1))
        },
    ]);
    let client = fixtures::client("unused.parquet", &gold.path);

    let rows = client.fetch_daily_gdd(d(1, 1), d(6, 30)).await.unwrap();
    let dates: Vec<NaiveDate> = rows.iter().map(|r| r.0).collect();
    assert_eq!(dates, vec![d(1, 1), d(5, 2)], "in range, with gdd50");
    assert_eq!(rows[1], (d(5, 2), 70.0, 50.0, 12.0));

    assert_eq!(client.fetch_gdd_ytd(d(6, 30)).await.unwrap(), Some(13.5));
    let empty = fixtures::client("unused.parquet", &gold.path)
        .fetch_gdd_ytd(NaiveDate::from_ymd_opt(2024, 6, 30).unwrap())
        .await
        .unwrap();
    assert_eq!(empty, None, "no rows from Jan 1 of that year");
}
