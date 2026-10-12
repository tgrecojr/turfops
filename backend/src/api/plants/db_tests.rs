//! Adding a plant and regenerating its plan, against PostgreSQL and a stand-in assistant.

use super::db_support::{assistant, profile, saved_plant, state, status, MODEL};
use super::*;
use crate::datasources::openrouter::stub::{completion, sample_plan};
use crate::models::plant::IdentificationConfidence;
use serde_json::json;
use sqlx::PgPool;

fn request(body: serde_json::Value) -> Json<CreatePlantRequest> {
    Json(serde_json::from_value(body).unwrap())
}

// @spec PLANT-API-002, PLANT-API-003, PLANT-PROMPT-001, PLANT-DATA-001
#[sqlx::test(migrations = "src/db/migrations")]
async fn adding_a_plant_saves_the_generated_plan_with_its_model_and_time(pool: PgPool) {
    let profile_id = profile(&pool, "6b").await;
    let (client, received) = assistant(200, completion(&sample_plan())).await;
    let state = state(pool.clone(), Some(client)).await;
    let before = Utc::now();
    let (code, Json(created)) = create_plant(
        State(state),
        request(json!({
            "common_name": "Panicle hydrangea",
            "scientific_name": "Hydrangea paniculata",
            "plant_type": "shrub",
            "location": "front bed",
            "planting_date": "2024-04-01",
            "notes": "gift from Mom"
        })),
    )
    .await
    .unwrap();
    assert_eq!(code, StatusCode::CREATED);

    let prompt = received.await.unwrap().user_prompt().to_string();
    assert!(prompt.contains("Input: Panicle hydrangea (Hydrangea paniculata)"));
    assert!(prompt.contains("USDA hardiness zone: 6b"), "{prompt}");
    assert!(prompt.contains("Plant type (user-reported): Shrub"));
    assert!(prompt.contains("Location on property: front bed"));
    assert!(!prompt.contains("2024-04-01") && !prompt.contains("gift from Mom"));

    let stored = plant_queries::get_plant(&pool, created.id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.lawn_profile_id, profile_id);
    assert_eq!(stored.common_name, "Panicle hydrangea");
    assert_eq!(
        stored.scientific_name.as_deref(),
        Some("Hydrangea paniculata")
    );
    assert_eq!(stored.plant_type, PlantType::Shrub);
    assert_eq!(stored.location.as_deref(), Some("front bed"));
    assert_eq!(stored.planting_date, NaiveDate::from_ymd_opt(2024, 4, 1));
    assert_eq!(stored.notes.as_deref(), Some("gift from Mom"));
    assert_eq!(stored.maintenance_plan.tasks.len(), 1);
    assert_eq!(stored.plan_model, MODEL);
    assert!(stored.plan_generated_at >= before - chrono::Duration::seconds(1));
}

// @spec PLANT-API-003, PLANT-API-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_name_left_empty_is_filled_from_the_plan(pool: PgPool) {
    profile(&pool, "7a").await;
    let (client, received) = assistant(200, completion(&sample_plan())).await;
    let (_, Json(by_science)) = create_plant(
        State(state(pool.clone(), Some(client)).await),
        request(json!({ "scientific_name": "Hydrangea paniculata", "plant_type": "Shrub" })),
    )
    .await
    .unwrap();
    assert!(received
        .await
        .unwrap()
        .user_prompt()
        .contains("Input: Hydrangea paniculata\n"));
    assert_eq!(by_science.common_name, "Panicle hydrangea");

    let (client, received) = assistant(200, completion(&sample_plan())).await;
    let (_, Json(by_name)) = create_plant(
        State(state(pool.clone(), Some(client)).await),
        request(
            json!({ "common_name": "Hydrangea", "scientific_name": "", "plant_type": "Shrub" }),
        ),
    )
    .await
    .unwrap();
    assert!(received
        .await
        .unwrap()
        .user_prompt()
        .contains("Input: Hydrangea\n"));
    assert_eq!(by_name.common_name, "Hydrangea");
    assert_eq!(
        by_name.scientific_name.as_deref(),
        Some("Hydrangea paniculata")
    );
}

// @spec PLANT-API-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_low_confidence_plan_with_no_tasks_is_saved(pool: PgPool) {
    profile(&pool, "7a").await;
    let unknown = json!({
        "identified_name": "Unknown plant", "scientific_name": null,
        "identification_confidence": "Low",
        "summary": "\"Blue thing\" doesn't name a plant I can identify.",
        "tasks": [], "warnings": []
    });
    let (client, _received) = assistant(200, completion(&unknown)).await;
    let (code, Json(saved)) = create_plant(
        State(state(pool.clone(), Some(client)).await),
        request(json!({ "common_name": "Blue thing", "plant_type": "Other" })),
    )
    .await
    .unwrap();
    assert_eq!(code, StatusCode::CREATED);
    let plan = &saved.maintenance_plan;
    assert_eq!(
        plan.identification_confidence,
        IdentificationConfidence::Low
    );
    assert!(plan.tasks.is_empty());
}

