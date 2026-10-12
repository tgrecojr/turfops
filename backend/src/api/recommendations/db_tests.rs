//! The recommendation endpoints over PostgreSQL: listing the feed with and without the
//! answered ones, the dashboard's top three, and what PATCH stores.

use super::{
    list_recommendations, patch_recommendation, ListRecommendationsQuery,
    PatchRecommendationRequest,
};
use crate::api::recommendation_feed::fixtures::{application, state_without_lake};
use crate::clock::Clock;
use crate::db::queries;
use crate::models::{
    ApplicationType, GrassType, LawnProfile, Recommendation, RecommendationState, Severity,
};
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;

async fn lawn(pool: &PgPool) -> i64 {
    let seed = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(pool, &seed).await.unwrap()
}

/// A logged application with a follow-up on `due`; returns the follow-up's rec id.
async fn follow_up(pool: &PgPool, profile_id: i64, due: NaiveDate, n: i64) -> String {
    let mut app = application(
        profile_id,
        ApplicationType::Fungicide,
        due - Duration::days(30 + n),
        Some(due),
    );
    app.product_name = Some(format!("Product {n}"));
    let id = queries::create_application(pool, &app).await.unwrap();
    format!("application_followup_{id}")
}

async fn list(state: &AppState, include_inactive: bool) -> Vec<Recommendation> {
    let query = ListRecommendationsQuery { include_inactive };
    let Json(recs) = list_recommendations(State(state.clone()), Query(query))
        .await
        .unwrap();
    recs
}

fn ids(recs: &[Recommendation]) -> Vec<&str> {
    recs.iter().map(|r| r.id.as_str()).collect()
}

// @spec FEED-API-001, FEED-STATE-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_list_hides_answered_advice_unless_asked_for_it(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let pid = lawn(&pool).await;
    let today = state.clock.today();
    let overdue = follow_up(&pool, pid, today - Duration::days(1), 0).await; // Warning
    let upcoming = follow_up(&pool, pid, today + Duration::days(3), 1).await; // Advisory
    queries::upsert_recommendation_state(&pool, &overdue, true, false, Some(Severity::Warning))
        .await
        .unwrap();
    let stored = queries::get_recommendation_states(&pool).await.unwrap();

    let active = list(&state, false).await;
    assert!(!ids(&active).contains(&overdue.as_str()));
    assert!(ids(&active).contains(&upcoming.as_str()));

    let all = list(&state, true).await;
    let flagged = all.iter().find(|r| r.id == overdue).unwrap();
    assert!(flagged.dismissed && !flagged.is_active());
    let order = |id: &str| all.iter().position(|r| r.id == id).unwrap();
    assert!(
        order(&overdue) < order(&upcoming),
        "feed order: Warning first"
    );
    let without_answered: Vec<_> = all.iter().filter(|r| r.is_active()).collect();
    assert_eq!(
        ids(&active),
        without_answered
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        queries::get_recommendation_states(&pool).await.unwrap(),
        stored
    );
}

// @spec FEED-API-006, FEED-PIPE-001, FEED-UI-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_dashboard_shows_the_first_three_of_the_page(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let pid = lawn(&pool).await;
    let today = state.clock.today();
    for n in 0..6 {
        let offset = if n % 2 == 0 { -n - 1 } else { n + 1 }; // overdue / upcoming
        follow_up(&pool, pid, today + Duration::days(offset), n).await;
    }

    let page = list(&state, false).await;
    assert!(page.len() > 3);
    let Json(dashboard) = crate::api::dashboard::get_dashboard(State(state.clone()))
        .await
        .unwrap();
    assert_eq!(ids(&dashboard.recommendations), ids(&page[..3]));
    // Recent Applications: the five newest entries.
    let log = queries::get_applications_for_profile(&pool, pid, 5, 0)
        .await
        .unwrap();
    assert_eq!(dashboard.recent_applications.len(), 5);
    let recent: Vec<_> = dashboard.recent_applications.iter().map(|a| a.id).collect();
    assert_eq!(recent, log.iter().map(|a| a.id).collect::<Vec<_>>());
}

async fn patch(
    state: &AppState,
    id: &str,
    dismissed: Option<bool>,
    addressed: Option<bool>,
    severity: Option<Severity>,
) -> serde_json::Value {
    let body = PatchRecommendationRequest {
        dismissed,
        addressed,
        severity,
    };
    let Json(out) = patch_recommendation(State(state.clone()), Path(id.into()), Json(body))
        .await
        .unwrap();
    out
}

async fn row(pool: &PgPool, id: &str) -> Option<RecommendationState> {
    queries::get_recommendation_states(pool)
        .await
        .unwrap()
        .remove(id)
}

async fn age_answer(pool: &PgPool, id: &str, days: i64) {
    sqlx::query("UPDATE recommendation_states SET updated_at = NOW() - make_interval(days => $2) WHERE id = $1")
        .bind(id)
        .bind(days as i32)
        .execute(pool)
        .await
        .unwrap();
}

// @spec FEED-API-002, FEED-API-005, FEED-STATE-001, FEED-STATE-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_answer_is_one_row_per_id_and_each_click_restarts_its_episode(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let out = patch(&state, "aeration", Some(true), None, Some(Severity::Info)).await;
    assert_eq!(
        out,
        serde_json::json!({"id": "aeration", "dismissed": true, "addressed": false})
    );
    age_answer(&pool, "aeration", 130).await;
    let seasonal = Recommendation::new(
        "aeration",
        crate::models::RecommendationCategory::Aeration,
        Severity::Info,
        "Aerate",
        "Core aerate now.",
    );
    let lapsed = row(&pool, "aeration").await.unwrap();
    assert!(
        !lapsed.suppresses(&seasonal, Utc::now()),
        "120-day episode is over"
    );

    let out = patch(&state, "aeration", None, Some(true), None).await;
    assert_eq!(
        out,
        serde_json::json!({"id": "aeration", "dismissed": true, "addressed": true})
    );
    let again = row(&pool, "aeration").await.unwrap();
    assert!(Utc::now() - again.updated_at < chrono::Duration::minutes(1));
    assert!(again.suppresses(&seasonal, Utc::now()), "a fresh episode");
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM recommendation_states")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // A first answer without a severity stores NULL.
    patch(&state, "rain_delay", Some(true), None, None).await;
    assert_eq!(row(&pool, "rain_delay").await.unwrap().severity, None);
}

// @spec FEED-API-003, FEED-API-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn omitted_fields_keep_their_stored_values_and_two_falses_restore(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    patch(
        &state,
        "grub_control_2026",
        Some(true),
        None,
        Some(Severity::Warning),
    )
    .await;

    patch(&state, "grub_control_2026", None, Some(true), None).await;
    let kept = row(&pool, "grub_control_2026").await.unwrap();
    assert!(kept.dismissed && kept.addressed);
    assert_eq!(kept.severity, Some(Severity::Warning));

    // Clearing one flag keeps the other, so the answer still stands.
    let out = patch(&state, "grub_control_2026", Some(false), None, None).await;
    assert_eq!(out["addressed"], true);
    let kept = row(&pool, "grub_control_2026").await.unwrap();
    assert!(!kept.dismissed && kept.addressed);
    assert_eq!(kept.severity, Some(Severity::Warning));

    // Both false once the stored flag is filled in: Restore deletes the row.
    let out = patch(&state, "grub_control_2026", None, Some(false), None).await;
    assert_eq!(
        out,
        serde_json::json!({"id": "grub_control_2026", "dismissed": false, "addressed": false})
    );
    assert!(row(&pool, "grub_control_2026").await.is_none());
}
