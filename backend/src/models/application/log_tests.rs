//! The application log's types, nitrogen definition and JSON shape.

use super::*;
use ApplicationType::*;

fn turf(kind: ApplicationType) -> Application {
    Application {
        id: Some(3),
        lawn_profile_id: 1,
        application_type: kind,
        product_name: None,
        application_date: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        plant_name: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: Utc::now(),
    }
}

// @spec APP-LOG-001
#[test]
fn every_type_has_its_scope() {
    let expected = [
        (
            ApplicationScope::TurfOnly,
            vec![PreEmergent, PostEmergent, GrubControl, Overseed],
        ),
        (
            ApplicationScope::TurfOnly,
            vec![Aeration, Dethatching, Lime, Sulfur, Mowing],
        ),
        (
            ApplicationScope::Universal,
            vec![Fertilizer, Fungicide, Insecticide, Wetting, Other],
        ),
        (
            ApplicationScope::PlantRequired,
            vec![Pruning, PlantFertilizer, Mulching],
        ),
        (
            ApplicationScope::PlantRequired,
            vec![Deadheading, WinterProtection],
        ),
    ];
    let mut seen = 0;
    for (scope, kinds) in expected {
        for kind in kinds {
            assert_eq!(kind.scope(), scope, "{kind:?}");
            seen += 1;
        }
    }
    assert_eq!(seen, 19, "every type is listed");
}

// @spec APP-LOG-003
#[test]
fn lawn_nitrogen_is_n_percent_times_rate_for_turf_entries_only() {
    let feed = |n: Option<f64>, rate: Option<f64>| Application {
        nitrogen_pct: n,
        rate_per_1000sqft: rate,
        ..turf(Fertilizer)
    };
    assert_eq!(feed(Some(46.0), Some(2.0)).turf_nitrogen_lbs(), Some(0.92));
    assert_eq!(feed(Some(0.0), Some(2.0)).turf_nitrogen_lbs(), None);
    assert_eq!(feed(Some(46.0), Some(0.0)).turf_nitrogen_lbs(), None);
    assert_eq!(feed(None, Some(2.0)).turf_nitrogen_lbs(), None);
    assert_eq!(feed(Some(46.0), None).turf_nitrogen_lbs(), None);

    let mut shrub = feed(Some(46.0), Some(2.0));
    shrub.plant_id = Some(4);
    assert_eq!(shrub.turf_nitrogen_lbs(), None, "not a turf entry");

    let urea = Application {
        product_name: Some("Urea 46-0-0".into()),
        ..feed(None, Some(2.0))
    };
    assert_eq!(urea.turf_nitrogen_lbs(), None, "never read from the name");
}

// @spec APP-LOG-005
#[test]
fn mowing_is_an_ordinary_turf_entry_with_no_cut_height() {
    assert_eq!(Mowing.scope(), ApplicationScope::TurfOnly);
    assert_eq!("mow".parse::<ApplicationType>(), Ok(Mowing));
    let json = serde_json::to_value(turf(Mowing)).unwrap();
    assert_eq!(json["application_type"], "Mowing");
    let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    assert!(
        keys.iter()
            .all(|k| !k.contains("height") && !k.contains("cut")),
        "{keys:?}"
    );
}

// @spec APP-LOG-007
#[test]
fn unset_links_and_dates_are_left_out_of_the_json() {
    let bare = serde_json::to_value(turf(Fungicide)).unwrap();
    for key in ["plant_id", "follow_up_date", "frac_classes", "product_id"] {
        assert!(bare.get(key).is_none(), "{key} in {bare}");
    }
    assert!(
        bare.get("product_name").is_some(),
        "other fields stay, as null"
    );

    let full = Application {
        plant_id: Some(4),
        follow_up_date: NaiveDate::from_ymd_opt(2026, 5, 15),
        frac_classes: Some(vec![super::super::FracClass::Frac11]),
        product_id: Some(9),
        ..turf(Fungicide)
    };
    let json = serde_json::to_value(full).unwrap();
    for key in ["plant_id", "follow_up_date", "frac_classes", "product_id"] {
        assert!(json.get(key).is_some(), "{key} missing from {json}");
    }
}