// @spec PLANT-API-007
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_failed_generation_is_a_503_that_writes_nothing(pool: PgPool) {
    let profile_id = profile(&pool, "7a").await;
    let (client, _) = assistant(500, r#"{"error":"overloaded"}"#.into()).await;
    let err = create_plant(
        State(state(pool.clone(), Some(client)).await),
        request(json!({ "common_name": "Holly", "plant_type": "Shrub" })),
    )
    .await
    .unwrap_err();
    assert_eq!(status(err), StatusCode::SERVICE_UNAVAILABLE);
    let none = plant_queries::list_plants_for_profile(&pool, profile_id)
        .await
        .unwrap();
    assert!(none.is_empty());

    let id = saved_plant(&pool, profile_id, "American holly").await;
    let (client, _) = assistant(200, r#"{"choices":[]}"#.into()).await;
    let err = refresh_plant_plan(State(state(pool.clone(), Some(client)).await), Path(id))
        .await
        .unwrap_err();
    assert_eq!(status(err), StatusCode::SERVICE_UNAVAILABLE);
    let kept = plant_queries::get_plant(&pool, id).await.unwrap().unwrap();
    assert_eq!(kept.maintenance_plan.summary, "The old plan.");
    assert_eq!(kept.plan_model, "old/model");
}

// @spec PLANT-AI-008
#[sqlx::test(migrations = "src/db/migrations")]
async fn a_turned_off_assistant_says_so_and_writes_nothing(pool: PgPool) {
    let profile_id = profile(&pool, "7a").await;
    let off = crate::datasources::OpenRouterClient::new(crate::config::OpenRouterConfig {
        api_key: "test".into(),
        model: MODEL.into(),
        enabled: false,
        base_url: "http://127.0.0.1:1".into(),
    });
    let state = state(pool.clone(), Some(off)).await;
    let err = create_plant(
        State(state.clone()),
        request(json!({ "common_name": "Holly", "plant_type": "Shrub" })),
    )
    .await
    .unwrap_err();
    let response = axum::response::IntoResponse::into_response(err);
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let message = body["error"].as_str().unwrap();
    assert!(
        message.starts_with("The plan assistant is turned off.")
            && message.contains("OPENROUTER_ENABLED=true"),
        "{message}"
    );
    assert!(plant_queries::list_plants_for_profile(&pool, profile_id)
        .await
        .unwrap()
        .is_empty());
}

// @spec PLANT-API-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn regenerating_replaces_the_plan_from_the_stored_plant(pool: PgPool) {
    let profile_id = profile(&pool, "6a").await;
    let id = saved_plant(&pool, profile_id, "American holly").await;
    let old = plant_queries::get_plant(&pool, id).await.unwrap().unwrap();
    let (client, received) = assistant(200, completion(&sample_plan())).await;
    let state = state(pool.clone(), Some(client)).await;
    let Json(updated) = refresh_plant_plan(State(state.clone()), Path(id))
        .await
        .unwrap();

    let prompt = received.await.unwrap().user_prompt().to_string();
    assert!(
        prompt.contains("Input: American holly (Ilex opaca)"),
        "{prompt}"
    );
    assert!(prompt.contains("USDA hardiness zone: 6a"));
    assert!(prompt.contains("Plant type (user-reported): Shrub"));
    assert!(prompt.contains("Location on property: back fence"));
    assert_eq!(
        updated.maintenance_plan.identified_name,
        "Panicle hydrangea"
    );
    assert_eq!(updated.plan_model, MODEL);
    assert!(updated.plan_generated_at > old.plan_generated_at);
    assert!(updated.updated_at > old.updated_at);
    assert_eq!(updated.common_name, "American holly", "names are kept");

    let missing = refresh_plant_plan(State(state), Path(id + 100))
        .await
        .unwrap_err();
    assert_eq!(status(missing), StatusCode::NOT_FOUND);
}

// @spec PLANT-AI-006
#[sqlx::test(migrations = "src/db/migrations")]
async fn without_an_assistant_adding_and_regenerating_are_503_and_the_rest_works(pool: PgPool) {
    let profile_id = profile(&pool, "7a").await;
    let id = saved_plant(&pool, profile_id, "American holly").await;
    let state = state(pool.clone(), None).await;
    let add = create_plant(
        State(state.clone()),
        request(json!({ "common_name": "Holly", "plant_type": "Shrub" })),
    )
    .await
    .unwrap_err();
    let refresh = refresh_plant_plan(State(state.clone()), Path(id))
        .await
        .unwrap_err();
    for err in [add, refresh] {
        let msg = err.owner_message();
        assert_eq!(status(err), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            msg.contains("isn't configured") && msg.contains("OPENROUTER_API_KEY"),
            "{msg}"
        );
    }

    assert_eq!(list_plants(State(state.clone())).await.unwrap().0.len(), 1);
    assert!(get_plant(State(state.clone()), Path(id)).await.is_ok());
    let edit: UpdatePlantRequest = serde_json::from_value(json!({ "location": "porch" })).unwrap();
    let Json(edited) = update_plant(State(state.clone()), Path(id), Json(edit))
        .await
        .unwrap();
    assert_eq!(edited.location.as_deref(), Some("porch"));
    assert_eq!(
        delete_plant(State(state), Path(id)).await.unwrap(),
        StatusCode::NO_CONTENT
    );
}
