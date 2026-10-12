//! What the care-plan system prompt asks for, and the schema the reply must match.

use super::plant::{plan_json_schema, SYSTEM_PROMPT};
use crate::models::plant::{IdentificationConfidence, TaskFrequency, TaskType};
use crate::models::Severity;
use serde::Serialize;

fn prompt() -> String {
    SYSTEM_PROMPT.to_lowercase()
}

// @spec PLANT-PROMPT-002
#[test]
fn the_prompt_asks_for_a_year_round_homeowner_plan_with_hand_tools_only() {
    let p = prompt();
    assert!(p.contains("year-round"), "{p}");
    assert!(p.contains("homeowner"));
    assert!(p.contains("certified applicator license"));
    for tool in ["hand pruners", "loppers", "bow rake"] {
        assert!(p.contains(tool), "{tool}");
    }
}

// @spec PLANT-PROMPT-003
#[test]
fn the_prompt_says_how_to_handle_an_ambiguous_or_unknown_plant() {
    let p = prompt();
    assert!(p.contains("ambiguous") && p.contains("most common residential variety"));
    assert!(p.contains("\"medium\" or \"low\""), "lower the confidence");
    assert!(p.contains("cannot identify the plant at all"));
    assert!(p.contains("a summary explaining why") && p.contains("an empty tasks array"));
}

// @spec PLANT-PROMPT-004
#[test]
fn the_prompt_asks_for_zone_tuned_mm_dd_windows_one_task_per_window() {
    let p = prompt();
    assert!(p.contains("every task must include a window as mm-dd"));
    assert!(p.contains("tune windows to the usda zone"));
    assert!(p.contains("one maintenancetask per window"));
}

// @spec PLANT-PROMPT-005
#[test]
fn the_prompt_asks_for_short_reasoned_tasks_and_only_relevant_warnings() {
    let p = prompt();
    assert!(p.contains("1-3 sentences") && p.contains("include why"));
    assert!(p.contains("warnings only if they are homeowner-relevant"));
    for hazard in ["pet toxicity", "invasive", "thorns", "sap"] {
        assert!(p.contains(hazard), "{hazard}");
    }
}

fn names<T: Serialize>(values: &[T]) -> Vec<String> {
    values
        .iter()
        .map(|v| {
            serde_json::to_value(v)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

fn schema_enum(node: &serde_json::Value) -> Vec<String> {
    node["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn all_required(node: &serde_json::Value) {
    let mut required: Vec<&str> = node["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let mut properties: Vec<&str> = node["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    required.sort();
    properties.sort();
    assert_eq!(required, properties, "every field is required");
    assert_eq!(node["additionalProperties"], false);
}

// @spec PLANT-PROMPT-008
#[test]
fn the_schema_is_strict_and_its_enums_are_the_backends_types() {
    let schema = plan_json_schema();
    all_required(&schema);
    let task = &schema["properties"]["tasks"]["items"];
    all_required(task);
    let props = &task["properties"];
    assert_eq!(
        schema["properties"]["scientific_name"]["type"],
        serde_json::json!(["string", "null"])
    );
    assert_eq!(
        props["zone_note"]["type"],
        serde_json::json!(["string", "null"])
    );

    use IdentificationConfidence as C;
    use TaskType as T;
    let tasks = [
        T::Pruning,
        T::Fertilizing,
        T::Mulching,
        T::Watering,
        T::PestInspection,
        T::Deadheading,
        T::WinterProtection,
        T::Other,
    ];
    assert_eq!(schema_enum(&props["task_type"]), names(&tasks));
    let freq = [
        TaskFrequency::Once,
        TaskFrequency::Twice,
        TaskFrequency::Monthly,
        TaskFrequency::AsNeeded,
    ];
    assert_eq!(schema_enum(&props["frequency"]), names(&freq));
    assert_eq!(
        schema_enum(&schema["properties"]["identification_confidence"]),
        names(&[C::High, C::Medium, C::Low])
    );
    let sev = [
        Severity::Info,
        Severity::Advisory,
        Severity::Warning,
        Severity::Critical,
    ];
    assert_eq!(schema_enum(&props["severity"]), names(&sev));

    let pattern = "^(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$";
    assert_eq!(props["window_start_month_day"]["pattern"], pattern);
    assert_eq!(props["window_end_month_day"]["pattern"], pattern);
}
