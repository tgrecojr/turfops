//! `GET /disease-risk` and the feed bridge against PostgreSQL and a lake fixture.

use super::*;
use crate::clock::Clock;
use crate::config::{
    Config, DataLakeConfig, DatabaseConfig, HomeAssistantConfig, LawnConfig, McpConfig, NoaaConfig,
    ServerConfig, TemperatureUnit,
};
use crate::logic::data_sync::DataSyncService;
use crate::models::{GrassType, RiskTier};
use axum::response::IntoResponse;
use duckdb::Connection;
use sqlx::PgPool;

fn config(silver: &str, clock: Clock) -> Config {
    Config {
        lawn: LawnConfig {
            name: "Test".into(),
            grass_type: "TallFescue".into(),
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: None,
            irrigation_type: None,
        },
        noaa: NoaaConfig {
            station_wbanno: 3761,
        },
        datalake: DataLakeConfig {
            silver_weather_path: silver.into(),
            gold_weather_path: silver.into(),
        },
        homeassistant: HomeAssistantConfig {
            url: String::new(),
            token: String::new(),
            temperature_entity: String::new(),
            humidity_entity: String::new(),
            temperature_unit: TemperatureUnit::Fahrenheit,
        },
        openweathermap: None,
        openrouter: None,
        server: ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            cors_allowed_origin: None,
        },
        database: DatabaseConfig {
            host: String::new(),
            port: 0,
            name: String::new(),
            user: String::new(),
            password: String::new(),
            max_connections: 1,
        },
        mcp: McpConfig { token: None },
        clock,
    }
}

pub(super) async fn state_with(pool: &PgPool, silver: &str, clock: Clock) -> AppState {
    let sync = DataSyncService::initialize(&config(silver, clock), pool.clone()).await;
    AppState::new(pool.clone(), sync, None, false, clock)
}

pub(super) async fn lawn(pool: &PgPool) -> LawnProfile {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(pool, &profile).await.unwrap();
    queries::get_default_lawn_profile(pool)
        .await
        .unwrap()
        .unwrap()
}

/// A silver parquet with 24 warm, saturated hours a day for `station` from twelve days
/// before `today` through `today`, filed under each local date.
pub(super) fn silver_fixture(name: &str, station: &str, today: NaiveDate) -> String {
    let path =
        std::env::temp_dir().join(format!("turfops_dr_{name}_{}.parquet", std::process::id()));
    let path_s = path.to_string_lossy().to_string();
    let start = today - Duration::days(disease::LOOKBACK_DAYS);
    Connection::open_in_memory()
        .unwrap()
        .execute_batch(&format!(
            "COPY (
                SELECT ts + INTERVAL 4 HOUR AS obs_ts_utc, ts AS obs_ts_local,
                       CAST(ts AS DATE) AS obs_date_local, '{station}' AS wbanno,
                       24.0 AS air_temp_c, 21.0 AS air_temp_min_c, 31.0 AS air_temp_max_c,
                       95.0 AS rh_pct, 0.0 AS precip_mm
                FROM (SELECT TIMESTAMP '{start} 00:00:00' + n * INTERVAL 1 HOUR AS ts
                      FROM range({hours}) t(n))
             ) TO '{path_s}' (FORMAT PARQUET)",
            hours = (disease::LOOKBACK_DAYS + 1) * 24
        ))
        .unwrap();
    path_s
}

pub(super) fn eastern() -> Clock {
    Clock::Zone(chrono_tz::America::New_York)
}

// @spec DIS-MODEL-API-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_page_and_the_feed_read_the_same_assessment(pool: PgPool) {
    let clock = eastern();
    let silver = silver_fixture("same", "03761", clock.today());
    let state = state_with(&pool, &silver, clock).await;
    let profile = lawn(&pool).await;

    let Json(response) = get_disease_risk(State(state.clone())).await.unwrap();
    let recs = recommendations(&state, &profile).await;
    std::fs::remove_file(&silver).ok();

    assert_eq!(response.today, clock.today());
    assert_eq!(response.station, "NOAA USCRN station 3761");
    assert!((Utc::now() - response.generated_at).num_seconds() < 60);
    assert_eq!(response.diseases.len(), 5);
    assert!(response
        .data_notes
        .iter()
        .any(|n| n.contains("Forecast unavailable")));

    let mut from_page: Vec<String> = response
        .diseases
        .iter()
        .filter(|r| r.tier >= RiskTier::High)
        .map(|r| format!("disease_{}", r.slug.replace('-', "_")))
        .collect();
    let mut from_feed: Vec<String> = recs.iter().map(|r| r.id.clone()).collect();
    from_page.sort();
    from_feed.sort();
    assert!(!from_page.is_empty());
    assert_eq!(from_page, from_feed);
    for rec in &recs {
        let risk = response
            .diseases
            .iter()
            .find(|r| rec.id.ends_with(&r.slug.replace('-', "_")))
            .unwrap();
        assert_eq!(rec.description, risk.summary);
    }
}

// @spec DIS-MODEL-DAY-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn today_is_the_lawns_date_even_with_no_station_rows(pool: PgPool) {
    // Kiritimati runs UTC+14: its date is ahead of UTC's for most of every day.
    let clock = Clock::Zone(chrono_tz::Pacific::Kiritimati);
    let silver = silver_fixture("nostation", "99999", clock.today());
    let state = state_with(&pool, &silver, clock).await;
    lawn(&pool).await;

    let Json(response) = get_disease_risk(State(state)).await.unwrap();
    std::fs::remove_file(&silver).ok();

    assert_eq!(response.today, clock.today());
    assert!(response.diseases.is_empty());
    assert!(response
        .data_notes
        .contains(&"No recent station observations are available.".to_string()));
}

// @spec DIS-MODEL-API-002
#[sqlx::test(migrations = "src/db/migrations")]
async fn no_profile_is_404_no_lake_is_503_and_a_failed_read_is_an_error(pool: PgPool) {
    let clock = eastern();
    let silver = silver_fixture("errors", "03761", clock.today());
    let state = state_with(&pool, &silver, clock).await;
    let status = |r: Result<Json<DiseaseRiskResponse>, TurfOpsError>| {
        r.unwrap_err().into_response().status().as_u16()
    };
    assert_eq!(status(get_disease_risk(State(state.clone())).await), 404);

    lawn(&pool).await;
    let sync = DataSyncService::initialize(&config(&silver, clock), pool.clone())
        .await
        .without_weather_lake();
    let no_lake = AppState::new(pool.clone(), sync, None, false, clock);
    assert_eq!(status(get_disease_risk(State(no_lake)).await), 503);
    std::fs::remove_file(&silver).ok();

    let broken = state_with(&pool, "/nonexistent/silver.parquet", clock).await;
    assert_eq!(status(get_disease_risk(State(broken)).await), 503);
}

// @spec DIS-MGMT-REC-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failed_assessment_contributes_no_recommendations(pool: PgPool) {
    let state = state_with(&pool, "/nonexistent/silver.parquet", eastern()).await;
    let profile = lawn(&pool).await;
    assert!(recommendations(&state, &profile).await.is_empty());
}
