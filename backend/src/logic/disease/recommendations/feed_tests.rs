//! What a disease risk looks like as a feed recommendation.

use super::*;
use crate::logic::disease::assess_all;
use crate::logic::disease::test_support::{date, day, run_of};
use crate::models::{DailyWeather, FracClass, FungicideRecord};

/// Warm, humid stretch: brown patch Severe, the rest at their own tiers.
fn hot_humid(today: chrono::NaiveDate, rh: f64) -> Vec<DailyWeather> {
    run_of(today, 8, |d| {
        let mut w = day(d, 26.0, 21.0, 32.0, rh);
        w.hours_rh90 = 14.0;
        w.leaf_wetness_hours = 14.0;
        w
    })
}

fn with_tier(mut risk: DiseaseRisk, tier: RiskTier) -> DiseaseRisk {
    risk.tier = tier;
    risk
}

// @spec DIS-MGMT-REC-001
#[test]
fn one_recommendation_per_high_or_severe_disease_only() {
    let today = date(7, 20);
    let ctx = DiseaseContext::default();
    let base = assess_all(&hot_humid(today, 90.0), today, &ctx)
        .into_iter()
        .next()
        .unwrap();
    let risks: Vec<DiseaseRisk> = [
        RiskTier::Low,
        RiskTier::Moderate,
        RiskTier::High,
        RiskTier::Severe,
    ]
    .into_iter()
    .map(|tier| with_tier(base.clone(), tier))
    .collect();
    let recs = to_recommendations(&risks, &ctx);
    let titles: Vec<&str> = recs.iter().map(|r| r.title.as_str()).collect();
    assert_eq!(
        titles,
        vec!["Brown Patch Risk — High", "Brown Patch Risk — Severe"]
    );
}

// @spec DIS-MGMT-REC-002
#[test]
fn a_disease_recommendation_carries_the_risk_and_its_plan() {
    let today = date(7, 20);
    let ctx = DiseaseContext::default();
    let risks = assess_all(&hot_humid(today, 90.0), today, &ctx);
    let gls = risks
        .iter()
        .find(|r| r.disease == Disease::GrayLeafSpot)
        .unwrap();
    let rec = to_recommendations(std::slice::from_ref(gls), &ctx)
        .pop()
        .expect("gray leaf spot surfaced");
    assert_eq!(rec.id, "disease_gray_leaf_spot");
    assert_eq!(rec.category, RecommendationCategory::DiseasePressure);
    assert_eq!(
        rec.title,
        format!("Gray Leaf Spot Risk — {}", gls.tier.as_str())
    );
    assert_eq!(rec.description, gls.summary);
    let m = &gls.methodology;
    for part in [&m.model_name, &m.citation, &m.summary] {
        assert!(rec.explanation.contains(part.as_str()));
    }
    assert!(m.steps.iter().all(|s| rec.explanation.contains(s.as_str())));
    let action = rec.suggested_action.as_deref().unwrap();
    assert!(action.starts_with(&gls.management.headline));
    assert!(gls
        .management
        .cultural
        .iter()
        .all(|c| action.contains(c.as_str())));
    let labels: Vec<&str> = rec.data_points.iter().map(|d| d.label.as_str()).collect();
    assert_eq!(labels[0], "Risk score");
    assert_eq!(rec.data_points[0].value, gls.score_label);
    assert!(gls
        .factors
        .iter()
        .all(|f| labels.contains(&f.label.as_str())));
    assert_eq!(rec.data_points.len(), gls.factors.len() + 1);
}

// @spec DIS-MGMT-REC-002
#[test]
fn severity_follows_action_then_tier() {
    let today = date(7, 20);
    let none = DiseaseContext::default();
    let base = assess_all(&hot_humid(today, 90.0), today, &none);
    let bp = base
        .iter()
        .find(|r| r.disease == Disease::BrownPatch)
        .unwrap();
    let severity = |risk: DiseaseRisk| to_recommendations(&[risk], &none)[0].severity;
    assert_eq!(
        severity(with_tier(bp.clone(), RiskTier::Severe)),
        Severity::Critical
    );
    assert_eq!(
        severity(with_tier(bp.clone(), RiskTier::High)),
        Severity::Warning
    );
    let mut protected = bp.clone();
    protected.management.action = ManagementAction::Protected;
    assert_eq!(severity(protected), Severity::Info);
    let mut cultural = with_tier(bp.clone(), RiskTier::Severe);
    cultural.management.action = ManagementAction::Cultural;
    assert_eq!(severity(cultural), Severity::Advisory);
}

// @spec DIS-MGMT-REC-003
#[test]
fn a_fungicide_need_lists_every_eligible_preventative_class() {
    let today = date(7, 20);
    // Heritage used a month ago: last used, not protecting.
    let ctx = DiseaseContext {
        fungicide_apps: vec![FungicideRecord {
            date: date(6, 15),
            product: "Heritage G".into(),
            class: Some(FracClass::Frac11),
        }],
        ..Default::default()
    };
    let risks = assess_all(&hot_humid(today, 90.0), today, &ctx);
    let recs = to_recommendations(&risks, &ctx);
    let bp = recs.iter().find(|r| r.id == "disease_brown_patch").unwrap();
    assert_eq!(bp.needs.len(), 1);
    let need = &bp.needs[0];
    assert_eq!(need.categories, vec![ProductCategory::Fungicide]);
    // Brown patch: 11 (last used) and M5 (restricted) are out; 12 is preventative-only
    // and still counts.
    assert_eq!(
        need.frac_classes,
        vec![
            FracClass::Frac7,
            FracClass::Frac3,
            FracClass::Frac12,
            FracClass::Frac1
        ]
    );
}
