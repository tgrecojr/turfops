//! Adding a product with no category and no assistant to fill one in (`--features db-tests`).

use super::crud_db_tests::request;
use super::test_state::{lawn, state};
use super::*;
use axum::response::IntoResponse;
use sqlx::PgPool;

async fn refusal(state: &AppState, assist: bool) -> (StatusCode, String) {
    let mut req = request("Mystery Granules", None);
    req.assist = Some(assist);
    let err = create_product(State(state.clone()), Json(req))
        .await
        .expect_err("a product without a category is refused");
    let response = err.into_response();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (status, body["error"].as_str().unwrap().to_string())
}

// @spec INV-API-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn no_category_without_the_assistant_is_a_400_and_saves_nothing(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let (status, message) = refusal(&state, false).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        message,
        "Pick a category to save the product without the assistant."
    );
    let Json(shelf) = list_products(
        State(state),
        Query(ListQuery {
            category: None,
            include_archived: true,
        }),
    )
    .await
    .unwrap();
    assert!(shelf.is_empty());
}

// @spec INV-API-005
#[sqlx::test(migrations = "src/db/migrations")]
async fn no_category_with_the_assistant_unconfigured_is_the_assistant_503(pool: PgPool) {
    lawn(&pool).await;
    let state = state(pool, None).await;
    let (status, message) = refusal(&state, true).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        message.starts_with("The product assistant isn't configured.")
            && message.contains("OPENROUTER_API_KEY"),
        "{message}"
    );
    let Json(shelf) = list_products(
        State(state),
        Query(ListQuery {
            category: None,
            include_archived: true,
        }),
    )
    .await
    .unwrap();
    assert!(shelf.is_empty());
}
