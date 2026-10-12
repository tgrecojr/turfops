//! `/api/v1/profile` against PostgreSQL.

use super::{get_profile, update_profile, UpdateProfileRequest};
use crate::api::applications::db_support::{lawn, reply, state};
use crate::models::{GrassType, IrrigationType, LawnProfile, SoilType};
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;
use sqlx::PgPool;

async fn put(
    state: &AppState,
    body: serde_json::Value,
) -> Result<LawnProfile, (StatusCode, String)> {
    let req: UpdateProfileRequest = serde_json::from_value(body).unwrap();
    match update_profile(State(state.clone()), Json(req)).await {
        Ok(Json(profile)) => Ok(profile),
        Err(err) => Err(reply(err).await),
    }
}

async fn profiles(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM lawn_profiles")
        .fetch_one(pool)
        .await
        .unwrap()
}

// @spec APP-PROF-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn the_profile_or_a_404_that_says_none_exists(pool: PgPool) {
    let state = state(&pool).await;
    let err = get_profile(State(state.clone())).await.unwrap_err();
    assert_eq!(
        reply(err).await,
        (StatusCode::NOT_FOUND, "No lawn profile found".to_string())
    );
    let id = lawn(&pool).await;
    let Json(profile) = get_profile(State(state)).await.unwrap();
    assert_eq!(profile.id, Some(id));
}

// @spec APP-PROF-002, APP-PROF-004
#[sqlx::test(migrations = "src/db/migrations")]
async fn every_profile_field_is_stored_by_its_variant_name(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    let saved = put(
        &state,
        json!({
            "name": "Back Forty", "grass_type": "kbg", "usda_zone": "zone six-ish",
            "soil_type": "silt loam", "lawn_size_sqft": 4200.0, "irrigation_type": "in ground"
        }),
    )
    .await
    .unwrap();
    assert_eq!(saved.name, "Back Forty");
    assert_eq!(saved.grass_type, GrassType::KentuckyBluegrass);
    assert_eq!(saved.usda_zone, "zone six-ish", "the zone is free text");
    assert_eq!(saved.soil_type, Some(SoilType::SiltLoam));
    assert_eq!(saved.lawn_size_sqft, Some(4200.0));
    assert_eq!(saved.irrigation_type, Some(IrrigationType::InGround));

    let stored: (String, Option<String>, Option<String>) =
        sqlx::query_as("SELECT grass_type, soil_type, irrigation_type FROM lawn_profiles")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        stored,
        (
            "KentuckyBluegrass".into(),
            Some("SiltLoam".into()),
            Some("InGround".into())
        )
    );
    let body = serde_json::to_value(&saved).unwrap();
    assert_eq!(body["grass_type"], "KentuckyBluegrass");
    assert_eq!(body["soil_type"], "SiltLoam");
    assert_eq!(body["irrigation_type"], "InGround");
}

// @spec APP-PROF-009
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_update_touches_only_the_fields_sent(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    put(
        &state,
        json!({ "soil_type": "Clay", "lawn_size_sqft": 5000.0, "irrigation_type": "Hose" }),
    )
    .await
    .unwrap();

    let renamed = put(&state, json!({ "name": "Side Yard" })).await.unwrap();
    assert_eq!(renamed.name, "Side Yard");
    assert_eq!(renamed.usda_zone, "7a", "absent = left alone");
    assert_eq!(renamed.soil_type, Some(SoilType::Clay));
    assert_eq!(renamed.lawn_size_sqft, Some(5000.0));

    let cleared = put(
        &state,
        json!({ "soil_type": "", "irrigation_type": "", "lawn_size_sqft": 0.0 }),
    )
    .await
    .unwrap();
    assert_eq!(cleared.soil_type, None);
    assert_eq!(cleared.irrigation_type, None);
    assert_eq!(cleared.lawn_size_sqft, None);
    assert_eq!(cleared.name, "Side Yard");
    let negative = put(&state, json!({ "lawn_size_sqft": 4000.0 }))
        .await
        .unwrap();
    assert_eq!(negative.lawn_size_sqft, Some(4000.0));
    assert_eq!(
        put(&state, json!({ "lawn_size_sqft": -1.0 }))
            .await
            .unwrap()
            .lawn_size_sqft,
        None
    );

    let stored_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM lawn_profiles")
            .fetch_one(&pool)
            .await
            .unwrap();
    let last = put(&state, json!({})).await.unwrap();
    let reread: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM lawn_profiles")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(reread >= stored_at);
    assert_eq!(last.updated_at, reread, "the response is the stored row");
}

// @spec APP-PROF-010
#[sqlx::test(migrations = "src/db/migrations")]
async fn an_unknown_type_is_a_400_naming_it_and_changes_nothing(pool: PgPool) {
    lawn(&pool).await;
    let state = state(&pool).await;
    for (field, value) in [
        ("grass_type", "Bluegras"),
        ("soil_type", "Mud"),
        ("irrigation_type", "drip"),
    ] {
        let (status, msg) = put(&state, json!({ "name": "Renamed", field: value }))
            .await
            .unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert!(msg.contains(value), "{msg}");
    }
    let Json(profile) = get_profile(State(state)).await.unwrap();
    assert_eq!(profile.name, "Front", "nothing changed");
}

// @spec APP-PROF-011, APP-PROF-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn saving_with_no_profile_creates_it_with_the_seeding_defaults(pool: PgPool) {
    let state = state(&pool).await;
    let (status, _) = put(&state, json!({ "grass_type": "Bluegras" }))
        .await
        .unwrap_err();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(profiles(&pool).await, 0, "a 400 creates nothing");

    let created = put(&state, json!({ "usda_zone": "6b", "soil_type": "" }))
        .await
        .unwrap();
    assert!(created.id.is_some(), "the stored row");
    assert_eq!(created.name, "Main Lawn");
    assert_eq!(created.grass_type, GrassType::TallFescue);
    assert_eq!(created.usda_zone, "6b");
    assert_eq!(
        (
            created.soil_type,
            created.lawn_size_sqft,
            created.irrigation_type
        ),
        (None, None, None)
    );

    put(&state, json!({ "name": "Front" })).await.unwrap();
    assert_eq!(profiles(&pool).await, 1, "the next save updates it");
}
