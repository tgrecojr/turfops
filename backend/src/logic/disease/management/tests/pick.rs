//! The rotation-aware, shelf-aware pick.

use super::*;

// @spec DIS-MGMT-PICK-001
#[test]
fn the_pick_is_the_best_unrestricted_class_not_used_last() {
    let today = date(7, 20);
    // Gray leaf spot: FRAC 1 first; once used, FRAC 11 (Excellent) — never M5 or Fair 3.
    let fresh = plan(
        Disease::GrayLeafSpot,
        RiskTier::High,
        &[],
        today,
        &DiseaseContext::default(),
    );
    assert_eq!(recommended(&fresh.preventative), vec![FracClass::Frac1]);
    let after_1 = ctx_with(vec![record(6, 1, "Cleary", Some(FracClass::Frac1))]);
    let rotated = plan(Disease::GrayLeafSpot, RiskTier::High, &[], today, &after_1);
    assert_eq!(recommended(&rotated.preventative), vec![FracClass::Frac11]);
    // Curative picks come from curative options only: after FRAC 21, 28 and 4 on
    // Pythium the preventative pick is P07, and no curative option is left to pick.
    let used: Vec<FungicideRecord> = [FracClass::Frac21, FracClass::Frac28, FracClass::Frac4]
        .into_iter()
        .map(|class| record(6, 1, "Pythium kit", Some(class)))
        .collect();
    let m = plan(
        Disease::PythiumBlight,
        RiskTier::High,
        &[],
        today,
        &ctx_with(used),
    );
    assert_eq!(recommended(&m.preventative), vec![FracClass::FracP07]);
    assert!(recommended(&m.curative).is_empty());
    assert!(m
        .curative
        .options
        .iter()
        .all(|o| o.frac_class != FracClass::FracP07));
}

// @spec DIS-MGMT-PICK-002
#[test]
fn among_equals_an_owned_class_wins_then_list_order() {
    let today = date(7, 20);
    // Red thread: FRAC 11 and 7 tie at Excellent; the list puts 11 first.
    let none = plan(
        Disease::RedThread,
        RiskTier::High,
        &[],
        today,
        &DiseaseContext::default(),
    );
    assert_eq!(recommended(&none.preventative), vec![FracClass::Frac11]);
    let owns_7 = DiseaseContext {
        shelf: vec![shelf("Velista", &[FracClass::Frac7])],
        ..Default::default()
    };
    let m = plan(Disease::RedThread, RiskTier::High, &[], today, &owns_7);
    assert_eq!(recommended(&m.preventative), vec![FracClass::Frac7]);
    // Owning a Good class never beats an Excellent one.
    let owns_3 = DiseaseContext {
        shelf: vec![shelf("Banner Maxx", &[FracClass::Frac3])],
        ..Default::default()
    };
    let m = plan(Disease::RedThread, RiskTier::High, &[], today, &owns_3);
    assert_eq!(recommended(&m.preventative), vec![FracClass::Frac11]);
}

// @spec DIS-MGMT-PICK-004
#[test]
fn each_option_shows_pick_last_used_and_shelf_products() {
    let today = date(7, 20);
    let mut ctx = ctx_with(vec![record(6, 20, "Heritage G", Some(FracClass::Frac11))]);
    ctx.shelf = vec![
        shelf("Velista", &[FracClass::Frac7]),
        shelf("Headway G", &[FracClass::Frac11, FracClass::Frac3]),
    ];
    let m = plan(Disease::BrownPatch, RiskTier::High, &[], today, &ctx);
    for program in [&m.preventative, &m.curative] {
        let by_class = |class| {
            program
                .options
                .iter()
                .find(|o| o.frac_class == class)
                .unwrap()
        };
        let strobi = by_class(FracClass::Frac11);
        assert!(strobi.last_used && !strobi.recommended);
        assert_eq!(
            strobi
                .on_hand
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Headway G"]
        );
        let sdhi = by_class(FracClass::Frac7);
        assert!(sdhi.recommended && !sdhi.last_used);
        assert_eq!(sdhi.on_hand[0].name, "Velista");
        assert_eq!(by_class(FracClass::Frac3).on_hand[0].name, "Headway G");
        assert!(by_class(FracClass::Frac1).on_hand.is_empty());
    }
}
