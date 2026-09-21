use super::*;
use crate::logic::disease::test_support::date;

mod outlook;

fn app(m: u32, d: u32, product: &str) -> FungicideRecord {
    FungicideRecord {
        date: date(m, d),
        product: product.into(),
        class: crate::models::frac_class_for_product(product),
    }
}

fn ctx_with(apps: Vec<FungicideRecord>) -> DiseaseContext {
    DiseaseContext {
        fungicide_apps: apps,
        ..Default::default()
    }
}

fn forecast(m: u32, d: u32, tier: RiskTier) -> DailyRisk {
    DailyRisk {
        date: date(m, d),
        value: 0.0,
        tier,
        is_forecast: true,
        partial: false,
    }
}

fn recommended(program: &FungicideProgram) -> Vec<FracClass> {
    program
        .options
        .iter()
        .filter(|o| o.recommended)
        .map(|o| o.frac_class)
        .collect()
}

#[test]
fn tiers_map_to_actions() {
    let today = date(7, 20);
    let ctx = DiseaseContext::default();
    let action = |tier| plan(Disease::BrownPatch, tier, &[], today, &ctx).action;
    assert_eq!(action(RiskTier::Low), ManagementAction::NoAction);
    assert_eq!(action(RiskTier::Moderate), ManagementAction::Monitor);
    assert_eq!(action(RiskTier::High), ManagementAction::ApplyPreventative);
    assert_eq!(
        action(RiskTier::Severe),
        ManagementAction::ApplyPreventative
    );
}

#[test]
fn exactly_one_pick_per_program_and_never_a_restricted_class() {
    let today = date(7, 20);
    let ctx = DiseaseContext::default();
    for disease in [
        Disease::BrownPatch,
        Disease::DollarSpot,
        Disease::PythiumBlight,
        Disease::GrayLeafSpot,
        Disease::RedThread,
    ] {
        let m = plan(disease, RiskTier::High, &[], today, &ctx);
        for program in [&m.preventative, &m.curative] {
            let picks = recommended(program);
            assert_eq!(picks.len(), 1, "{disease:?}");
            assert_ne!(picks[0], FracClass::FracM5, "{disease:?}");
        }
        assert!(m.curative.options.len() <= m.preventative.options.len());
    }
}

#[test]
fn brown_patch_defaults_to_strobilurin_then_rotates_away() {
    let today = date(7, 20);
    let fresh = plan(
        Disease::BrownPatch,
        RiskTier::High,
        &[],
        today,
        &DiseaseContext::default(),
    );
    assert_eq!(recommended(&fresh.preventative), vec![FracClass::Frac11]);

    // Heritage 30 days ago: outside the protection window, but still the last class used.
    let ctx = ctx_with(vec![app(6, 20, "Heritage G")]);
    let rotated = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(rotated.action, ManagementAction::ApplyPreventative);
    assert_eq!(recommended(&rotated.preventative), vec![FracClass::Frac7]);
    let heritage = rotated
        .preventative
        .options
        .iter()
        .find(|o| o.frac_class == FracClass::Frac11)
        .unwrap();
    assert!(heritage.last_used && !heritage.recommended);
    assert!(rotated.headline.contains("FRAC 7"));
}

#[test]
fn recent_effective_application_means_protected() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 10, "Heritage G")]);
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(m.action, ManagementAction::Protected);
    let status = m.protection.unwrap();
    assert_eq!(status.protected_through, date(7, 31));
    assert_eq!(status.days_remaining, 11);
    assert!(m.headline.starts_with("Protected"));
}

#[test]
fn severe_pressure_shortens_the_protection_window() {
    let today = date(7, 28);
    let ctx = ctx_with(vec![app(7, 10, "Heritage G")]);
    let high = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(high.action, ManagementAction::Protected);
    let severe = plan(Disease::BrownPatch, RiskTier::Severe, &[], today, &ctx);
    assert_eq!(severe.action, ManagementAction::ApplyPreventative);
    assert!(severe.headline.contains("curative program"));
}

#[test]
fn azoxystrobin_is_not_dollar_spot_protection() {
    // PPA-1 does not list azoxystrobin for dollar spot and notes it can enhance it.
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 15, "azoxystrobin")]);
    let dollar = plan(Disease::DollarSpot, RiskTier::High, &[], today, &ctx);
    assert_eq!(dollar.action, ManagementAction::ApplyPreventative);
    assert!(dollar.protection.is_none());
}

#[test]
fn strobilurin_covers_pythium_at_high_but_not_at_severe() {
    // PPA-1 rates azoxystrobin 3 on Pythium blight, but says cyazofamid, mefenoxam and
    // propamocarb are the most efficacious under high pressure.
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 15, "azoxystrobin")]);
    let high = plan(Disease::PythiumBlight, RiskTier::High, &[], today, &ctx);
    assert_eq!(high.action, ManagementAction::Protected);

    let severe = plan(Disease::PythiumBlight, RiskTier::Severe, &[], today, &ctx);
    assert_eq!(severe.action, ManagementAction::ApplyPreventative);
    assert!(severe.protection.is_none());
}

