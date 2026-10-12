//! Protection from logged fungicides and the outlook heads-up.

use super::*;

fn fungicide_log(product: &str, on: NaiveDate) -> crate::models::Application {
    crate::models::Application {
        id: None,
        lawn_profile_id: 1,
        application_type: crate::models::ApplicationType::Fungicide,
        product_name: Some(product.into()),
        application_date: on,
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
        created_at: chrono::Utc::now(),
    }
}

fn protected_by(
    disease: Disease,
    tier: RiskTier,
    today: NaiveDate,
    ctx: &DiseaseContext,
) -> Option<String> {
    plan(disease, tier, &[], today, ctx)
        .protection
        .map(|p| p.product)
}

// @spec DIS-MGMT-PROT-001
#[test]
fn protection_is_the_newest_good_record_still_in_its_window() {
    let today = date(7, 20);
    // Newest first, as the log builds it: a Fair-rated class on gray leaf spot doesn't
    // count, so the older Excellent one does.
    let ctx = ctx_with(vec![
        record(7, 25, "Future", Some(FracClass::Frac1)),
        record(7, 18, "Banner", Some(FracClass::Frac3)),
        record(7, 10, "Cleary", Some(FracClass::Frac1)),
        record(7, 5, "Heritage", Some(FracClass::Frac11)),
    ]);
    assert_eq!(
        protected_by(Disease::GrayLeafSpot, RiskTier::High, today, &ctx).as_deref(),
        Some("Cleary")
    );
    // A class that isn't one of the disease's options never protects.
    let pythium_only = ctx_with(vec![record(7, 18, "Banner", Some(FracClass::Frac3))]);
    assert!(protected_by(Disease::PythiumBlight, RiskTier::High, today, &pythium_only).is_none());
    // The window ends on its last day.
    let edge = ctx_with(vec![record(6, 29, "Cleary", Some(FracClass::Frac1))]);
    assert!(protected_by(Disease::BrownPatch, RiskTier::High, today, &edge).is_some());
    let lapsed = ctx_with(vec![record(6, 28, "Cleary", Some(FracClass::Frac1))]);
    assert!(protected_by(Disease::BrownPatch, RiskTier::High, today, &lapsed).is_none());
}

// @spec DIS-MGMT-PROT-002
#[test]
fn systemic_classes_cover_21_days_contact_and_phosphonate_14_less_7_at_severe() {
    let today = date(7, 20);
    let through = |disease, tier, class| {
        let ctx = ctx_with(vec![record(7, 19, "X", Some(class))]);
        plan(disease, tier, &[], today, &ctx)
            .protection
            .map(|p| p.protected_through)
    };
    assert_eq!(
        through(Disease::BrownPatch, RiskTier::High, FracClass::Frac3),
        Some(date(8, 9))
    );
    assert_eq!(
        through(Disease::BrownPatch, RiskTier::Severe, FracClass::Frac3),
        Some(date(8, 2))
    );
    assert_eq!(
        through(Disease::BrownPatch, RiskTier::High, FracClass::FracM5),
        Some(date(8, 2))
    );
    assert_eq!(
        through(Disease::BrownPatch, RiskTier::Severe, FracClass::FracM5),
        Some(date(7, 26))
    );
    assert_eq!(
        through(Disease::PythiumBlight, RiskTier::High, FracClass::FracP07),
        Some(date(8, 2))
    );
    assert_eq!(
        through(Disease::PythiumBlight, RiskTier::Severe, FracClass::FracP07),
        Some(date(7, 26))
    );
    assert_eq!(protection_days(FracClass::FracM3, RiskTier::High), 14);
    assert_eq!(protection_days(FracClass::Frac21, RiskTier::Moderate), 21);
}

// @spec DIS-MGMT-PROT-003
#[test]
fn a_strobilurin_is_not_pythium_protection_at_severe() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![record(7, 18, "Heritage", Some(FracClass::Frac11))]);
    assert!(protected_by(Disease::PythiumBlight, RiskTier::High, today, &ctx).is_some());
    assert!(protected_by(Disease::PythiumBlight, RiskTier::Severe, today, &ctx).is_none());
    // The same application still protects brown patch at Severe.
    assert!(protected_by(Disease::BrownPatch, RiskTier::Severe, today, &ctx).is_some());
}

