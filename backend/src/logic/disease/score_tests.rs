//! Each model's native score, scale and tiers.

use super::test_support::{date, day, run_of};
use super::*;
use crate::models::Disease;

fn assess_one(disease: Disease, series: &[DailyWeather], today: NaiveDate) -> DiseaseRisk {
    assess_all(series, today, &DiseaseContext::default())
        .into_iter()
        .find(|r| r.disease == disease)
        .expect("disease scored")
}

fn floors(scale: &RiskScale) -> (f64, f64, f64) {
    (scale.moderate_at, scale.high_at, scale.severe_at)
}

fn wet(mut d: DailyWeather, hours: f64) -> DailyWeather {
    d.hours_rh90 = hours;
    d.leaf_wetness_hours = hours;
    d
}

// @spec DIS-MODEL-SCORE-001
#[test]
fn brown_patch_is_that_days_e_index_tiered_at_0_5_6() {
    let today = date(7, 20);
    let mut series = run_of(today, 3, |d| day(d, 24.0, 15.0, 30.0, 70.0));
    // Only the headline day's own Tmin and RH count — no trailing window.
    series[2].temp_min_c = 20.0;
    series[2].rh_mean = 90.0;
    let risk = assess_one(Disease::BrownPatch, &series, today);
    let expected = -21.5 + 0.15 * 90.0 + 1.4 * 20.0 - 0.033 * 20.0 * 20.0;
    assert!((risk.score - expected).abs() < 1e-9);
    assert_eq!(floors(&risk.scale), (0.0, 5.0, 6.0));
    assert_eq!(risk.scale.tier_for(-0.1), RiskTier::Low);
    assert_eq!(risk.scale.tier_for(0.0), RiskTier::Moderate);
    assert_eq!(risk.scale.tier_for(5.0), RiskTier::High);
    assert_eq!(risk.scale.tier_for(6.0), RiskTier::Severe);
}

// @spec DIS-MODEL-SCORE-002
#[test]
fn dollar_spot_is_smith_kerns_on_five_day_means_tiered_at_10_20_40() {
    let today = date(9, 20);
    let mut series = run_of(today, 6, |d| day(d, 20.0, 15.0, 25.0, 70.0));
    // The day six back is outside the 5-day window and must not move the mean.
    series[0].temp_mean_c = 34.0;
    series[0].rh_mean = 100.0;
    let risk = assess_one(Disease::DollarSpot, &series, today);
    let logit: f64 = -11.4041 + 0.0894 * 70.0 + 0.1932 * 20.0;
    let expected = 100.0 * logit.exp() / (1.0 + logit.exp());
    assert!((risk.score - expected).abs() < 1e-9);
    assert_eq!(floors(&risk.scale), (10.0, 20.0, 40.0));

    // 0% once the 5-day mean leaves 10–35°C.
    let cold = run_of(today, 5, |d| day(d, 9.0, 5.0, 13.0, 99.0));
    assert_eq!(assess_one(Disease::DollarSpot, &cold, today).score, 0.0);
    // Fewer than three usable days in the window: no value.
    let short = run_of(today, 2, |d| day(d, 20.0, 15.0, 25.0, 85.0));
    assert!(assess_all(&short, today, &DiseaseContext::default())
        .iter()
        .all(|r| r.disease != Disease::DollarSpot));
}

