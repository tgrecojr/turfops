//! Lawn-history amplifiers: seedlings for gray leaf spot, hunger for red thread.

use super::test_support::{date, day, run_of};
use super::*;
use crate::models::Disease;
use crate::models::ManagementAction;

fn wet(mut d: DailyWeather, hours: f64) -> DailyWeather {
    d.hours_rh90 = hours;
    d.leaf_wetness_hours = hours;
    d
}

fn find(risks: Vec<DiseaseRisk>, disease: Disease) -> DiseaseRisk {
    risks.into_iter().find(|r| r.disease == disease).unwrap()
}

fn seeded(days: i64) -> DiseaseContext {
    DiseaseContext {
        days_since_overseed: Some(days),
        // Fed recently, so red thread is not raised in the same runs.
        days_since_fertilizer: Some(10),
        ..Default::default()
    }
}

/// Gray leaf spot at Moderate on the weather alone: 0.5 × 0.6 → index 30.
fn gls_moderate(today: NaiveDate) -> Vec<DailyWeather> {
    run_of(today, 4, |d| wet(day(d, 21.5, 18.0, 26.0, 90.0), 12.0))
}

// @spec DIS-MODEL-AMP-002
#[test]
fn overseeding_within_sixty_days_raises_gray_leaf_spot_from_moderate() {
    let today = date(9, 10);
    let series = gls_moderate(today);
    let tier =
        |ctx: &DiseaseContext| find(assess_all(&series, today, ctx), Disease::GrayLeafSpot).tier;
    assert_eq!(tier(&seeded(200)), RiskTier::Moderate);
    assert_eq!(tier(&seeded(60)), RiskTier::High);
    assert_eq!(tier(&seeded(61)), RiskTier::Moderate);

    // Below Moderate nothing is raised.
    let dry = run_of(today, 4, |d| day(d, 21.5, 18.0, 26.0, 60.0));
    let low = find(assess_all(&dry, today, &seeded(5)), Disease::GrayLeafSpot);
    assert_eq!(low.tier, RiskTier::Low);
    assert!(low.tier_note.is_none());
}

// @spec DIS-MODEL-AMP-003
#[test]
fn no_feeding_in_sixty_days_raises_red_thread_from_moderate() {
    let today = date(5, 10);
    // 1.0 × 0.375 → index 37.5, Moderate on the weather alone.
    let series = run_of(today, 6, |d| wet(day(d, 18.0, 12.0, 22.0, 85.0), 7.0));
    let tier = |fed: Option<i64>| {
        let ctx = DiseaseContext {
            days_since_fertilizer: fed,
            ..Default::default()
        };
        find(assess_all(&series, today, &ctx), Disease::RedThread).tier
    };
    assert_eq!(tier(Some(60)), RiskTier::Moderate);
    assert_eq!(tier(Some(61)), RiskTier::High);
    assert_eq!(tier(None), RiskTier::High);

    let cool_dry = run_of(today, 6, |d| day(d, 18.0, 12.0, 22.0, 60.0));
    let ctx = DiseaseContext::default();
    let low = find(assess_all(&cool_dry, today, &ctx), Disease::RedThread);
    assert_eq!(low.tier, RiskTier::Low);
}

// @spec DIS-MODEL-AMP-004
#[test]
fn a_raised_tier_tops_out_at_severe_and_leaves_the_daily_bars_alone() {
    let today = date(9, 10);
    let ctx = seeded(14);
    let raised = find(
        assess_all(&gls_moderate(today), today, &ctx),
        Disease::GrayLeafSpot,
    );
    assert_eq!(raised.tier, RiskTier::High);
    assert_eq!(
        raised.tier_note.as_deref(),
        Some("Raised from Moderate — recently overseeded turf is highly susceptible")
    );
    // Daily bars stay on the weather-only scale.
    for d in &raised.daily {
        assert_eq!(d.tier, raised.scale.tier_for(d.value));
        assert_eq!(d.tier, RiskTier::Moderate);
    }
    // Every consumer reads the raised tier: management acts on High, the feed surfaces it.
    assert_eq!(
        raised.management.action,
        ManagementAction::ApplyPreventative
    );
    let recs = recommendations::to_recommendations(std::slice::from_ref(&raised), &ctx);
    assert_eq!(recs.len(), 1);
    assert!(recs[0].title.ends_with("High"), "{}", recs[0].title);

    // Severe weather stays Severe: no raise, no note.
    let severe = run_of(today, 4, |d| wet(day(d, 27.0, 22.0, 32.0, 90.0), 18.0));
    let top = find(assess_all(&severe, today, &ctx), Disease::GrayLeafSpot);
    assert_eq!(top.tier, RiskTier::Severe);
    assert!(top.tier_note.is_none());
}
