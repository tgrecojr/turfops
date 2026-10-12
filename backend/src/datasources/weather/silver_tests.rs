//! Silver reads against small parquet files: in-place reads, binding, units and
//! sentinels, the per-sensor live reading, the 7-day summary and the local-day buckets.

use super::fixtures::{self, SilverRow};
use super::*;
use chrono::{NaiveDate, NaiveDateTime, Timelike};

fn ts(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
}

fn hour(at: NaiveDateTime, soil_10_c: Option<f64>) -> SilverRow {
    SilverRow {
        at,
        soil_temp_10: soil_10_c,
        air_temp_c: Some(10.0),
        rh_pct: Some(60.0),
        precip_mm: Some(0.0),
        ..Default::default()
    }
}

// @spec WX-SRC-LAKE-001
#[tokio::test]
async fn the_lake_is_read_in_place_so_a_rewritten_file_is_seen_on_the_next_read() {
    let at = ts("2026-09-20 12:00:00");
    let file = fixtures::silver(&[hour(at, Some(20.0))]);
    let client = fixtures::client(&file.path, "unused.parquet");
    let first = client.fetch_latest().await.unwrap().unwrap();
    assert_eq!(first.soil_temp_10_f, Some(68.0));

    let newer = fixtures::silver(&[hour(at, Some(25.0))]);
    std::fs::copy(&newer.path, &file.path).unwrap();
    let second = client.fetch_latest().await.unwrap().unwrap();
    assert_eq!(
        second.soil_temp_10_f,
        Some(77.0),
        "nothing cached between reads"
    );
}

// @spec WX-SRC-LAKE-002
#[tokio::test]
async fn a_quoted_path_is_escaped_and_the_station_is_a_bound_filter() {
    assert_eq!(
        WeatherLakeClient::parquet("/lake/o'brien.parquet"),
        "read_parquet('/lake/o''brien.parquet')"
    );
    let source = fixtures::silver(&[hour(ts("2026-09-20 12:00:00"), Some(20.0))]);
    let quoted = fixtures::TempParquet {
        path: fixtures::temp_path("o'brien"),
    };
    std::fs::copy(&source.path, &quoted.path).unwrap();

    let reading = fixtures::client(&quoted.path, "unused.parquet")
        .fetch_latest()
        .await
        .unwrap();
    assert_eq!(reading.unwrap().soil_temp_10_f, Some(68.0));

    let other_station = WeatherLakeClient::new(
        &crate::config::DataLakeConfig {
            silver_weather_path: quoted.path.clone(),
            gold_weather_path: "unused.parquet".into(),
        },
        9999,
    );
    assert!(other_station.fetch_latest().await.unwrap().is_none());
}

// @spec WX-SRC-LAKE-003
#[tokio::test]
async fn silver_is_converted_to_fahrenheit_with_sentinels_and_negatives_dropped() {
    let good = SilverRow {
        at: ts("2026-09-20 13:00:00"),
        soil_temp_10: Some(20.0),
        soil_temp_5: Some(-49.0),
        soil_moisture_10: Some(0.25),
        air_temp_c: Some(10.0),
        rh_pct: Some(55.0),
        precip_mm: Some(2.5),
        ..Default::default()
    };
    let bad = SilverRow {
        at: ts("2026-09-20 12:00:00"),
        soil_temp_10: Some(-50.0),
        soil_temp_5: Some(-9999.0),
        soil_moisture_10: Some(-0.1),
        air_temp_c: Some(-60.0),
        rh_pct: Some(-1.0),
        precip_mm: Some(-1.0),
        ..Default::default()
    };
    let file = fixtures::silver(&[good, bad]);
    let rows = fixtures::client(&file.path, "unused.parquet")
        .fetch_range(
            ts("2026-09-20 00:00:00").and_utc(),
            ts("2026-09-21 00:00:00").and_utc(),
        )
        .await
        .unwrap();

    let (newest, oldest) = (&rows[0], &rows[1]);
    assert_eq!(newest.soil_temp_10_f, Some(68.0));
    assert!(
        (newest.soil_temp_5_f.unwrap() - -56.2).abs() < 1e-9,
        "-49 °C kept"
    );
    assert_eq!(newest.ambient_temp_f, Some(50.0));
    assert_eq!(newest.humidity_percent, Some(55.0));
    assert_eq!(newest.soil_moisture_10, Some(0.25));
    assert_eq!(
        newest.precipitation_mm,
        Some(2.5),
        "precipitation stays in mm"
    );

    assert_eq!(oldest.soil_temp_10_f, None, "-50 °C is a sentinel");
    assert_eq!(oldest.soil_temp_5_f, None);
    assert_eq!(oldest.ambient_temp_f, None);
    assert_eq!(oldest.soil_moisture_10, None);
    assert_eq!(oldest.humidity_percent, None);
    assert_eq!(oldest.precipitation_mm, None);
}