#[test]
fn pythium_specific_chemistry_still_protects_at_severe() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 15, "Segway")]);
    let severe = plan(Disease::PythiumBlight, RiskTier::Severe, &[], today, &ctx);
    assert_eq!(severe.action, ManagementAction::Protected);
}

#[test]
fn pythium_pick_is_cyazofamid_not_mefenoxam() {
    // PPA-1: cyazofamid 3.5 and propamocarb 3.5 outrank mefenoxam 3 (resistance risk).
    let today = date(7, 20);
    let fresh = plan(
        Disease::PythiumBlight,
        RiskTier::Severe,
        &[],
        today,
        &DiseaseContext::default(),
    );
    assert_eq!(recommended(&fresh.preventative), vec![FracClass::Frac21]);
    assert_eq!(recommended(&fresh.curative), vec![FracClass::Frac21]);

    let after_segway = ctx_with(vec![app(6, 1, "Segway")]);
    let rotated = plan(
        Disease::PythiumBlight,
        RiskTier::Severe,
        &[],
        today,
        &after_segway,
    );
    assert_eq!(recommended(&rotated.preventative), vec![FracClass::Frac28]);
}

#[test]
fn thiophanate_methyl_counts_as_brown_patch_protection() {
    // PPA-1 rates thiophanate-methyl 2.5 on brown patch → Good.
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 12, "Cleary's 3336")]);
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(m.action, ManagementAction::Protected);
}

#[test]
fn chlorothalonil_is_frac_m5_and_counts_toward_protection_only() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 14, "Daconil Ultrex")]);
    assert_eq!(ctx.fungicide_apps[0].class, Some(FracClass::FracM5));
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(m.action, ManagementAction::Protected);
    assert_eq!(m.protection.unwrap().protected_through, date(7, 28)); // 14-day contact window
    assert!(m
        .preventative
        .options
        .iter()
        .all(|o| !(o.frac_class == FracClass::FracM5 && o.recommended)));
}

#[test]
fn unknown_products_and_future_dated_apps_are_ignored() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 25, "Heritage G"), app(7, 15, "Mystery Juice")]);
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(m.action, ManagementAction::ApplyPreventative);
    assert_eq!(recommended(&m.preventative), vec![FracClass::Frac11]);
}

#[test]
fn moderate_risk_warns_when_the_outlook_escalates() {
    let today = date(7, 20);
    let ctx = DiseaseContext::default();
    let calm = plan(Disease::BrownPatch, RiskTier::Moderate, &[], today, &ctx);
    assert!(!calm.headline.contains("forecast"));

    let daily = [
        forecast(7, 21, RiskTier::Moderate),
        forecast(7, 22, RiskTier::Severe),
    ];
    let rising = plan(Disease::BrownPatch, RiskTier::Moderate, &daily, today, &ctx);
    assert_eq!(rising.action, ManagementAction::Monitor);
    assert!(rising.headline.contains("Severe on 7/22"));

    let covered = ctx_with(vec![app(7, 12, "Heritage G")]);
    let protected = plan(
        Disease::BrownPatch,
        RiskTier::Moderate,
        &daily,
        today,
        &covered,
    );
    assert_eq!(protected.action, ManagementAction::Monitor);
    assert!(protected.headline.contains("Heritage G"));
    assert!(protected.headline.contains("No action needed"));

    let far = [forecast(7, 24, RiskTier::Severe)];
    let later = plan(Disease::BrownPatch, RiskTier::Moderate, &far, today, &ctx);
    assert!(!later.headline.contains("forecast"));
}

#[test]
fn red_thread_remedy_is_nitrogen_not_fungicide() {
    let today = date(5, 10);
    let hungry = plan(
        Disease::RedThread,
        RiskTier::Severe,
        &[],
        today,
        &DiseaseContext::default(),
    );
    assert_eq!(hungry.action, ManagementAction::Cultural);
    assert!(hungry.headline.starts_with("Feed the lawn"));

    let fed = DiseaseContext {
        days_since_fertilizer: Some(20),
        ..Default::default()
    };
    let m = plan(Disease::RedThread, RiskTier::High, &[], today, &fed);
    assert_eq!(m.action, ManagementAction::Cultural);
    assert!(m.headline.starts_with("Recently fed"));
}

#[test]
fn pythium_curative_program_excludes_preventative_only_chemistry() {
    let m = plan(
        Disease::PythiumBlight,
        RiskTier::Severe,
        &[],
        date(7, 20),
        &DiseaseContext::default(),
    );
    let curative: Vec<FracClass> = m.curative.options.iter().map(|o| o.frac_class).collect();
    assert!(!curative.contains(&FracClass::FracP07));
    assert!(!curative.contains(&FracClass::Frac11));
    assert!(curative.contains(&FracClass::Frac21));
}

#[test]
fn rotation_warning_and_label_note_are_surfaced() {
    let ctx = DiseaseContext {
        rotation_warning: Some("Your last 2 applications were FRAC 11".into()),
        ..Default::default()
    };
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], date(7, 20), &ctx);
    assert!(m.notes.iter().any(|n| n.contains("last 2 applications")));
    assert!(m.notes.last().unwrap().contains("product label"));
}
