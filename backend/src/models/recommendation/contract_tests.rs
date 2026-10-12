//! The shape every surface receives and the severity order every comparison uses.

use super::*;
use crate::models::{InventoryState, InventoryStatus, ProductCategory, ProductNeed};

fn sample() -> Recommendation {
    Recommendation::new(
        "grub_control_2026",
        RecommendationCategory::GrubControl,
        Severity::Advisory,
        "Grub preventative window",
        "Apply a grub preventative.",
    )
    .with_explanation("Egg-laying is under way.")
    .with_data_point("GDD", 600, "Calculated")
    .with_action("Apply before peak hatch.")
    .with_need(ProductNeed::new(
        "grub preventative",
        ProductCategory::InsectControl,
    ))
}

// @spec FEED-REC-001
#[test]
fn a_recommendation_carries_every_field_a_surface_shows() {
    let mut rec = sample().with_feeding(0.5);
    rec.inventory = Some(InventoryStatus {
        state: InventoryState::NotOnHand,
        matches: vec![],
        missing: vec!["grub preventative".into()],
    });
    let json = serde_json::to_value(&rec).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = vec![
        "id",
        "category",
        "severity",
        "title",
        "description",
        "explanation",
        "data_points",
        "suggested_action",
        "needs",
        "inventory",
        "created_at",
        "dismissed",
        "addressed",
    ];
    expected.sort_unstable();
    assert_eq!(keys, expected, "and nothing internal (the feeding mark)");
    assert_eq!(json["id"], "grub_control_2026");
    assert_eq!(json["category"], "GrubControl");
    assert_eq!(json["severity"], "Advisory");
    assert_eq!(json["explanation"], "Egg-laying is under way.");
    assert_eq!(json["data_points"][0]["label"], "GDD");
    assert_eq!(json["data_points"][0]["value"], "600");
    assert_eq!(json["suggested_action"], "Apply before peak hatch.");
    assert_eq!(json["inventory"]["state"], "NotOnHand");
    assert_eq!(json["dismissed"], false);
    assert_eq!(json["addressed"], false);
}

// @spec FEED-REC-002
#[test]
fn severities_order_info_advisory_warning_critical() {
    let mut shuffled = vec![
        Severity::Warning,
        Severity::Info,
        Severity::Critical,
        Severity::Advisory,
    ];
    shuffled.sort();
    assert_eq!(
        shuffled,
        vec![
            Severity::Info,
            Severity::Advisory,
            Severity::Warning,
            Severity::Critical
        ]
    );
    assert!(Severity::Info < Severity::Advisory);
    assert!(Severity::Advisory < Severity::Warning);
    assert!(Severity::Warning < Severity::Critical);
}

// @spec FEED-REC-003
#[test]
fn needs_are_always_sent_and_inventory_only_when_set() {
    let plain = Recommendation::new(
        "rain_delay",
        RecommendationCategory::ApplicationTiming,
        Severity::Info,
        "t",
        "d",
    );
    let json = serde_json::to_value(&plain).unwrap();
    assert_eq!(json["needs"], serde_json::json!([]));
    assert!(json.get("inventory").is_none());

    let json = serde_json::to_value(sample()).unwrap();
    assert_eq!(json["needs"].as_array().unwrap().len(), 1);
    assert!(json.get("inventory").is_none(), "no verdict yet");
}
