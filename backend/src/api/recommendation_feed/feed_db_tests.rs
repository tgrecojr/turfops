//! The feed end to end over PostgreSQL, with the weather lake missing (an outage): the
//! rules, landscape plants and follow-ups still contribute; disease and timing do not.

use super::fixtures::{application, plant_fertilizing_on, state_without_lake};
use crate::api::recommendations::{list_recommendations, ListRecommendationsQuery};
use crate::clock::Clock;
use crate::db::{plant_queries, product_queries, queries, soil_test_queries};
use crate::error::TurfOpsError;
use crate::models::product::{ProductCategory, ProductFacts, ProductForm, StockStatus};
use crate::models::{
    ApplicationType, GrassType, InventoryState, LawnProfile, Product, Recommendation,
    RecommendationCategory, Severity, SoilTest,
};
use crate::state::AppState;
use axum::extract::{Query, State};
use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;

async fn lawn(pool: &PgPool) -> LawnProfile {
    let seed = LawnProfile::new("Front".into(), GrassType::TallFescue, "7a".into());
    queries::create_lawn_profile(pool, &seed).await.unwrap();
    queries::get_default_lawn_profile(pool)
        .await
        .unwrap()
        .unwrap()
}

/// Log an application whose follow-up falls on `follow_up`; returns its follow-up rec id.
async fn follow_up(pool: &PgPool, profile_id: i64, follow_up: NaiveDate, n: i64) -> String {
    let mut app = application(
        profile_id,
        ApplicationType::Fungicide,
        follow_up - Duration::days(30 + n),
        Some(follow_up),
    );
    app.product_name = Some(format!("Product {n}"));
    let id = queries::create_application(pool, &app).await.unwrap();
    format!("application_followup_{id}")
}

async fn feed(state: &AppState, profile: &LawnProfile) -> Vec<Recommendation> {
    let summary = state
        .sync_service
        .write()
        .await
        .get_or_refresh()
        .await
        .unwrap();
    super::all(state, profile, &summary).await.unwrap()
}

fn position(feed: &[Recommendation], id: &str) -> usize {
    feed.iter().position(|r| r.id == id).expect(id)
}

// @spec FEED-PIPE-002, FEED-PIPE-015, FEED-PIPE-016
#[sqlx::test(migrations = "src/db/migrations")]
async fn sources_contribute_in_order_and_a_lake_outage_costs_only_its_own(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let profile = lawn(&pool).await;
    let pid = profile.id.unwrap();
    let today = state.clock.today();
    let plant_id = plant_queries::create_plant(&pool, &plant_fertilizing_on(pid, today))
        .await
        .unwrap();
    let follow = follow_up(&pool, pid, today + Duration::days(3), 0).await;

    let feed = feed(&state, &profile).await;
    let plant = format!("plant_{plant_id}_");
    let plant_pos = feed.iter().position(|r| r.id.starts_with(&plant)).unwrap();
    assert_eq!(feed[plant_pos].severity, Severity::Advisory);
    assert_eq!(feed[position(&feed, &follow)].severity, Severity::Advisory);
    // Equal severities keep source order: rules, then plants, then follow-ups.
    assert!(plant_pos < position(&feed, &follow));
    let summary = state
        .sync_service
        .write()
        .await
        .get_or_refresh()
        .await
        .unwrap();
    let apps = queries::get_applications_for_profile(&pool, pid, 1000, 0)
        .await
        .unwrap();
    for rule in state.rules_engine.evaluate(&summary, &profile, &apps) {
        if rule.severity == Severity::Advisory {
            assert!(position(&feed, &rule.id) < plant_pos, "{}", rule.id);
        }
    }
    // The lake is down: no disease, timing-window or aeration advice; the rest stands.
    assert!(!feed.iter().any(|r| r.id.starts_with("disease_")
        || r.id.starts_with("timing_")
        || r.id == "aeration"));
}

