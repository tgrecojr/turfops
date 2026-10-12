//! The action, the headline and the shape of a plan.

use super::*;

const DISEASES: [Disease; 5] = [
    Disease::BrownPatch,
    Disease::DollarSpot,
    Disease::PythiumBlight,
    Disease::GrayLeafSpot,
    Disease::RedThread,
];
const TIERS: [RiskTier; 4] = [
    RiskTier::Low,
    RiskTier::Moderate,
    RiskTier::High,
    RiskTier::Severe,
];

// @spec DIS-MGMT-ACT-001
#[test]
fn the_action_follows_the_headline_tier() {
    let today = date(7, 20);
    let none = DiseaseContext::default();
    let covered = ctx_with(vec![app(7, 18, "Segway"), app(7, 18, "Heritage G")]);
    for disease in DISEASES {
        let action = |tier, ctx: &DiseaseContext| plan(disease, tier, &[], today, ctx).action;
        assert_eq!(action(RiskTier::Low, &covered), ManagementAction::NoAction);
        assert_eq!(
            action(RiskTier::Moderate, &covered),
            ManagementAction::Monitor
        );
        for tier in [RiskTier::High, RiskTier::Severe] {
            let (bare, protected) = if disease == Disease::RedThread {
                (ManagementAction::Cultural, ManagementAction::Cultural)
            } else {
                (
                    ManagementAction::ApplyPreventative,
                    ManagementAction::Protected,
                )
            };
            assert_eq!(action(tier, &none), bare, "{disease:?} {tier:?}");
            if disease != Disease::DollarSpot {
                assert_eq!(action(tier, &covered), protected, "{disease:?} {tier:?}");
            }
        }
    }
}

// @spec DIS-MGMT-ACT-002
#[test]
fn a_plan_carries_every_part_with_notes_in_order() {
    let ctx = DiseaseContext {
        rotation_warning: Some("Your last 2 applications were FRAC 11.".into()),
        ..Default::default()
    };
    for disease in DISEASES {
        let m = plan(disease, RiskTier::High, &[], date(7, 20), &ctx);
        let spec = programs::program_for(disease);
        assert!(!m.headline.is_empty());
        assert_eq!(m.cultural.len(), spec.cultural.len());
        for program in [&m.preventative, &m.curative] {
            assert!(!program.when.is_empty() && !program.interval.is_empty());
            assert!(!program.guidance.is_empty() && !program.options.is_empty());
        }
        assert!(m.protection.is_none());
        let n = spec.notes.len();
        let program_notes: Vec<&str> = m.notes[..n].iter().map(String::as_str).collect();
        assert_eq!(program_notes, spec.notes.to_vec());
        assert_eq!(m.notes[n], "Your last 2 applications were FRAC 11.");
        assert_eq!(m.notes[n + 1], LABEL_NOTE);
        assert_eq!(m.notes.len(), n + 2);
    }
}

// @spec DIS-MGMT-ACT-003
#[test]
fn red_thread_says_feed_with_an_amount_until_the_lawn_is_fed() {
    let hungry = plan(
        Disease::RedThread,
        RiskTier::High,
        &[],
        date(5, 10),
        &DiseaseContext {
            days_since_fertilizer: Some(61),
            ..Default::default()
        },
    );
    assert!(
        hungry
            .headline
            .contains("Feed the lawn: about 0.5 lb N/1000 sq ft"),
        "{}",
        hungry.headline
    );
    assert!(hungry.headline.contains("A fungicide is rarely needed"));

    let fed = plan(
        Disease::RedThread,
        RiskTier::High,
        &[],
        date(5, 10),
        &DiseaseContext {
            days_since_fertilizer: Some(60),
            ..Default::default()
        },
    );
    assert!(fed.headline.contains("stay the course"), "{}", fed.headline);
    assert!(!fed.headline.contains("Feed the lawn"));
}

// @spec DIS-MGMT-ACT-004
#[test]
fn a_protected_headline_names_product_class_dates_and_the_next_pick() {
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 15, "Heritage G")]);
    let high = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    assert_eq!(high.action, ManagementAction::Protected);
    let h = &high.headline;
    for part in [
        "Heritage G",
        "FRAC 11 (Strobilurins)",
        "applied 7/15",
        "through about 8/5",
        "rotating to FRAC 7 (SDHI)",
    ] {
        assert!(h.contains(part), "{part}: {h}");
    }
    assert!(!h.contains("scout anyway"));

    let severe = plan(Disease::BrownPatch, RiskTier::Severe, &[], today, &ctx);
    assert_eq!(severe.action, ManagementAction::Protected);
    assert!(severe.headline.contains("through about 7/29"));
    assert!(
        severe.headline.contains("scout anyway"),
        "{}",
        severe.headline
    );
}

// @spec DIS-MGMT-ACT-005
#[test]
fn an_apply_headline_leads_by_tier_and_ends_with_the_shelf() {
    let today = date(7, 20);
    let none = DiseaseContext::default();
    let pythium_high = plan(Disease::PythiumBlight, RiskTier::High, &[], today, &none);
    assert!(pythium_high
        .headline
        .starts_with("Protect vulnerable areas"));
    assert!(pythium_high.headline.contains("FRAC 21 (QiI)"));

    let severe = plan(Disease::BrownPatch, RiskTier::Severe, &[], today, &none);
    assert!(
        severe.headline.starts_with("Apply now: FRAC 11"),
        "{}",
        severe.headline
    );
    assert!(severe.headline.contains("follow the curative program"));
    assert!(severe
        .headline
        .ends_with(" Nothing on your shelf is in FRAC 11 (Strobilurins)."));

    let owned = DiseaseContext {
        shelf: vec![shelf("Heritage G", &[FracClass::Frac11])],
        ..Default::default()
    };
    let high = plan(Disease::BrownPatch, RiskTier::High, &[], today, &owned);
    assert!(high.headline.starts_with("Apply a preventative: FRAC 11"));
    assert!(!high.headline.contains("curative"));
    assert!(high.headline.ends_with(" You have Heritage G on hand."));
}

// @spec DIS-MGMT-ACT-006
#[test]
fn no_management_text_states_a_fungicide_rate() {
    let rate_words = [
        " oz", "fl oz", " ml", " gal", "lb ai", "lb a.i", "/acre", "per acre",
    ];
    let ctx = DiseaseContext {
        shelf: vec![shelf("Heritage G", &[FracClass::Frac11])],
        ..ctx_with(vec![app(7, 15, "Heritage G")])
    };
    for disease in DISEASES {
        for tier in TIERS {
            let m = plan(disease, tier, &[], date(7, 20), &ctx);
            let mut texts = vec![m.headline.clone()];
            texts.extend(m.cultural.clone());
            texts.extend(m.notes.clone());
            for program in [&m.preventative, &m.curative] {
                texts.extend(program.guidance.clone());
                texts.push(program.when.clone());
                texts.extend(program.options.iter().filter_map(|o| o.note.clone()));
            }
            for text in &texts {
                let lower = text.to_lowercase();
                for word in rate_words {
                    assert!(!lower.contains(word), "{disease:?}: {text}");
                }
            }
            assert!(m.preventative.guidance[0].contains("label's preventative rate"));
            assert!(m.curative.guidance[0].contains("label's curative"));
        }
    }
}
