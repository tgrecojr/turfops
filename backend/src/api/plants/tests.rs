use super::*;

fn request(common: Option<&str>, kind: &str, planted: Option<&str>) -> CreatePlantRequest {
    CreatePlantRequest {
        common_name: common.map(Into::into),
        scientific_name: None,
        plant_type: kind.into(),
        location: None,
        planting_date: planted.map(Into::into),
        notes: None,
    }
}

fn message(req: CreatePlantRequest) -> String {
    match validate_new_plant(&req) {
        Err(TurfOpsError::InvalidData(msg)) => msg,
        other => panic!("expected a 400, got ok={}", other.is_ok()),
    }
}

// @spec PLANT-API-004
#[test]
fn a_bad_new_plant_is_named_in_the_owners_words() {
    let msg = message(request(Some("  "), "Shrub", None));
    assert!(
        msg.contains("common name") && !msg.contains("common_name"),
        "{msg}"
    );
    let msg = message(request(Some("Holly"), "Bush", None));
    assert!(msg.contains("Bush"), "{msg}");
    let msg = message(request(Some("Holly"), "Shrub", Some("04/01/2024")));
    assert!(
        msg.contains("04/01/2024")
            && msg.contains("planting date")
            && !msg.contains("planting_date"),
        "{msg}"
    );
    let ok = validate_new_plant(&request(Some("Holly"), "Shrub", Some("2024-04-01"))).unwrap();
    assert_eq!(ok.input, "Holly");
}

// @spec PLANT-API-013
#[test]
fn deleting_an_unknown_plant_is_not_found() {
    assert!(deleted(1, 4).is_ok());
    assert!(matches!(deleted(0, 4), Err(TurfOpsError::NotFound(_))));
}
