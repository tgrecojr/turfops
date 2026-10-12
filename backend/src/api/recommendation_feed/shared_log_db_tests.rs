//! The feed's one read of the application log (`--features db-tests`).

use super::fixtures::state_without_lake;
use crate::clock::Clock;
use crate::db::queries;
use crate::models::{ApplicationType, GrassType, LawnProfile};
use chrono::Duration;
use sqlx::PgPool;

// @spec FEED-PIPE-011
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_sources_share_one_read_of_the_newest_thousand_entries(pool: PgPool) {
    let seed = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    let profile_id = queries::create_lawn_profile(&pool, &seed).await.unwrap();
    let profile = queries::get_default_lawn_profile(&pool)
        .await
        .unwrap()
        .unwrap();
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let today = state.clock.today();

    // A fungicide whose follow-up is due today, then a thousand newer mowings.
    let mut fungicide = super::fixtures::application(
        profile_id,
        ApplicationType::Fungicide,
        today - Duration::days(30),
        Some(today),
    );
    fungicide.product_name = Some("Heritage G".into());
    let fungicide_id = queries::create_application(&pool, &fungicide)
        .await
        .unwrap();
    let reminder = format!("application_followup_{fungicide_id}");

    let summary = state
        .sync_service
        .write()
        .await
        .get_or_refresh()
        .await
        .unwrap();
    let (feed, log) = super::all_with_history(&state, &profile, &summary)
        .await
        .unwrap();
    assert_eq!(log.len(), 1);
    assert!(
        feed.iter().any(|r| r.id == reminder),
        "the follow-up reads the log"
    );

    sqlx::query(
        "INSERT INTO applications (lawn_profile_id, application_type, product_name, \
         application_date, created_at) \
         SELECT $1, 'Mowing', 'cut ' || g, $2::date - (g % 29), now() \
         FROM generate_series(1, 1000) AS g",
    )
    .bind(profile_id)
    .bind(today)
    .execute(&pool)
    .await
    .unwrap();

    let (feed, log) = super::all_with_history(&state, &profile, &summary)
        .await
        .unwrap();
    assert_eq!(log.len(), 1000, "at most 1000 rows");
    assert!(
        log.windows(2)
            .all(|w| w[0].application_date >= w[1].application_date),
        "newest first"
    );
    assert!(log.iter().all(|a| a.id != Some(fungicide_id)));
    assert!(
        !feed.iter().any(|r| r.id == reminder),
        "follow-ups see the same thousand entries the rules do"
    );
}
