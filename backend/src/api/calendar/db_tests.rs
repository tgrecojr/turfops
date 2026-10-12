//! `GET /api/v1/applications/calendar` against PostgreSQL.

use super::{get_calendar, CalendarQuery, CalendarResponse};
use crate::api::seasonal_plan::db_support::*;
use crate::models::ApplicationType;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{Datelike, Local};
use sqlx::PgPool;

async fn calendar(
    pool: &PgPool,
    year: Option<i32>,
    month: Option<u32>,
) -> Result<CalendarResponse, StatusCode> {
    get_calendar(
        State(state(pool, None)),
        Query(CalendarQuery { year, month }),
    )
    .await
    .map(|json| json.0)
    .map_err(status_of)
}

// @spec PLAN-CAL-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_calendar_defaults_to_this_month_and_keys_days_by_date(pool: PgPool) {
    let lawn = create_profile(&pool, "Front").await;
    log(
        &pool,
        application(lawn, ApplicationType::Mowing, d(2025, 6, 7)),
    )
    .await;

    let today = Local::now().date_naive();
    let current = calendar(&pool, None, None).await.unwrap();
    assert_eq!((current.year, current.month), (today.year(), today.month()));

    let june = calendar(&pool, Some(2025), Some(6)).await.unwrap();
    assert_eq!((june.year, june.month), (2025, 6));
    let json = serde_json::to_value(&june).unwrap();
    assert_eq!(json["days"]["2025-06-07"][0]["application_type"], "Mowing");
}

// @spec PLAN-CAL-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_month_holds_applications_and_follow_ups_under_their_logged_date(pool: PgPool) {
    let lawn = create_profile(&pool, "Front").await;
    let other = create_profile(&pool, "Back").await;
    log(
        &pool,
        application(lawn, ApplicationType::Mowing, d(2025, 6, 7)),
    )
    .await;
    let mut seeded = application(lawn, ApplicationType::Overseed, d(2025, 5, 20));
    seeded.follow_up_date = Some(d(2025, 6, 10));
    log(&pool, seeded).await;
    log(
        &pool,
        application(lawn, ApplicationType::Mowing, d(2025, 7, 1)),
    )
    .await;
    log(
        &pool,
        application(other, ApplicationType::Mowing, d(2025, 6, 8)),
    )
    .await;

    let june = calendar(&pool, Some(2025), Some(6)).await.unwrap();
    let keys: Vec<&str> = june.days.keys().map(String::as_str).collect();
    assert_eq!(keys, ["2025-05-20", "2025-06-07"]);
    assert_eq!(
        june.days["2025-05-20"][0].follow_up_date,
        Some(d(2025, 6, 10)),
        "a follow-up due in June sits under its May application date"
    );
    assert!(
        june.days
            .values()
            .flatten()
            .all(|a| a.lawn_profile_id == lawn),
        "only the default profile"
    );
}

// @spec PLAN-CAL-003
#[sqlx::test(migrations = "src/db/migrations")]
async fn no_profile_is_404_and_an_impossible_month_is_400(pool: PgPool) {
    assert_eq!(
        calendar(&pool, Some(2025), Some(6)).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
    create_profile(&pool, "Front").await;
    assert_eq!(
        calendar(&pool, Some(2025), Some(13)).await.unwrap_err(),
        StatusCode::BAD_REQUEST
    );
}

// @spec PLAN-CAL-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_default_month_comes_from_the_app_clock(pool: PgPool) {
    use crate::clock::Clock;
    use crate::logic::data_sync::DataSyncService;
    use crate::state::AppState;
    use chrono::{TimeZone, Utc};
    create_profile(&pool, "Front").await;
    let clock = Clock::Frozen {
        // 02:00 UTC on Mar 1 is still Feb 29 in New York.
        at: Utc.with_ymd_and_hms(2024, 3, 1, 2, 0, 0).unwrap(),
        zone: chrono_tz::America::New_York,
    };
    let sync = DataSyncService::with_lake_only(pool.clone(), None, clock);
    let state = AppState::new(pool, sync, None, false, clock);
    let Json(feb) = get_calendar(
        State(state),
        Query(CalendarQuery {
            year: None,
            month: None,
        }),
    )
    .await
    .unwrap();
    assert_eq!((feb.year, feb.month), (2024, 2));
}
