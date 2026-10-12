//! `GET /api/v1/applications/calendar` against PostgreSQL.

use super::{get_calendar, CalendarQuery, CalendarResponse};
use crate::api::seasonal_plan::db_support::*;
use crate::models::ApplicationType;
use axum::extract::{Query, State};
use axum::http::StatusCode;
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
