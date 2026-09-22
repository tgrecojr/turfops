use super::rotation::recommend_rotation;
use super::*;
use crate::models::ApplicationType;
use chrono::NaiveDate;

#[test]
fn product_lookup() {
    assert_eq!(
        frac_class_for_product("Heritage TL"),
        Some(FracClass::Frac11)
    );
    assert_eq!(
        frac_class_for_product("Banner MAXX"),
        Some(FracClass::Frac3)
    );
    assert_eq!(
        frac_class_for_product("Daconil Action"),
        Some(FracClass::FracM5)
    );
    assert_eq!(
        frac_class_for_product("Mancozeb DG"),
        Some(FracClass::FracM3)
    );
    assert_eq!(
        frac_class_for_product("Cleary's 3336"),
        Some(FracClass::Frac1)
    );
    assert_eq!(frac_class_for_product("random stuff"), None);
}

#[test]
fn multisite_identification() {
    assert!(FracClass::FracM3.is_multisite());
    assert!(FracClass::FracM5.is_multisite());
    assert!(!FracClass::Frac1.is_multisite());
    assert!(!FracClass::Frac11.is_multisite());
}

// --- analyze_fungicide_rotation tests ---

fn test_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()
}

fn make_fungicide_app(product: Option<&str>, days_ago: i64) -> Application {
    use chrono::Utc;
    let date = test_today() - chrono::Duration::days(days_ago);
    Application {
        id: None,
        lawn_profile_id: 1,
        application_type: ApplicationType::Fungicide,
        product_name: product.map(|s| s.to_string()),
        application_date: date,
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        weather_snapshot: None,
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

#[test]
fn rotation_empty_history() {
    let advice = analyze_fungicide_rotation(&[], test_today());
    assert_eq!(advice.total_apps_this_season, 0);
    assert!(advice.last_class.is_none());
    assert_eq!(advice.consecutive_same_class, 0);
    assert!(advice.recommended_next.is_none());
    assert!(advice.rotation_warning.is_none());
}

#[test]
fn rotation_single_known_product() {
    let apps = vec![make_fungicide_app(Some("Heritage TL"), 10)];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 1);
    assert_eq!(advice.last_class, Some(FracClass::Frac11));
    assert_eq!(advice.consecutive_same_class, 1);
    assert_eq!(advice.recommended_next, Some(FracClass::Frac3));
    assert!(advice.rotation_warning.is_none()); // only 1 consecutive
}

#[test]
fn rotation_consecutive_same_class() {
    let apps = vec![
        make_fungicide_app(Some("Heritage TL"), 30),
        make_fungicide_app(Some("Insignia"), 14),
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 2);
    assert_eq!(advice.last_class, Some(FracClass::Frac11));
    assert_eq!(advice.consecutive_same_class, 2);
    assert!(advice.rotation_warning.is_some());
    let warning = advice.rotation_warning.unwrap();
    assert!(warning.contains("FRAC 11"));
    assert!(warning.contains("Rotate to"));
}

#[test]
fn rotation_mixed_classes_no_warning() {
    let apps = vec![
        make_fungicide_app(Some("Heritage TL"), 30), // FRAC 11
        make_fungicide_app(Some("Banner MAXX"), 14), // FRAC 3
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 2);
    assert_eq!(advice.last_class, Some(FracClass::Frac3));
    assert_eq!(advice.consecutive_same_class, 1);
    assert!(advice.rotation_warning.is_none());
}

#[test]
fn rotation_unknown_products_handled() {
    let apps = vec![
        make_fungicide_app(Some("Mystery Spray"), 30),
        make_fungicide_app(Some("Unknown Product"), 14),
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 2);
    assert!(advice.last_class.is_none()); // can't resolve either
    assert_eq!(advice.consecutive_same_class, 0);
    assert!(advice.rotation_warning.is_none());
}

#[test]
fn rotation_multisite_excluded() {
    // Multi-site fungicides should not count for rotation
    let apps = vec![
        make_fungicide_app(Some("Heritage TL"), 30),    // FRAC 11
        make_fungicide_app(Some("Daconil Action"), 14), // FRAC M5 (multi-site)
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 2);
    // Last single-site class should be FRAC 11 (Daconil is excluded)
    assert_eq!(advice.last_class, Some(FracClass::Frac11));
    assert_eq!(advice.consecutive_same_class, 1);
}

#[test]
fn rotation_three_apps_general_warning() {
    let apps = vec![
        make_fungicide_app(Some("Heritage TL"), 42),  // FRAC 11
        make_fungicide_app(Some("Banner MAXX"), 21),  // FRAC 3
        make_fungicide_app(Some("Cleary's 3336"), 7), // FRAC 1
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 3);
    assert_eq!(advice.last_class, Some(FracClass::Frac1));
    assert_eq!(advice.consecutive_same_class, 1); // no consecutive same-class
                                                  // Should still get a general warning at 3+ apps
    assert!(advice.rotation_warning.is_some());
    let warning = advice.rotation_warning.unwrap();
    assert!(warning.contains("3 times this season"));
}

#[test]
fn rotation_recommend_avoids_last_class() {
    // If last was FRAC 11, should recommend FRAC 3 (next in rotation order)
    assert_eq!(
        recommend_rotation(FracClass::Frac11),
        Some(FracClass::Frac3)
    );
    // If last was FRAC 3, should recommend FRAC 11
    assert_eq!(
        recommend_rotation(FracClass::Frac3),
        Some(FracClass::Frac11)
    );
    // If last was FRAC 1, should recommend FRAC 11
    assert_eq!(
        recommend_rotation(FracClass::Frac1),
        Some(FracClass::Frac11)
    );
}

#[test]
fn rotation_handles_newest_first_history() {
    // The application queries return ORDER BY application_date DESC.
    let apps = vec![
        make_fungicide_app(Some("Banner MAXX"), 5),
        make_fungicide_app(Some("Heritage TL"), 25),
        make_fungicide_app(Some("Heritage TL"), 45),
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.last_class, Some(FracClass::Frac3));
    assert_eq!(advice.consecutive_same_class, 1);
    // Three apps earn the general heads-up, which must name the class used *last*
    // and must not be the consecutive-use warning.
    let note = advice.rotation_warning.expect("3+ apps note");
    assert!(note.contains(&FracClass::Frac3.to_string()), "{note}");
    assert!(!note.contains("Rotate to"), "{note}");
}

#[test]
fn rotation_ignores_plant_applications() {
    let mut shrub = make_fungicide_app(Some("Heritage TL"), 3);
    shrub.plant_id = Some(4);
    let apps = vec![make_fungicide_app(Some("Heritage TL"), 20), shrub];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.total_apps_this_season, 1);
    assert!(advice.rotation_warning.is_none());
}

