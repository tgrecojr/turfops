//! The product-profile request contract: the strict schema and the model's instructions.

use super::*;
use crate::models::plant::IdentificationConfidence;

/// Every FRAC class the app knows; the match fails to compile when a class is added.
fn every_frac_class() -> Vec<FracClass> {
    use FracClass as F;
    let all = vec![
        F::Frac1,
        F::Frac3,
        F::Frac4,
        F::Frac7,
        F::Frac11,
        F::Frac12,
        F::Frac14,
        F::Frac21,
        F::Frac28,
        F::FracP07,
        F::FracM3,
        F::FracM5,
    ];
    for f in &all {
        match f {
            F::Frac1
            | F::Frac3
            | F::Frac4
            | F::Frac7
            | F::Frac11
            | F::Frac12
            | F::Frac14
            | F::Frac21
            | F::Frac28
            | F::FracP07
            | F::FracM3
            | F::FracM5 => {}
        }
    }
    all
}

fn values(schema: &Value) -> Vec<String> {
    let schema = schema.get("anyOf").map(|a| &a[0]).unwrap_or(schema);
    schema["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn names<T: std::fmt::Display>(all: &[T]) -> Vec<String> {
    all.iter().map(|v| v.to_string()).collect()
}

// @spec INV-LLM-005
#[test]
fn the_schema_is_strict_and_spells_values_from_the_apps_vocabularies() {
    let schema = product_json_schema();
    let props = schema["properties"].as_object().unwrap();
    let mut required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
    required.sort();
    keys.sort();
    assert_eq!(required, keys, "every field required");
    assert_eq!(schema["additionalProperties"], false);
    for nullable in [
        "npk",
        "herbicide_timing",
        "amendment_kind",
        "suggested_label_rate",
    ] {
        assert!(
            props[nullable]["anyOf"][1]["type"] == "null",
            "{nullable} may be null"
        );
    }

    assert_eq!(values(&props["category"]), names(ProductCategory::ALL));
    assert_eq!(values(&props["form"]), names(ProductForm::ALL));
    assert_eq!(
        values(&props["herbicide_timing"]),
        names(HerbicideTiming::ALL)
    );
    assert_eq!(values(&props["amendment_kind"]), names(AmendmentKind::ALL));
    assert_eq!(values(&props["mobility"]), names(Mobility::ALL));
    assert_eq!(
        values(&props["nitrogen_release"]),
        names(NitrogenRelease::ALL)
    );
    assert_eq!(values(&props["suitable_for"]), names(SuitableFor::ALL));
    assert_eq!(
        values(&props["targets"]["items"]),
        names(ProductTarget::ALL)
    );
    let unit = &props["suggested_label_rate"]["anyOf"][0]["properties"]["unit"];
    assert_eq!(values(unit), names(RateUnit::ALL));
    let frac: Vec<Value> = every_frac_class()
        .iter()
        .map(|f| serde_json::to_value(f).unwrap())
        .collect();
    assert_eq!(
        props["frac_classes"]["items"]["enum"].as_array().unwrap(),
        &frac
    );
    let confidence: Vec<Value> = [
        IdentificationConfidence::High,
        IdentificationConfidence::Medium,
        IdentificationConfidence::Low,
    ]
    .iter()
    .map(|c| serde_json::to_value(c).unwrap())
    .collect();
    assert_eq!(props["confidence"]["enum"].as_array().unwrap(), &confidence);

    for key in keys {
        let allowed = key == "herbicide_timing";
        assert!(
            allowed || !(key.contains("interval") || key.contains("timing")),
            "{key}"
        );
    }
}

// @spec INV-LLM-006
#[test]
fn the_instructions_hold_the_model_to_the_label() {
    let p = PRODUCT_SYSTEM_PROMPT;
    for rule in [
        "Prefer the exact retail product",
        "lower the confidence",
        "confidence \"Low\", category \"Other\" (or the user's category)",
        "never invent a formulation",
        "Report only what the label states",
        "leave the list empty if unsure",
        "suggested_label_rate: fill it ONLY when",
        "you are confident of it; otherwise null",
        "never give application intervals or timing",
        "at most 4",
        "1–2 plain sentences",
        "tuned to the grass type",
        "and zone",
    ] {
        assert!(p.contains(rule), "missing: {rule}");
    }
}
