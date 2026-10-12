//! A profile refresh the assistant can't answer (`--features db-tests`).

use super::crud_db_tests::add;
use super::test_state::{lawn, state, state_with_assistant_off};
use super::*;
use axum::response::IntoResponse;
use sqlx::PgPool;

async fn refused(state: AppState, name: &str) -> String {
    let product = add(&state, name, ProductCategory::Fungicide).await;
    let err = refresh_product_profile(State(state), Path(product.id.unwrap()))
        .await
        .expect_err("the refresh cannot be answered");
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let message = body["error"].as_str().unwrap().to_string();
    assert!(
        !message.to_lowercase().contains("category"),
        "a refresh never says to pick a category: {message}"
    );
    message
}

// @spec INV-LLM-012
#[sqlx::test(migrations = "src/db/migrations")]
async fn each_refresh_outage_says_the_profile_cant_be_refreshed_and_what_to_do(pool: PgPool) {
    lawn(&pool).await;
    let absent = refused(state(pool.clone(), None).await, "Heritage G").await;
    assert!(
        absent.contains("profile can't be refreshed") && absent.contains("OPENROUTER_API_KEY"),
        "{absent}"
    );
    let off = refused(state_with_assistant_off(pool.clone()).await, "Banner Maxx").await;
    assert!(
        off.contains("profile can't be refreshed") && off.contains("OPENROUTER_ENABLED=true"),
        "{off}"
    );
    // Nothing listens on port 1, so the assistant does not answer.
    let down = refused(
        state(pool, Some("http://127.0.0.1:1".into())).await,
        "Daconil",
    )
    .await;
    assert!(
        down.contains("wasn't refreshed") && down.contains("Try again in a few minutes"),
        "{down}"
    );
}