// @spec DIS-MODEL-SCORE-003
#[test]
fn pythium_is_an_experimental_nutter_threshold_score_tiered_at_2_3_4() {
    let today = date(7, 20);
    let score = |max_c: f64, min_c: f64, hours: f64| {
        let series = run_of(today, 1, |d| wet(day(d, 25.0, min_c, max_c, 85.0), hours));
        assess_one(Disease::PythiumBlight, &series, today).score
    };
    assert_eq!(score(30.0, 15.0, 0.0), 1.0); // hot day
    assert_eq!(score(28.0, 20.0, 0.0), 1.0); // warm night
    assert_eq!(score(30.0, 20.0, 6.0), 3.0);
    assert_eq!(score(30.0, 20.0, 10.0), 4.0);
    assert_eq!(score(30.0, 20.0, 14.0), 5.0);
    assert_eq!(score(25.0, 15.0, 16.0), 1.0); // no heat: moisture capped at 1

    let series = run_of(today, 1, |d| day(d, 25.0, 15.0, 28.0, 70.0));
    let risk = assess_one(Disease::PythiumBlight, &series, today);
    assert!(!risk.methodology.validated);
    assert_eq!(floors(&risk.scale), (2.0, 3.0, 4.0));
}

// @spec DIS-MODEL-SCORE-004
#[test]
fn gray_leaf_spot_is_an_experimental_seasonal_suitability_index() {
    let at = |d: NaiveDate, mean_c: f64, hours: f64| {
        let series = run_of(d, 1, |x| wet(day(x, mean_c, 18.0, 30.0, 90.0), hours));
        assess_one(Disease::GrayLeafSpot, &series, d).score
    };
    let aug = date(8, 10);
    assert_eq!(at(aug, 27.0, 16.0), 100.0);
    assert!((at(aug, 21.5, 16.0) - 50.0).abs() < 1e-9); // temperature ramp 18→25
    assert!((at(aug, 32.5, 16.0) - 50.0).abs() < 1e-9); // and back down 30→35
    assert!((at(aug, 27.0, 11.0) - 50.0).abs() < 1e-9); // wetness ramp 6→16
    assert_eq!(at(aug, 27.0, 6.0), 0.0);
    assert_eq!(at(date(6, 30), 27.0, 16.0), 0.0); // outside July–October
    assert_eq!(at(date(11, 1), 27.0, 16.0), 0.0);

    // 3-day trailing mean: a lone favorable day after two dry ones scores a third.
    let mut series = run_of(aug, 4, |d| day(d, 27.0, 18.0, 30.0, 60.0));
    series[3] = wet(day(aug, 27.0, 18.0, 30.0, 90.0), 16.0);
    let risk = assess_one(Disease::GrayLeafSpot, &series, aug);
    assert!((risk.score - 100.0 / 3.0).abs() < 1e-9);
    assert!(!risk.methodology.validated);
    assert_eq!(floors(&risk.scale), (25.0, 50.0, 75.0));
}

// @spec DIS-MODEL-SCORE-005
#[test]
fn red_thread_is_an_experimental_cool_wet_suitability_index() {
    let may = date(5, 10);
    let at = |mean_c: f64, hours: f64, rain: f64| {
        let series = run_of(may, 1, |d| {
            let mut w = wet(day(d, mean_c, 10.0, 20.0, 80.0), hours);
            w.precip_mm = rain;
            w
        });
        assess_one(Disease::RedThread, &series, may).score
    };
    assert_eq!(at(18.0, 12.0, 0.0), 100.0);
    assert!((at(9.5, 12.0, 0.0) - 50.0).abs() < 1e-9); // temperature ramp 4→15
    assert!((at(24.5, 12.0, 0.0) - 50.0).abs() < 1e-9); // and back down 22→27
    assert!((at(18.0, 8.0, 0.0) - 50.0).abs() < 1e-9); // wetness ramp 4→12
    assert_eq!(at(18.0, 2.0, 0.0), 0.0);
    assert!((at(18.0, 2.0, 2.5) - 50.0).abs() < 1e-9); // rain day floors wetness at 0.5

    // 5-day trailing mean.
    let mut series = run_of(may, 6, |d| day(d, 18.0, 10.0, 20.0, 60.0));
    series[5] = wet(day(may, 18.0, 10.0, 20.0, 90.0), 12.0);
    series[0] = wet(day(series[0].date, 18.0, 10.0, 20.0, 90.0), 12.0);
    let risk = assess_one(Disease::RedThread, &series, may);
    assert!((risk.score - 100.0 / 5.0).abs() < 1e-9);
    assert!(!risk.methodology.validated);
    assert_eq!(floors(&risk.scale), (25.0, 50.0, 75.0));
}

