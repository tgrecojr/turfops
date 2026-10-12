//! Fixtures for the MCP tests that need PostgreSQL (`--features db-tests`).

use super::test_support::{config, state, Lake};
use crate::clock::Clock;
use crate::db::{plant_queries, queries, soil_test_queries};
use crate::logic::data_sync::DataSyncService;
use crate::models::plant::Plant;
use crate::models::{Application, ApplicationType, GrassType, LawnProfile, SoilTest};
use crate::state::AppState;
use chrono::{NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;

/// The lawn profile every handler reads; returns its id.
pub async fn lawn(pool: &PgPool) -> i64 {
    let mut profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    profile.lawn_size_sqft = Some(5000.0);
    queries::create_lawn_profile(pool, &profile).await.unwrap()
}

/// A lawn, a lake of `days` days ending yesterday and the state over both.
pub async fn lawn_with_lake(pool: PgPool, name: &str, days: i64) -> (AppState, Lake, i64) {
    let id = lawn(&pool).await;
    let lake = Lake::with_days(name, days, &[]);
    let state = state(pool, &lake).await;
    (state, lake, id)
}

pub fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

/// Log an application; returns its id.
pub async fn log(
    pool: &PgPool,
    profile: i64,
    kind: ApplicationType,
    on: NaiveDate,
    product: &str,
) -> i64 {
    let fertilizer = kind == ApplicationType::Fertilizer;
    let app = Application {
        id: None,
        lawn_profile_id: profile,
        application_type: kind,
        product_name: Some(product.into()),
        application_date: on,
        rate_per_1000sqft: fertilizer.then_some(3.0),
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: fertilizer.then_some(24.0),
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    };
    queries::create_application(pool, &app).await.unwrap()
}

/// Record a soil test; returns its id.
pub async fn soil_test(pool: &PgPool, profile: i64, on: NaiveDate, ph: f64) -> i64 {
    let test = SoilTest {
        id: None,
        lawn_profile_id: profile,
        test_date: on,
        lab_name: Some("Penn State".into()),
        ph,
        buffer_ph: None,
        phosphorus_ppm: Some(20.0),
        potassium_ppm: Some(90.0),
        calcium_ppm: None,
        magnesium_ppm: None,
        sulfur_ppm: None,
        iron_ppm: None,
        manganese_ppm: None,
        zinc_ppm: None,
        boron_ppm: None,
        copper_ppm: None,
        organic_matter_pct: Some(4.0),
        cec: None,
        notes: None,
        created_at: Utc::now(),
    };
    soil_test_queries::create_soil_test(pool, &test)
        .await
        .unwrap()
}

/// Put a product on the shelf; returns its id.
pub async fn product(pool: &PgPool, profile: i64, name: &str, category: &str, archived: bool) {
    let form = if category == "Seed" {
        "Seed"
    } else {
        "Granular"
    };
    sqlx::query(
        "INSERT INTO products (lawn_profile_id, name, category, form, archived) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(profile)
    .bind(name)
    .bind(category)
    .bind(form)
    .bind(archived)
    .execute(pool)
    .await
    .unwrap();
}

/// Add a landscape plant with a one-task care plan.
pub async fn plant(pool: &PgPool, profile: i64, name: &str) {
    let plant: Plant = serde_json::from_value(json!({
        "id": null, "lawn_profile_id": profile, "common_name": name,
        "scientific_name": "Hydrangea macrophylla", "plant_type": "Shrub",
        "location": "Front bed", "planting_date": "2024-05-01", "notes": null,
        "maintenance_plan": {
            "identified_name": name, "scientific_name": "Hydrangea macrophylla",
            "identification_confidence": "High", "summary": "Blooms on old wood.",
            "tasks": [{"task_type": "Pruning", "window_start_month_day": "07-15",
                       "window_end_month_day": "08-15", "frequency": "Once",
                       "description": "Prune after flowering.", "severity": "Info",
                       "zone_note": null}],
            "warnings": ["Do not prune in spring."]
        },
        "plan_generated_at": Utc::now(), "plan_model": "test-model",
        "created_at": Utc::now(), "updated_at": Utc::now()
    }))
    .unwrap();
    plant_queries::create_plant(pool, &plant).await.unwrap();
}

/// A handler body as JSON, read back from its text as a tool's reply is.
pub fn body<T: serde::Serialize>(result: crate::error::Result<axum::Json<T>>) -> Value {
    match result {
        Ok(axum::Json(body)) => {
            serde_json::from_str(&serde_json::to_string(&body).unwrap()).unwrap()
        }
        Err(err) => panic!("handler failed: {err}"),
    }
}

/// `value` with every key that records when it was computed removed, at any depth, so
/// two computations of the same answer compare equal.
pub fn timeless(mut value: Value) -> Value {
    fn walk(v: &mut Value) {
        match v {
            Value::Object(map) => {
                for key in ["generated_at", "fetched_at", "last_updated", "created_at"] {
                    map.remove(key);
                }
                map.values_mut().for_each(walk);
            }
            Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut value);
    value
}

/// The names of `items` (each an object with `key`), in order.
pub fn names<'a>(items: &'a Value, key: &str) -> Vec<&'a str> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i[key].as_str().unwrap())
        .collect()
}

/// App state whose forecast comes from a local stand-in for OpenWeatherMap that answers
/// every request with six days of 3-hourly points from today's UTC midnight.
pub async fn state_with_forecast(pool: PgPool, lake: &Lake) -> AppState {
    let config = config(&lake.silver, &lake.gold);
    let owm = crate::config::OpenWeatherMapConfig {
        api_key: "test-key".into(),
        latitude: 39.8,
        longitude: -75.8,
        enabled: true,
    };
    let client = crate::datasources::OpenWeatherMapClient::new(owm, Clock::System)
        .with_base_url(&forecast_stub().await);
    let sync = DataSyncService::initialize(&config, pool.clone())
        .await
        .with_forecast_client(client);
    AppState::new(pool, sync, None, true, Clock::System)
}

/// A local HTTP server answering every request with the same OpenWeatherMap forecast.
async fn forecast_stub() -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let midnight = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let list: Vec<Value> = (0..48)
        .map(|i| {
            json!({"dt": midnight + i * 3 * 3600,
                   "main": {"temp": 60.0 + (i % 8) as f64, "feels_like": 60.0, "humidity": 60.0},
                   "weather": [{"id": 800, "main": "Clear", "description": "clear sky"}],
                   "clouds": {"all": 0.0}, "wind": {"speed": 5.0}, "pop": 0.0})
        })
        .collect();
    let body = json!({"list": list, "city": {"name": "Avondale", "country": "US",
                      "coord": {"lat": 39.8, "lon": -75.8}, "timezone": 0}})
    .to_string();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 64 * 1024];
                let _ = socket.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    format!("http://{addr}")
}
