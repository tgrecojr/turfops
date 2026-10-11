//! The heads-up for a forecast High/Severe day is judged on that day, at that tier.

use super::*;

#[test]
fn chemistry_that_fails_at_severe_is_not_promised_for_a_severe_day() {
    // Strobilurins count against Pythium at High but not once pressure is Severe.
    let today = date(7, 20);
    let daily = [forecast(7, 22, RiskTier::Severe)];
    let ctx = ctx_with(vec![app(7, 15, "Heritage G")]);
    let plan = plan(
        Disease::PythiumBlight,
        RiskTier::Moderate,
        &daily,
        today,
        &ctx,
    );
    assert_eq!(plan.action, ManagementAction::Monitor);
    assert!(!plan.headline.contains("covers"), "{}", plan.headline);
    assert!(plan.headline.contains("have a preventative on hand"));
}

#[test]
fn severe_pressure_shortens_the_promised_coverage() {
    // Heritage on 7/12: 21 days at High (to 8/2), but only 14 at Severe (to 7/26).
    let today = date(7, 20);
    let ctx = ctx_with(vec![app(7, 12, "Heritage G")]);
    let severe_soon = [forecast(7, 22, RiskTier::Severe)];
    let covered = plan(
        Disease::BrownPatch,
        RiskTier::Moderate,
        &severe_soon,
        today,
        &ctx,
    );
    assert!(
        covered.headline.contains("through about 7/26"),
        "{}",
        covered.headline
    );

    // Applied 7/7: gone by 7/21 under Severe pressure, so the 7/22 day is not covered —
    // although at today's Moderate tier it would read as protected through 7/28.
    let older = ctx_with(vec![app(7, 7, "Heritage G")]);
    let exposed = plan(
        Disease::BrownPatch,
        RiskTier::Moderate,
        &severe_soon,
        today,
        &older,
    );
    assert!(!exposed.headline.contains("covers"), "{}", exposed.headline);
}

#[test]
fn a_low_day_still_gets_the_heads_up() {
    let today = date(7, 20);
    let daily = [forecast(7, 21, RiskTier::Severe)];
    let ctx = DiseaseContext::default();
    let plan = plan(Disease::PythiumBlight, RiskTier::Low, &daily, today, &ctx);
    assert_eq!(plan.action, ManagementAction::NoAction);
    assert!(
        plan.headline.contains("Severe on 7/21"),
        "{}",
        plan.headline
    );

    // Red thread's remedy is nitrogen: a forecast never asks for a fungicide on hand.
    let red = super::plan(Disease::RedThread, RiskTier::Low, &daily, today, &ctx);
    assert!(!red.headline.contains("preventative"), "{}", red.headline);
}

// @spec DIS-MGMT-PROT-007
#[test]
fn red_thread_never_gets_a_fungicide_heads_up() {
    let today = date(5, 10);
    let daily = [forecast(5, 12, RiskTier::Severe)];
    let plan = plan(
        Disease::RedThread,
        RiskTier::Moderate,
        &daily,
        today,
        &DiseaseContext::default(),
    );
    assert!(!plan.headline.contains("preventative"), "{}", plan.headline);
    assert!(!plan.headline.contains("FRAC"), "{}", plan.headline);
}

// @spec DIS-MGMT-PROT-008
#[test]
fn a_partial_forecast_day_does_not_raise_the_outlook() {
    let today = date(7, 20);
    let mut tail = forecast(7, 23, RiskTier::Severe);
    tail.partial = true;
    let plan = plan(
        Disease::BrownPatch,
        RiskTier::Low,
        &[tail],
        today,
        &DiseaseContext::default(),
    );
    assert!(
        !plan.headline.contains("forecast to reach"),
        "{}",
        plan.headline
    );
}