#[test]
fn premixes_and_homeowner_products_resolve() {
    assert_eq!(
        frac_classes_for_product("Headway G"),
        vec![FracClass::Frac11, FracClass::Frac3]
    );
    // Listing both actives works as well as the trade name.
    assert_eq!(
        frac_classes_for_product("azoxystrobin + propiconazole"),
        vec![FracClass::Frac3, FracClass::Frac11]
    );
    assert_eq!(
        frac_classes_for_product("Scotts DiseaseEx"),
        vec![FracClass::Frac11]
    );
    assert_eq!(
        frac_classes_for_product("Azoxy 2SC"),
        vec![FracClass::Frac11]
    );
    // "Eagle" alone is not a fungicide name.
    assert!(frac_classes_for_product("Bald Eagle lawn mix").is_empty());
    assert_eq!(
        frac_classes_for_product("Eagle 20EW"),
        vec![FracClass::Frac3]
    );
}

#[test]
fn recorded_classes_win_over_the_product_name() {
    let mut app = make_fungicide_app(Some("Store-brand lawn fungicide"), 3);
    assert!(classes_of(&app).is_empty());
    app.frac_classes = Some(vec![FracClass::Frac7]);
    assert_eq!(classes_of(&app), vec![FracClass::Frac7]);
    // An empty pick means "not recorded": fall back to the name.
    let mut named = make_fungicide_app(Some("Heritage TL"), 3);
    named.frac_classes = Some(vec![]);
    assert_eq!(classes_of(&named), vec![FracClass::Frac11]);
}

#[test]
fn a_premix_counts_toward_rotation_for_each_class() {
    let apps = vec![
        make_fungicide_app(Some("Heritage TL"), 30),
        make_fungicide_app(Some("Headway G"), 10),
    ];
    let advice = analyze_fungicide_rotation(&apps, test_today());
    assert_eq!(advice.last_class, Some(FracClass::Frac11));
    assert_eq!(advice.consecutive_same_class, 2);
    assert!(advice.rotation_warning.is_some());
}
