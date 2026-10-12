//! Lenient parsing of the profile's types, and their stored / serialized form.

use super::*;
use crate::db::queries::enum_to_db_string;

// @spec APP-PROF-004
#[test]
fn types_parse_leniently_and_store_as_their_variant_names() {
    for (raw, grass) in [
        ("tttf", GrassType::TallFescue),
        ("KBG", GrassType::KentuckyBluegrass),
        ("prg", GrassType::PerennialRyegrass),
        ("Fine Fescue", GrassType::FineFescue),
        ("st. augustine", GrassType::StAugustine),
        ("MIXED", GrassType::Mixed),
    ] {
        assert_eq!(raw.parse::<GrassType>(), Ok(grass), "{raw}");
    }
    assert_eq!("Sandy Loam".parse::<SoilType>(), Ok(SoilType::SandyLoam));
    assert_eq!("CLAY LOAM".parse::<SoilType>(), Ok(SoilType::ClayLoam));
    assert_eq!(
        "In Ground".parse::<IrrigationType>(),
        Ok(IrrigationType::InGround)
    );
    assert_eq!(
        "Hose/Sprinkler".parse::<IrrigationType>(),
        Ok(IrrigationType::Hose)
    );

    let stored = |v: serde_json::Value| v.as_str().unwrap().to_string();
    assert_eq!(
        enum_to_db_string(GrassType::StAugustine).unwrap(),
        "StAugustine"
    );
    assert_eq!(
        stored(serde_json::to_value(GrassType::StAugustine).unwrap()),
        "StAugustine"
    );
    assert_eq!(enum_to_db_string(SoilType::SiltLoam).unwrap(), "SiltLoam");
    assert_eq!(
        stored(serde_json::to_value(SoilType::SiltLoam).unwrap()),
        "SiltLoam"
    );
    assert_eq!(
        enum_to_db_string(IrrigationType::InGround).unwrap(),
        "InGround"
    );
    assert_eq!(
        stored(serde_json::to_value(IrrigationType::InGround).unwrap()),
        "InGround"
    );
}

// @spec APP-PROF-002
#[test]
fn a_profile_holds_name_grass_zone_and_three_optional_facts() {
    let profile = LawnProfile::new("Front".into(), GrassType::TallFescue, "any text".into());
    let json = serde_json::to_value(&profile).unwrap();
    for key in ["name", "grass_type", "usda_zone"] {
        assert!(!json[key].is_null(), "{key}");
    }
    for key in ["soil_type", "lawn_size_sqft", "irrigation_type"] {
        assert!(json[key].is_null(), "{key} is optional and unset");
    }
    assert_eq!(json["usda_zone"], "any text", "the zone is not validated");
}
