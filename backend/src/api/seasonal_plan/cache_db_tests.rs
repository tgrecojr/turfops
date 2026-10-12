//! The 10 cm crossing cache: what a row holds, how a year is replaced, and how
//! `GET /api/v1/seasonal-plan` fills it from the weather lake.

use super::db_support::*;
use crate::db::queries;
use crate::models::seasonal_plan::ThresholdCrossing;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

fn crossing(year: i32, name: &str, month: u32, day: u32) -> ThresholdCrossing {
    ThresholdCrossing {
        year,
        threshold_name: name.into(),
        crossing_date: d(year, month, day),
        avg_soil_temp_f: 51.5,
    }
}

/// A cache row as stored: year, threshold, date, mean, computed at, station.
type StoredRow = (
    i32,
    String,
    chrono::NaiveDate,
    f64,
    DateTime<Utc>,
    Option<i32>,
);

// @spec PLAN-CACHE-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_cached_crossing_records_its_date_mean_time_and_station(pool: PgPool) {
    let before = Utc::now() - chrono::Duration::seconds(5);
    let rows = [crossing(2024, "soil_50f_rising", 4, 6)];
    queries::replace_threshold_crossings(&pool, 2024, STATION, &rows, true)
        .await
        .unwrap();

    let stored: Vec<StoredRow> = sqlx::query_as(
        "SELECT year, threshold_name, crossing_date, avg_soil_temp_f, computed_at, \
                    station_wbanno FROM seasonal_threshold_crossings",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(stored.len(), 1, "one row per year and threshold");
    let (year, name, date, mean, computed_at, station) = &stored[0];
    assert_eq!((*year, name.as_str()), (2024, "soil_50f_rising"));
    assert_eq!(*date, d(2024, 4, 6));
    assert_eq!(*mean, 51.5);
    assert!(*computed_at >= before, "computed_at is when it was written");
    assert_eq!(*station, Some(STATION));
}

// @spec PLAN-CACHE-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn replacing_a_year_drops_a_threshold_that_no_longer_crosses(pool: PgPool) {
    let first = [
        crossing(2024, "soil_50f_rising", 4, 6),
        crossing(2024, "soil_55f_rising", 4, 23),
    ];
    queries::replace_threshold_crossings(&pool, 2024, STATION, &first, false)
        .await
        .unwrap();
    let again = [crossing(2024, "soil_50f_rising", 4, 8)];
    queries::replace_threshold_crossings(&pool, 2024, STATION, &again, false)
        .await
        .unwrap();

    let rows = cached(&pool).await;
    assert_eq!(
        rows,
        vec![(2024, "soil_50f_rising".into(), d(2024, 4, 8), Some(STATION))]
    );
}

// @spec PLAN-CACHE-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failed_replacement_leaves_the_year_as_it_was(pool: PgPool) {
    let first = [crossing(2024, "soil_50f_rising", 4, 6)];
    queries::replace_threshold_crossings(&pool, 2024, STATION, &first, false)
        .await
        .unwrap();
    // The second row repeats the key, so the insert fails after the delete ran.
    let clash = [
        crossing(2024, "soil_55f_rising", 4, 23),
        crossing(2024, "soil_55f_rising", 4, 24),
    ];
    assert!(
        queries::replace_threshold_crossings(&pool, 2024, STATION, &clash, false)
            .await
            .is_err()
    );

    let rows = cached(&pool).await;
    assert_eq!(
        rows,
        vec![(2024, "soil_50f_rising".into(), d(2024, 4, 6), Some(STATION))],
        "delete and inserts are one transaction"
    );
}

// @spec PLAN-CACHE-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_plan_discards_crossings_cached_for_another_station(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let year = this_year() - 5;
    insert_crossing(&pool, year, "soil_50f_rising", d(year, 4, 1), Some(9999)).await;
    insert_crossing(&pool, year, "soil_55f_rising", d(year, 4, 20), None).await;
    let kept = this_year() - 4;
    insert_crossing(&pool, kept, "soil_50f_rising", d(kept, 4, 2), Some(STATION)).await;
    mark_year(&pool, STATION, kept, true).await;
    let dir = scratch("other-station");
    let state = state(&pool, Some(lake(&dir.join("gold.parquet"))));

    get_plan(&state, Some(this_year() - 1)).await.unwrap();

    let rows = cached(&pool).await;
    assert_eq!(
        rows,
        vec![(kept, "soil_50f_rising".into(), d(kept, 4, 2), Some(STATION))],
        "another station's row and the row with no station are gone"
    );
}

// @spec PLAN-CACHE-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_plan_recomputes_every_unsettled_year_of_the_last_ten(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let now = this_year();
    let (recomputed, settled, too_old) = (now - 2, now - 3, now - 11);
    // Stale rows: an unsettled year in range, a settled one, and one before the range.
    insert_crossing(
        &pool,
        recomputed,
        "soil_99f_rising",
        d(recomputed, 3, 1),
        Some(STATION),
    )
    .await;
    insert_crossing(
        &pool,
        settled,
        "soil_99f_rising",
        d(settled, 3, 1),
        Some(STATION),
    )
    .await;
    mark_year(&pool, STATION, settled, true).await;
    insert_crossing(
        &pool,
        too_old,
        "soil_99f_rising",
        d(too_old, 3, 1),
        Some(STATION),
    )
    .await;
    let dir = scratch("recompute");
    let gold = dir.join("gold.parquet");
    write_gold_year(&gold, recomputed);
    let state = state(&pool, Some(lake(&gold)));

    get_plan(&state, Some(now - 1)).await.unwrap();

    let names = |year: i32, rows: &[(i32, String, chrono::NaiveDate, Option<i32>)]| {
        rows.iter()
            .filter(|r| r.0 == year)
            .map(|r| (r.1.clone(), r.2))
            .collect::<Vec<_>>()
    };
    let rows = cached(&pool).await;
    assert_eq!(
        names(recomputed, &rows),
        vec![
            ("soil_50f_rising".to_string(), d(recomputed, 4, 4)),
            ("soil_55f_rising".to_string(), d(recomputed, 4, 6)),
            ("soil_60f_rising".to_string(), d(recomputed, 4, 7)),
        ],
        "the unsettled year is replaced with the lake's crossings"
    );
    assert_eq!(
        names(settled, &rows).len(),
        1,
        "a settled year is not re-read"
    );
    assert_eq!(names(too_old, &rows).len(), 1, "outside the ten-year range");
}
