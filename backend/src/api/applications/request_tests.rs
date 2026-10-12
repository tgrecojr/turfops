//! The application request's shape: what is required, and what an empty field means.

use super::validate::validate;
use super::{fungicide_classes, CreateApplicationRequest};
use crate::models::{ApplicationType, FracClass};
use serde_json::json;

fn parse(body: serde_json::Value) -> Result<CreateApplicationRequest, serde_json::Error> {
    serde_json::from_value(body)
}

// @spec APP-LOG-006
#[test]
fn only_the_type_and_a_calendar_date_are_required() {
    let bare = parse(json!({ "application_type": "Mowing", "application_date": "2026-05-01" }))
        .expect("type and date are enough");
    assert!(validate(&bare, None).is_ok());

    assert!(
        parse(json!({ "application_type": "Mowing" })).is_err(),
        "no date"
    );
    assert!(
        parse(json!({ "application_date": "2026-05-01" })).is_err(),
        "no type"
    );

    let timed = parse(json!({
        "application_type": "Mowing", "application_date": "2026-05-01T08:00:00"
    }))
    .unwrap();
    assert!(validate(&timed, None).is_err(), "a date has no time");

    let full = parse(json!({
        "application_type": "Fungicide", "application_date": "2026-05-01",
        "product_name": "Heritage G", "product_id": 4, "rate_per_1000sqft": 3.0,
        "coverage_sqft": 5000.0, "nitrogen_pct": 0.0, "phosphorus_pct": 0.0,
        "potassium_pct": 0.0, "frac_classes": ["Frac11"], "plant_id": null,
        "follow_up_date": "2026-05-29", "notes": "front yard"
    }))
    .unwrap();
    assert!(validate(&full, None).is_ok());
}

// @spec APP-FRAC-001
#[test]
fn frac_classes_belong_to_a_fungicide_and_an_empty_pick_is_unrecorded() {
    let premix = Some(vec![FracClass::Frac3, FracClass::Frac11]);
    assert_eq!(
        fungicide_classes(ApplicationType::Fungicide, premix.clone()),
        premix
    );
    assert_eq!(
        fungicide_classes(ApplicationType::Fungicide, Some(vec![])),
        None
    );
    assert_eq!(fungicide_classes(ApplicationType::Fungicide, None), None);
    assert_eq!(fungicide_classes(ApplicationType::Fertilizer, premix), None);
}
