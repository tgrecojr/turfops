//! What the shared OpenRouter client sends, and what it does with the answer.

use super::stub::{capture, completion, sample_plan, Captured};
use super::tests::sample_config;
use super::*;
use crate::error::Outage;

fn hydrangea(location: Option<&str>) -> PlantPlanRequest<'_> {
    PlantPlanRequest {
        input: "Panicle hydrangea (Hydrangea paniculata)",
        usda_zone: "7a",
        plant_type: PlantType::Shrub,
        location,
    }
}

/// One plan request against a stub that answers with the sample plan.
async fn send(location: Option<&str>) -> (PlantMaintenancePlan, Captured) {
    let (url, received) = capture(200, completion(&sample_plan())).await;
    let client = OpenRouterClient::new(OpenRouterConfig {
        base_url: url,
        ..sample_config()
    });
    let plan = client
        .generate_plant_plan(hydrangea(location))
        .await
        .unwrap();
    (plan, received.await.unwrap())
}

// @spec PLANT-AI-002
#[tokio::test]
async fn every_completion_carries_the_key_and_the_apps_attribution() {
    let (_, sent) = send(None).await;
    assert_eq!(sent.header("authorization"), Some("Bearer test_key"));
    assert_eq!(
        sent.header("http-referer"),
        Some("https://github.com/tgrecojr/turfops")
    );
    assert_eq!(sent.header("x-title"), Some("TurfOps"));
}

// @spec PLANT-AI-003
#[tokio::test]
async fn the_system_prompt_is_cached_and_the_reply_is_strict_json_read_into_the_type() {
    let (plan, sent) = send(None).await;
    let system = &sent.body["messages"][0]["content"][0];
    assert_eq!(sent.body["messages"][0]["role"], "system");
    assert_eq!(system["cache_control"]["type"], "ephemeral");
    assert_eq!(system["text"], SYSTEM_PROMPT);
    let format = &sent.body["response_format"];
    assert_eq!(format["type"], "json_schema");
    assert_eq!(format["json_schema"]["strict"], true);
    assert_eq!(format["json_schema"]["schema"], plan_json_schema());
    assert_eq!(plan.identified_name, "Panicle hydrangea");
    assert_eq!(plan.tasks.len(), 1);
}

// @spec PLANT-PROMPT-001
#[tokio::test]
async fn the_user_prompt_names_the_plant_zone_type_and_location() {
    let (_, sent) = send(Some("front bed")).await;
    let prompt = sent.user_prompt();
    assert!(
        prompt.contains("Input: Panicle hydrangea (Hydrangea paniculata)"),
        "{prompt}"
    );
    assert!(prompt.contains("USDA hardiness zone: 7a"), "{prompt}");
    assert!(
        prompt.contains("Plant type (user-reported): Shrub"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Location on property: front bed"),
        "{prompt}"
    );

    let (_, sent) = send(None).await;
    assert!(
        sent.user_prompt()
            .contains("Location on property: unspecified"),
        "{}",
        sent.user_prompt()
    );
    let lower = sent.user_prompt().to_lowercase();
    assert!(!lower.contains("planting") && !lower.contains("notes"));
}

// @spec PLANT-PROMPT-009
#[tokio::test]
async fn care_plans_are_requested_at_temperature_0_2() {
    let (_, sent) = send(None).await;
    assert_eq!(sent.body["temperature"], 0.2);
}

// @spec PLANT-AI-004
#[test]
fn the_client_waits_5_s_to_connect_and_60_s_for_an_answer() {
    assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(5));
    assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(60));
}

// @spec PLANT-AI-005
#[tokio::test]
async fn a_disabled_client_refuses_without_calling_openrouter() {
    let (url, received) = capture(200, completion(&sample_plan())).await;
    let client = OpenRouterClient::new(OpenRouterConfig {
        base_url: url,
        enabled: false,
        ..sample_config()
    });
    let err = client
        .generate_plant_plan(hydrangea(None))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        TurfOpsError::Unavailable {
            origin: Source::PlanAssistant,
            outage: Outage::Disabled
        }
    ));
    let waited = tokio::time::timeout(Duration::from_millis(300), received).await;
    assert!(waited.is_err(), "OpenRouter was never called");
}

// @spec PLANT-AI-009
#[tokio::test]
async fn the_api_key_never_reaches_debug_output_or_a_logged_error() {
    let config = OpenRouterConfig {
        api_key: "sk-or-v1-very-secret".into(),
        ..sample_config()
    };
    assert!(!format!("{config:?}").contains("very-secret"));

    // A refused connection and an upstream error: neither the owner's message nor the
    // logged detail carries the key.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}/v1", closed.local_addr().unwrap());
    drop(closed);
    let (failing, _received) = capture(401, r#"{"error":"bad key"}"#.into()).await;
    for base_url in [dead, failing] {
        let client = OpenRouterClient::new(OpenRouterConfig {
            base_url,
            ..config.clone()
        });
        let err = client
            .generate_plant_plan(hydrangea(None))
            .await
            .unwrap_err();
        let logged = format!("{err:?} {err} {}", err.owner_message());
        assert!(!logged.contains("very-secret"), "{logged}");
    }
}