// @spec WX-SRC-LAKE-004
#[tokio::test]
async fn a_sensor_silent_for_over_24_hours_is_left_empty() {
    let newest = ts("2026-09-20 18:00:00");
    let file = fixtures::silver(&[
        SilverRow {
            at: newest - Duration::hours(30),
            soil_temp_10: Some(15.0),
            ..Default::default()
        },
        SilverRow {
            at: newest - Duration::hours(23),
            soil_temp_5: Some(18.0),
            ..Default::default()
        },
        SilverRow {
            at: newest,
            air_temp_c: Some(20.0),
            ..Default::default()
        },
    ]);
    let reading = fixtures::client(&file.path, "unused.parquet")
        .fetch_latest()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        reading.timestamp,
        newest.and_utc(),
        "stamped with the newest row"
    );
    assert_eq!(
        reading.soil_temp_10_f, None,
        "30 h old is outside the window"
    );
    assert!((reading.soil_temp_5_f.unwrap() - 64.4).abs() < 1e-9);
    assert_eq!(reading.ambient_temp_f, Some(68.0));
}

// @spec WX-SRC-LAKE-005
#[tokio::test]
async fn the_week_summary_covers_the_168_hours_before_now() {
    let now = Utc::now().naive_utc().with_nanosecond(0).unwrap();
    let mut rows: Vec<SilverRow> = (1..=48)
        .map(|h| SilverRow {
            precip_mm: Some(0.5),
            ..hour(
                now - Duration::hours(h),
                Some(if h <= 24 { 20.0 } else { 18.0 }),
            )
        })
        .collect();
    rows.push(SilverRow {
        precip_mm: Some(100.0),
        ..hour(now - Duration::days(8), Some(100.0))
    });
    let file = fixtures::silver(&rows);

    let s = fixtures::client(&file.path, "unused.parquet")
        .fetch_summary(chrono::Utc::now())
        .await
        .unwrap();
    assert_eq!(s.current.unwrap().soil_temp_10_f, Some(68.0));
    assert!((s.soil_temp_7day_avg_f.unwrap() - 66.2).abs() < 1e-9);
    assert_eq!(s.ambient_temp_7day_avg_f, Some(50.0));
    assert_eq!(s.humidity_7day_avg, Some(60.0));
    assert_eq!(
        s.precipitation_7day_total_mm,
        Some(24.0),
        "the 8-day-old hour is out"
    );
    assert_eq!(s.soil_temp_trend, Trend::Rising);
}

// @spec WX-SRC-MODEL-003
#[tokio::test]
async fn precipitation_stays_in_millimetres_through_the_backend() {
    let now = Utc::now().naive_utc().with_nanosecond(0).unwrap();
    let file = fixtures::silver(&[SilverRow {
        precip_mm: Some(25.4),
        ..hour(now - Duration::hours(1), Some(20.0))
    }]);
    let s = fixtures::client(&file.path, "unused.parquet")
        .fetch_summary(chrono::Utc::now())
        .await
        .unwrap();
    assert_eq!(s.current.as_ref().unwrap().precipitation_mm, Some(25.4));
    assert_eq!(s.precipitation_7day_total_mm, Some(25.4));
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["precipitation_7day_total_mm"], 25.4);
    assert_eq!(json["current"]["precipitation_mm"], 25.4);
    assert!(json.get("precipitation_7day_total_in").is_none());
}

// @spec WX-SRC-LAKE-009
#[tokio::test]
async fn dawn_and_afternoon_flags_count_only_usable_hours() {
    // Local = UTC − 4 h. 05:00 local has RH 0 (unusable); 14:00 local is usable.
    let file = fixtures::silver(&[
        SilverRow {
            at: ts("2026-09-20 09:00:00"),
            air_temp_c: Some(15.0),
            rh_pct: Some(0.0),
            ..Default::default()
        },
        SilverRow {
            at: ts("2026-09-20 18:00:00"),
            air_temp_c: Some(25.0),
            rh_pct: Some(50.0),
            ..Default::default()
        },
    ]);
    let lake = fixtures::client(&file.path, "unused.parquet")
        .fetch_daily_disease_inputs(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap())
        .await
        .unwrap();
    let day = &lake.days[0];
    assert_eq!(day.hours_covered, 1.0);
    assert!(!day.covers_dawn, "the only dawn hour has no RH");
    assert!(day.covers_afternoon);
    assert_eq!(day.temp_mean_c, 25.0);
}

// @spec WX-SRC-LAKE-011
#[tokio::test]
async fn daily_reads_use_the_lakes_local_date_as_given() {
    // Twelve hours spanning UTC midnight, all labelled Sep 20 by the lake.
    let label = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
    let rows: Vec<SilverRow> = (0..12)
        .map(|h| SilverRow {
            local_date: Some(label),
            soil_temp_5: Some(10.0),
            ..hour(ts("2026-09-20 18:00:00") + Duration::hours(h), None)
        })
        .collect();
    let silver = fixtures::silver(&rows);
    let gold = fixtures::gold(&[fixtures::GoldRow::day(label)]);
    let client = fixtures::client(&silver.path, &gold.path);
    let start = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();

    let disease = client.fetch_daily_disease_inputs(start).await.unwrap();
    assert_eq!(disease.days.len(), 1);
    assert_eq!(
        (disease.days[0].date, disease.days[0].hours_covered),
        (label, 12.0)
    );

    let climate = client.fetch_climate_record(start).await.unwrap();
    assert_eq!(climate.days.len(), 1);
    assert_eq!(climate.days[0].date, label);
    assert_eq!(climate.days[0].soil_temp_5_f, Some(50.0));
}
