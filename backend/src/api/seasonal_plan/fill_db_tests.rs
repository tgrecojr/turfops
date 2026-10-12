//! How the cache fill degrades: a failed lake read or cache write, and no lake at all.

use super::db_support::*;
use sqlx::PgPool;

// @spec PLAN-CACHE-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failed_lake_read_keeps_the_cached_rows_and_still_returns_the_plan(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let plan_year = this_year();
    settled_history(&pool, STATION, plan_year).await;
    let unsettled = plan_year - 5;
    insert_crossing(
        &pool,
        unsettled,
        "soil_50f_rising",
        d(unsettled, 4, 9),
        Some(STATION),
    )
    .await;
    // No gold parquet at this path: every year's read fails.
    let dir = scratch("read-fails");
    let state = state(&pool, Some(lake(&dir.join("missing.parquet"))));

    let plan = get_plan(&state, Some(plan_year + 1)).await.unwrap();

    assert!(
        cached(&pool).await.iter().any(|r| r.0 == unsettled),
        "the unread year keeps its rows"
    );
    assert_eq!(plan.data_years_used, 3, "built from what is cached");
    assert!(plan.activities.iter().any(|a| a.id == "spring_nitrogen"));
}

// @spec PLAN-CACHE-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failed_cache_write_skips_that_year_and_fills_the_others(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let now = this_year();
    let (failing, filled) = (now - 4, now - 2);
    insert_crossing(
        &pool,
        failing,
        "soil_99f_rising",
        d(failing, 3, 1),
        Some(STATION),
    )
    .await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE FUNCTION refuse_year() RETURNS trigger AS $$ BEGIN \
           IF NEW.year = {failing} THEN RAISE EXCEPTION 'write refused'; END IF; \
           RETURN NEW; END $$ LANGUAGE plpgsql"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER refuse_year BEFORE INSERT ON seasonal_crossing_years \
         FOR EACH ROW EXECUTE FUNCTION refuse_year()",
    )
    .execute(&pool)
    .await
    .unwrap();
    let dir = scratch("write-fails");
    let gold = dir.join("gold.parquet");
    write_gold_year(&gold, filled);
    let state = state(&pool, Some(lake(&gold)));

    get_plan(&state, Some(now - 1)).await.unwrap();

    let rows = cached(&pool).await;
    let of = |year: i32| rows.iter().filter(|r| r.0 == year).count();
    assert_eq!(of(failing), 1, "the failed year keeps its cached row");
    assert_eq!(of(filled), 3, "a later year is still filled");
}

// @spec PLAN-CACHE-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_a_lake_the_plan_uses_the_cache_as_it_is(pool: PgPool) {
    create_profile(&pool, "Front").await;
    let plan_year = this_year() + 1;
    // Rows for some other station would be discarded by a fill; with no lake they stay
    // and vote.
    settled_history(&pool, 9999, plan_year).await;
    let unsettled = plan_year - 6;
    insert_crossing(
        &pool,
        unsettled,
        "soil_50f_rising",
        d(unsettled, 4, 9),
        None,
    )
    .await;
    let before = cached(&pool).await;
    let state = state(&pool, None);

    let plan = get_plan(&state, Some(plan_year)).await.unwrap();

    assert_eq!(cached(&pool).await, before, "no fill ran");
    assert_eq!(plan.data_years_used, 3);
    let nitrogen = plan
        .activities
        .iter()
        .find(|a| a.id == "spring_nitrogen")
        .expect("built from the cached crossings");
    assert_eq!(nitrogen.date_window.predicted_start, d(plan_year, 4, 23));
}