// @spec DIS-MODEL-SCORE-006
#[test]
fn every_disease_keeps_its_own_native_score_and_scale() {
    let today = date(7, 20);
    let series = run_of(today, 8, |d| wet(day(d, 26.0, 21.0, 32.0, 88.0), 12.0));
    let risks = assess_all(&series, today, &DiseaseContext::default());
    let units: Vec<&str> = risks.iter().map(|r| r.scale.unit.as_str()).collect();
    assert_eq!(units, vec!["E-index", "%", "/ 5", "/ 100", "/ 100"]);
    let bp = &risks[0];
    assert!((bp.score - brown_patch::e_index(88.0, 21.0)).abs() < 1e-9);
    let ds = &risks[1];
    assert!((ds.score - dollar_spot::probability_pct(26.0, 88.0)).abs() < 1e-9);
    assert_eq!(risks[2].score, pythium::daily_score(&series[7]));
    // Each tier is read off the disease's own scale.
    for risk in &risks {
        assert_eq!(risk.scale.tier_for(risk.score), risk.tier, "{}", risk.name);
    }
    // The response carries per-disease scores only — no blended field.
    let json = serde_json::to_value(crate::models::DiseaseRiskResponse {
        generated_at: chrono::Utc::now(),
        today,
        station: "s".into(),
        diseases: risks,
        data_notes: vec![],
    })
    .unwrap();
    let mut keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["data_notes", "diseases", "generated_at", "station", "today"]
    );
}

// @spec DIS-MODEL-SCORE-007
#[test]
fn every_disease_explains_its_scale_factors_summary_and_method() {
    let today = date(7, 20);
    let series = run_of(today, 8, |d| wet(day(d, 22.0, 18.0, 28.0, 88.0), 12.0));
    let risks = assess_all(&series, today, &DiseaseContext::default());
    assert_eq!(risks.len(), 5);
    for risk in &risks {
        let s = &risk.scale;
        assert!(s.min < s.moderate_at);
        assert!(s.moderate_at < s.high_at && s.high_at < s.severe_at && s.severe_at <= s.max);
        assert!(!s.unit.is_empty());
        assert!(!risk.factors.is_empty(), "{}", risk.name);
        for f in &risk.factors {
            assert!(!f.label.is_empty() && !f.value.is_empty() && !f.note.is_empty());
        }
        assert!(risk.summary.ends_with('.'), "{}", risk.summary);
        let m = &risk.methodology;
        assert!(!m.model_name.is_empty() && !m.citation.is_empty() && !m.summary.is_empty());
        assert!(!m.steps.is_empty());
    }
    let validated: Vec<bool> = risks.iter().map(|r| r.methodology.validated).collect();
    assert_eq!(validated, vec![true, true, false, false, false]);
}

// @spec DIS-MODEL-SCORE-008
#[test]
fn pythium_shows_peak_dew_point_without_scoring_it() {
    let today = date(7, 20);
    let mut muggy = wet(day(today, 27.0, 22.0, 33.0, 92.0), 8.0);
    muggy.dew_point_max_c = 24.0;
    let mut dry = muggy.clone();
    dry.dew_point_max_c = 10.0;
    assert_eq!(pythium::daily_score(&muggy), pythium::daily_score(&dry));

    let risk = assess_one(Disease::PythiumBlight, &[muggy], today);
    let dew = risk
        .factors
        .iter()
        .find(|f| f.label == "Peak dew point")
        .expect("dew point factor");
    assert_eq!(dew.value, "75°F");
    assert!(dew.note.contains("not scored"), "{}", dew.note);
}