// @spec DIS-MGMT-PROT-004
#[test]
fn logged_chlorothalonil_protects_but_is_never_recommended() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![record(7, 18, "Daconil", Some(FracClass::FracM5))]);
    for disease in [
        Disease::BrownPatch,
        Disease::DollarSpot,
        Disease::GrayLeafSpot,
    ] {
        let m = plan(disease, RiskTier::High, &[], today, &ctx);
        assert_eq!(m.action, ManagementAction::Protected, "{disease:?}");
        let m5 = m
            .preventative
            .options
            .iter()
            .find(|o| o.frac_class == FracClass::FracM5)
            .unwrap();
        assert!(m5.restricted && !m5.recommended, "{disease:?}");
        assert!(!m.headline.contains("rotating to FRAC M5"));
    }
}

// @spec DIS-MGMT-PROT-005
#[test]
fn the_heads_up_is_for_the_first_high_day_within_three_days() {
    let today = date(7, 20);
    let daily = [
        forecast(7, 21, RiskTier::Moderate),
        forecast(7, 22, RiskTier::High),
        forecast(7, 23, RiskTier::Severe),
    ];
    let none = DiseaseContext::default();
    for tier in [RiskTier::Low, RiskTier::Moderate] {
        let m = plan(Disease::BrownPatch, tier, &daily, today, &none);
        assert!(
            m.headline.contains("forecast to reach High on 7/22"),
            "{}",
            m.headline
        );
        assert!(m
            .headline
            .contains("FRAC 11 (Strobilurins), e.g. azoxystrobin"));
    }
    let covered = ctx_with(vec![record(7, 12, "Heritage G", Some(FracClass::Frac11))]);
    let m = plan(Disease::BrownPatch, RiskTier::Low, &daily, today, &covered);
    assert!(
        m.headline.contains(
            "Heritage G (FRAC 11 (Strobilurins)) applied 7/12 covers brown patch through about 8/2"
        ),
        "{}",
        m.headline
    );
    // Today itself and day four are outside the outlook.
    let outside = [
        forecast(7, 20, RiskTier::Severe),
        forecast(7, 24, RiskTier::Severe),
    ];
    let quiet = plan(Disease::BrownPatch, RiskTier::Low, &outside, today, &none);
    assert!(!quiet.headline.contains("forecast"), "{}", quiet.headline);
}

// @spec DIS-MGMT-PROT-006
#[test]
fn without_forecast_days_there_is_no_heads_up() {
    let today = date(7, 20);
    // Only observed days: nothing after today to warn about.
    let observed: Vec<DailyRisk> = (14..=20)
        .map(|d| DailyRisk {
            is_forecast: false,
            ..forecast(7, d, RiskTier::Severe)
        })
        .collect();
    for tier in [RiskTier::Low, RiskTier::Moderate] {
        let m = plan(
            Disease::BrownPatch,
            tier,
            &observed,
            today,
            &DiseaseContext::default(),
        );
        assert!(!m.headline.contains("forecast"), "{}", m.headline);
    }
}

// @spec DIS-MGMT-FRAC-002
#[test]
fn a_premix_protects_for_each_class_it_contains() {
    // Headway G (FRAC 11 + 3) logged once: dollar spot has no FRAC 11 option, so its
    // protection comes from the FRAC 3 record of the same application.
    let today = date(7, 20);
    let history = [fungicide_log("Headway G", date(7, 15))];
    let ctx = crate::logic::disease::context_from_history(&history, today);
    let dollar = plan(Disease::DollarSpot, RiskTier::High, &[], today, &ctx);
    assert_eq!(dollar.protection.unwrap().class_label, "FRAC 3 (DMIs)");
    let pythium = plan(Disease::PythiumBlight, RiskTier::High, &[], today, &ctx);
    assert_eq!(
        pythium.protection.unwrap().class_label,
        "FRAC 11 (Strobilurins)"
    );
}

// @spec DIS-MGMT-FRAC-003
#[test]
fn an_unclassified_fungicide_earns_no_protection_and_no_rotation_credit() {
    let today = date(7, 20);
    let history = [fungicide_log("Store-brand lawn fungicide", date(7, 15))];
    let ctx = crate::logic::disease::context_from_history(&history, today);
    assert_eq!(ctx.fungicide_apps.len(), 1);
    assert!(ctx.fungicide_apps[0].class.is_none());
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert!(m.protection.is_none());
    assert!(m.preventative.options.iter().all(|o| !o.last_used));
    assert_eq!(recommended(&m.preventative), vec![FracClass::Frac11]);
}
