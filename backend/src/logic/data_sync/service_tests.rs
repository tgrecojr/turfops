//! The I/O half: when the service contacts its sources, and what it reports.

use super::test_env::*;
use super::DataSyncService;
use chrono::{Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Home Assistant and OpenWeatherMap stubs answering 200, and the service using them.
async fn stubbed() -> (Stub, Stub, DataSyncService) {
    let ha = stub(200, HA_STATE).await;
    let owm = stub(200, OWM_FORECAST).await;
    let mut vars = with_home_assistant(&ha.url);
    vars.extend(with_forecast());
    let service = service(&config_of(&vars), Some(&owm.url), unreachable_pool()).await;
    (ha, owm, service)
}

// @spec SYNC-REFRESH-007
#[tokio::test]
async fn startup_sets_up_clients_without_contacting_a_source() {
    let (ha, owm, _service) = stubbed().await;
    assert_eq!((ha.count(), owm.count()), (0, 0));

    // Sources that are down do not delay startup.
    let mut down = with_home_assistant(UNREACHABLE);
    down.extend(with_forecast());
    let started = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        DataSyncService::initialize(&config_of(&down), unreachable_pool()),
    )
    .await;
    assert!(started.is_ok(), "startup waited on a source");
}

// @spec SYNC-REFRESH-001
#[tokio::test]
async fn an_idle_server_contacts_no_source_until_a_request_needs_conditions() {
    let (ha, owm, service) = stubbed().await;
    let state = app_state(unreachable_pool(), service, false);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!((ha.count(), owm.count()), (0, 0), "no poller, no scheduler");

    state
        .sync_service
        .write()
        .await
        .get_or_refresh()
        .await
        .unwrap();
    assert!(ha.count() > 0 && owm.count() > 0, "read while serving");
}

// @spec SYNC-REFRESH-003
#[tokio::test]
async fn a_fresh_summary_is_returned_without_contacting_any_source() {
    let (ha, owm, mut service) = stubbed().await;
    let first = service.get_or_refresh().await.unwrap();
    let calls = (ha.count(), owm.count());
    assert!(calls.0 > 0 && calls.1 > 0);

    let second = service.get_or_refresh().await.unwrap();
    assert_eq!((ha.count(), owm.count()), calls);
    assert_eq!(second.last_updated, first.last_updated);
}

// @spec SYNC-REFRESH-004
#[tokio::test]
async fn only_the_stale_part_is_refreshed_and_the_other_is_kept() {
    let (ha, owm, mut service) = stubbed().await;
    service.get_or_refresh().await.unwrap();
    let (ha_calls, owm_calls) = (ha.count(), owm.count());

    service.state.age_sensors(Duration::minutes(6));
    let summary = service.get_or_refresh().await.unwrap();
    assert!(ha.count() > ha_calls, "sensors re-read");
    assert_eq!(owm.count(), owm_calls, "forecast not fetched");
    assert!(summary.forecast.is_some(), "cached forecast kept");

    let ha_calls = ha.count();
    let sensors_at = summary.last_updated;
    service.state.age_forecast(Duration::minutes(31));
    let summary = service.get_or_refresh().await.unwrap();
    assert_eq!(owm.count(), owm_calls + 1, "forecast fetched");
    assert_eq!(ha.count(), ha_calls, "sensors not re-read");
    assert_eq!(summary.last_updated, sensors_at, "cached sensors kept");
}

// @spec SYNC-REFRESH-006
#[tokio::test]
async fn simultaneous_requests_share_one_read_of_each_source() {
    let (ha, owm, service) = stubbed().await;
    let shared = Arc::new(RwLock::new(service));
    let request = || {
        let shared = shared.clone();
        async move { shared.write().await.get_or_refresh().await.unwrap() }
    };
    let (a, b, c) = tokio::join!(request(), request(), request());
    // One refresh: Home Assistant's two entities and one forecast fetch.
    assert_eq!(ha.count(), 2, "{:?}", ha.requests());
    assert_eq!(owm.count(), 1);
    assert_eq!(a.last_updated, b.last_updated);
    assert_eq!(b.last_updated, c.last_updated);
}