// @spec FEED-PIPE-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn follow_ups_are_judged_on_the_lawn_clock(pool: PgPool) {
    // Kiritimati (UTC+14) is always at least a day ahead of Pago Pago (UTC-11).
    let ahead = Clock::from_tz(Some("Pacific/Kiritimati")).unwrap();
    let behind = Clock::from_tz(Some("Pacific/Pago_Pago")).unwrap();
    let profile = lawn(&pool).await;
    let due = ahead.today();
    let id = follow_up(&pool, profile.id.unwrap(), due, 0).await;

    let on_the_day = feed(&state_without_lake(pool.clone(), ahead).await, &profile).await;
    assert_eq!(
        on_the_day[position(&on_the_day, &id)].severity,
        Severity::Warning
    );
    let day_before = feed(&state_without_lake(pool.clone(), behind).await, &profile).await;
    assert_eq!(
        day_before[position(&day_before, &id)].severity,
        Severity::Advisory
    );
}

fn product(profile_id: i64, name: &str, category: ProductCategory) -> Product {
    Product {
        id: None,
        lawn_profile_id: profile_id,
        name: name.into(),
        brand: None,
        stock_status: StockStatus::InStock,
        facts: ProductFacts::manual(category, ProductForm::Granular),
        profile: None,
        profile_generated_at: None,
        profile_model: None,
        notes: None,
        archived: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// @spec FEED-PIPE-014
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_shelf_is_every_product_not_archived(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let profile = lawn(&pool).await;
    let pid = profile.id.unwrap();
    let plant = plant_fertilizing_on(pid, state.clock.today());
    let plant_id = plant_queries::create_plant(&pool, &plant).await.unwrap();
    let prefix = format!("plant_{plant_id}_fertilizing_");
    let old = product(pid, "Old Shrub Food", ProductCategory::Fertilizer);
    let old_id = product_queries::create_product(&pool, &old).await.unwrap();
    sqlx::query("UPDATE products SET archived = TRUE WHERE id = $1")
        .bind(old_id)
        .execute(&pool)
        .await
        .unwrap();

    let feed_now = feed(&state, &profile).await;
    let rec = feed_now.iter().find(|r| r.id.starts_with(&prefix)).unwrap();
    let status = rec.inventory.as_ref().expect("the shelf was read");
    assert_eq!(
        status.state,
        InventoryState::NotOnHand,
        "archived is off the shelf"
    );

    let kelp = product(pid, "Kelp Tonic", ProductCategory::Supplement);
    product_queries::create_product(&pool, &kelp).await.unwrap();
    let feed_now = feed(&state, &profile).await;
    let rec = feed_now.iter().find(|r| r.id.starts_with(&prefix)).unwrap();
    let status = rec.inventory.as_ref().unwrap();
    assert_eq!(status.state, InventoryState::OnHand);
    let names: Vec<_> = status
        .matches
        .iter()
        .map(|m| m.product.name.as_str())
        .collect();
    assert_eq!(names, vec!["Kelp Tonic"]);
}

// @spec FEED-PIPE-018
#[sqlx::test(migrations = "src/db/migrations")]
async fn soil_test_advice_never_enters_the_feed(pool: PgPool) {
    let state = state_without_lake(pool.clone(), Clock::System).await;
    let profile = lawn(&pool).await;
    let test = SoilTest {
        id: None,
        lawn_profile_id: profile.id.unwrap(),
        test_date: state.clock.today() - Duration::days(10),
        lab_name: None,
        ph: 5.0,
        buffer_ph: None,
        phosphorus_ppm: Some(4.0),
        potassium_ppm: Some(30.0),
        calcium_ppm: None,
        magnesium_ppm: None,
        sulfur_ppm: None,
        iron_ppm: Some(1.0),
        manganese_ppm: None,
        zinc_ppm: None,
        boron_ppm: None,
        copper_ppm: None,
        organic_matter_pct: None,
        cec: None,
        notes: None,
        created_at: Utc::now(),
    };
    soil_test_queries::create_soil_test(&pool, &test)
        .await
        .unwrap();

    let feed = feed(&state, &profile).await;
    assert!(!feed
        .iter()
        .any(|r| r.category == RecommendationCategory::SoilTest || r.id.starts_with("soil_")));
}

// @spec FEED-PIPE-017
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_a_lawn_profile_the_feed_endpoints_are_not_found(pool: PgPool) {
    let state = state_without_lake(pool, Clock::System).await;
    let query = ListRecommendationsQuery {
        include_inactive: false,
    };
    let list = list_recommendations(State(state.clone()), Query(query)).await;
    assert!(matches!(list, Err(TurfOpsError::NotFound(_))));
    let dashboard = crate::api::dashboard::get_dashboard(State(state)).await;
    assert!(matches!(dashboard, Err(TurfOpsError::NotFound(_))));
}
