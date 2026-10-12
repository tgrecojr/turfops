//! The climate-record slot in `AppState`: one per process, empty at startup, shared.

use super::test_env::*;
use crate::api::timing::climate_read;
use crate::datasources::weather::ClimateRecord;
use chrono::{Duration, Utc};
use std::sync::Arc;

// @spec SYNC-MEMO-001
#[tokio::test]
async fn one_climate_slot_starts_empty_and_is_shared_by_every_caller() {
    let state = offline_state(unreachable_pool()).await;
    assert!(
        state.climate_cache.read().await.is_none(),
        "empty at startup"
    );

    let elsewhere = state.clone();
    assert!(Arc::ptr_eq(&state.climate_cache, &elsewhere.climate_cache));
    let lake = state
        .sync_service
        .read()
        .await
        .weather_client()
        .cloned()
        .unwrap();

    // A record stored through one handle is what another handle's caller is served,
    // aged from the time stored with it: 30 minutes old is fresh, so no re-read.
    let record = Arc::new(ClimateRecord { days: vec![] });
    let read_at = Utc::now() - Duration::minutes(30);
    *state.climate_cache.write().await = Some((read_at, record.clone()));
    let served = climate_read(&elsewhere, &lake).await.unwrap();
    assert!(Arc::ptr_eq(&served.record, &record));
    assert_eq!(served.last_good_at, None);

    // Two hours old: re-read (the lake is down), then served as the last good read of
    // the time stored with it.
    let read_at = Utc::now() - Duration::hours(2);
    *elsewhere.climate_cache.write().await = Some((read_at, record.clone()));
    let served = climate_read(&state, &lake).await.unwrap();
    assert!(Arc::ptr_eq(&served.record, &record));
    assert_eq!(served.last_good_at, Some(read_at));
}