// @spec SYNC-CONN-001
#[tokio::test]
async fn connection_status_reports_each_source_by_its_probe() {
    let (ha, owm, service) = stubbed().await;
    let status = service.check_connections().await;
    assert!(!status.soildata, "the lake is not mounted");
    assert!(status.homeassistant);
    assert!(status.openweathermap);
    assert!(ha
        .requests()
        .iter()
        .any(|r| r.starts_with("GET /api/ ") && r.ends_with(&format!("Bearer {HA_TOKEN}"))));
    assert!(owm
        .requests()
        .iter()
        .any(|r| r.starts_with("GET /weather?")));
    let json = serde_json::to_value(&status).unwrap();
    for key in ["soildata", "homeassistant", "openweathermap"] {
        assert!(json.get(key).is_some(), "{key}");
    }
}

// @spec SYNC-CONN-001
#[tokio::test]
async fn an_unconfigured_or_failing_source_reports_false() {
    let unconfigured = service(&config(&[]), None, unreachable_pool()).await;
    let status = unconfigured.check_connections().await;
    assert!(!status.homeassistant && !status.openweathermap);

    let ha = stub(401, "{}").await;
    let owm = stub(500, "{}").await;
    let mut vars = with_home_assistant(&ha.url);
    vars.extend(with_forecast());
    let failing = service(&config_of(&vars), Some(&owm.url), unreachable_pool()).await;
    let status = failing.check_connections().await;
    assert!(!status.homeassistant && !status.openweathermap);

    let mut vars = with_home_assistant(UNREACHABLE);
    vars.extend(with_forecast());
    let down = service(&config_of(&vars), Some(UNREACHABLE), unreachable_pool()).await;
    let status = down.check_connections().await;
    assert!(!status.soildata && !status.homeassistant && !status.openweathermap);
}

// @spec SYNC-CONN-002
#[tokio::test]
async fn a_connection_check_leaves_the_summary_and_its_timers_alone() {
    let (_ha, _owm, mut service) = stubbed().await;
    service.check_connections().await;
    let now = Utc::now();
    let due = service.state.due(now);
    assert!(due.sensors && due.forecast, "a probe is not a refresh");
    assert_eq!(service.state.summary(now).last_updated, None);

    service.get_or_refresh().await.unwrap();
    service.state.age_sensors(Duration::minutes(2));
    let before = service.state.summary(Utc::now());
    let due = service.state.due(Utc::now());
    *service.probes.lock().unwrap() = Default::default();
    service.check_connections().await;
    assert_eq!(service.state.due(Utc::now()), due);
    let after = service.state.summary(Utc::now());
    assert_eq!(after.last_updated, before.last_updated);
    assert_eq!(after.forecast.is_some(), before.forecast.is_some());
}

// @spec SYNC-FAIL-001
#[tokio::test]
async fn a_refresh_with_every_source_failing_still_returns_a_summary() {
    let ha = stub(500, "{}").await;
    let owm = stub(500, "{}").await;
    let mut vars = with_home_assistant(&ha.url);
    vars.extend(with_forecast());
    let service = service(&config_of(&vars), Some(&owm.url), unreachable_pool()).await;
    let state = app_state(unreachable_pool(), service, false);

    let axum::Json(summary) =
        crate::api::environmental::get_environmental(axum::extract::State(state.clone()))
            .await
            .expect("GET /environmental absorbs source errors");
    assert!(summary.current.is_some() && summary.last_updated.is_some());
    assert!(ha.count() > 0 && owm.count() > 0, "every source was tried");

    let axum::Json(forced) =
        crate::api::environmental::refresh_environmental(axum::extract::State(state))
            .await
            .expect("POST /environmental/refresh absorbs source errors");
    assert!(forced.current.is_some());
    assert!(forced.forecast.is_none());
}
