//! Request validation and delete of a missing test.

use super::*;

fn body() -> CreateSoilTestRequest {
    CreateSoilTestRequest {
        test_date: "2026-03-23".into(),
        lab_name: None,
        ph: 6.8,
        buffer_ph: Some(6.9),
        phosphorus_ppm: Some(13.0),
        potassium_ppm: Some(50.0),
        calcium_ppm: Some(196.0),
        magnesium_ppm: Some(44.0),
        sulfur_ppm: Some(5.0),
        iron_ppm: Some(2.4),
        manganese_ppm: Some(10.9),
        zinc_ppm: Some(0.1),
        boron_ppm: Some(0.0),
        copper_ppm: Some(0.08),
        organic_matter_pct: Some(3.0),
        cec: Some(8.0),
        notes: None,
    }
}

// @spec SOIL-API-011
#[test]
fn a_negative_measurement_is_a_400_naming_the_field() {
    assert!(
        validate_request(&body()).is_ok(),
        "zero and positive are fine"
    );
    type Set = fn(&mut CreateSoilTestRequest);
    let cases: [(&str, Set); 14] = [
        ("Buffer pH", |r| r.buffer_ph = Some(-1.0)),
        ("Phosphorus", |r| r.phosphorus_ppm = Some(-1.0)),
        ("Potassium", |r| r.potassium_ppm = Some(-1.0)),
        ("Calcium", |r| r.calcium_ppm = Some(-1.0)),
        ("Magnesium", |r| r.magnesium_ppm = Some(-1.0)),
        ("Sulfur", |r| r.sulfur_ppm = Some(-1.0)),
        ("Iron", |r| r.iron_ppm = Some(-1.0)),
        ("Manganese", |r| r.manganese_ppm = Some(-1.0)),
        ("Zinc", |r| r.zinc_ppm = Some(-1.0)),
        ("Boron", |r| r.boron_ppm = Some(-1.0)),
        ("Copper", |r| r.copper_ppm = Some(-1.0)),
        ("Organic matter", |r| r.organic_matter_pct = Some(-1.0)),
        ("CEC", |r| r.cec = Some(-1.0)),
        ("Phosphorus", |r| r.phosphorus_ppm = Some(-0.01)),
    ];
    for (field, set) in cases {
        let mut req = body();
        set(&mut req);
        match validate_request(&req) {
            Err(TurfOpsError::InvalidData(msg)) => {
                assert!(msg.starts_with(field), "{field}: {msg}");
                assert!(msg.contains("can't be negative"), "{field}: {msg}");
            }
            other => panic!("{field}: expected a 400, got {other:?}"),
        }
    }
}

// @spec SOIL-API-012
#[test]
fn deleting_a_missing_test_is_not_found() {
    assert!(deleted(1, 7).is_ok());
    match deleted(0, 7) {
        Err(TurfOpsError::NotFound(msg)) => assert_eq!(msg, "Soil test 7 not found"),
        other => panic!("expected 404, got {other:?}"),
    }
}

// @spec SOIL-DATA-005
#[test]
fn same_date_tests_put_the_latest_entry_first() {
    assert_eq!(
        soil_test_queries::newest_first!(),
        "test_date DESC, created_at DESC, id DESC"
    );
}
