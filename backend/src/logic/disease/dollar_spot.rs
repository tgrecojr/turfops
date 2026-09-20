//! Dollar spot — Smith-Kerns logistic model (Smith et al. 2018, PLOS ONE 13(3):e0194216).

use super::{fmt_temp_f, score_window, trailing_mean};
use crate::models::{
    DailyWeather, Disease, DiseaseRisk, FactorStatus, Methodology, RiskFactor, RiskScale, RiskTier,
};
use chrono::NaiveDate;

const WINDOW_DAYS: i64 = 5;
/// Trailing window needs at least this many usable days.
const MIN_WINDOW_DAYS: usize = 3;
const INTERCEPT: f64 = -11.4041;
const COEF_MEAN_RH: f64 = 0.0894;
const COEF_MEAN_TEMP_C: f64 = 0.1932;
/// The model is inactive outside this 5-day mean air temp range (°C).
const ACTIVE_TEMP_MIN_C: f64 = 10.0;
const ACTIVE_TEMP_MAX_C: f64 = 35.0;
/// Published action threshold (%): fungicide protection is advised above this.
const ACTION_THRESHOLD_PCT: f64 = 20.0;

/// Probability (%) of a dollar spot epidemic from 5-day means of air temp (°C) and RH (%).
pub(super) fn probability_pct(mean_temp_c: f64, mean_rh: f64) -> f64 {
    if !(ACTIVE_TEMP_MIN_C..=ACTIVE_TEMP_MAX_C).contains(&mean_temp_c) {
        return 0.0;
    }
    let logit = INTERCEPT + COEF_MEAN_RH * mean_rh + COEF_MEAN_TEMP_C * mean_temp_c;
    100.0 * logit.exp() / (1.0 + logit.exp())
}

fn scale() -> RiskScale {
    RiskScale {
        min: 0.0,
        max: 100.0,
        unit: "%".into(),
        moderate_at: 10.0,
        high_at: ACTION_THRESHOLD_PCT,
        severe_at: 40.0,
    }
}

fn window_means(series: &[DailyWeather], idx: usize) -> Option<(f64, f64)> {
    let temp = trailing_mean(series, idx, WINDOW_DAYS, MIN_WINDOW_DAYS, |d| d.temp_mean_c)?;
    let rh = trailing_mean(series, idx, WINDOW_DAYS, MIN_WINDOW_DAYS, |d| d.rh_mean)?;
    Some((temp, rh))
}

pub(super) fn assess(series: &[DailyWeather], today: NaiveDate) -> Option<DiseaseRisk> {
    let scale = scale();
    let value_at = |i: usize| window_means(series, i).map(|(t, rh)| probability_pct(t, rh));
    let scored = score_window(series, today, &scale, value_at)?;
    let (mean_temp_c, mean_rh) = window_means(series, scored.headline_idx)?;
    let pct = scored.headline_value;
    let tier = scale.tier_for(pct);

    let disease = Disease::DollarSpot;
    Some(DiseaseRisk {
        disease,
        slug: disease.slug().into(),
        name: disease.name().into(),
        pathogen: disease.pathogen().into(),
        tier,
        score: pct,
        score_label: format!("{pct:.0}% probability"),
        as_of: series[scored.headline_idx].date,
        scale,
        daily: scored.daily,
        factors: factors(mean_temp_c, mean_rh, pct),
        summary: summary(tier, pct),
        methodology: methodology(),
    })
}

fn factors(mean_temp_c: f64, mean_rh: f64, pct: f64) -> Vec<RiskFactor> {
    let active = (ACTIVE_TEMP_MIN_C..=ACTIVE_TEMP_MAX_C).contains(&mean_temp_c);
    let (temp_status, temp_note) = if !active {
        (
            FactorStatus::Unfavorable,
            "Outside the 50–95°F activity range — model inactive",
        )
    } else if mean_temp_c >= 15.0 {
        (FactorStatus::Favorable, "Within the prime activity range")
    } else {
        (
            FactorStatus::Marginal,
            "Cool end of the activity range (50–59°F)",
        )
    };
    let (rh_status, rh_note) = if mean_rh >= 80.0 {
        (FactorStatus::Favorable, "Sustained high humidity (≥80%)")
    } else if mean_rh >= 65.0 {
        (FactorStatus::Marginal, "Moderate humidity (65–79%)")
    } else {
        (FactorStatus::Unfavorable, "Dry air limits infection")
    };
    let (pct_status, pct_note) = if pct >= ACTION_THRESHOLD_PCT {
        (
            FactorStatus::Favorable,
            "Above the published 20% action threshold",
        )
    } else if pct >= 10.0 {
        (
            FactorStatus::Marginal,
            "Approaching the 20% action threshold",
        )
    } else {
        (
            FactorStatus::Unfavorable,
            "Well below the 20% action threshold",
        )
    };

    vec![
        RiskFactor {
            label: "5-day mean air temp".into(),
            value: fmt_temp_f(mean_temp_c),
            status: temp_status,
            note: temp_note.into(),
        },
        RiskFactor {
            label: "5-day mean relative humidity".into(),
            value: format!("{mean_rh:.0}%"),
            status: rh_status,
            note: rh_note.into(),
        },
        RiskFactor {
            label: "Epidemic probability".into(),
            value: format!("{pct:.0}%"),
            status: pct_status,
            note: pct_note.into(),
        },
    ]
}

fn summary(tier: RiskTier, pct: f64) -> String {
    let verdict = match tier {
        RiskTier::Severe => {
            "double the 20% action threshold or more. Expect active dollar spot on \
             unprotected turf"
        }
        RiskTier::High => "above the 20% action threshold. Conditions favor an outbreak",
        RiskTier::Moderate => "approaching the 20% action threshold. Watch the trend",
        RiskTier::Low => "well below the 20% action threshold",
    };
    format!("The Smith-Kerns dollar spot probability is {pct:.0}%, {verdict}.")
}

fn methodology() -> Methodology {
    Methodology {
        model_name: "Smith-Kerns dollar spot model".into(),
        citation: "Smith, Kerns, et al. (2018), PLOS ONE 13(3): e0194216".into(),
        validated: true,
        summary: "A logistic regression validated across six US research sites (including \
                  Pennsylvania) that predicts the probability of a dollar spot epidemic from \
                  recent air temperature and relative humidity."
            .into(),
        steps: vec![
            "MEANAT = 5-day moving average of daily mean air temperature (°C); MEANRH = 5-day \
             moving average of daily mean relative humidity (%)."
                .into(),
            "logit = −11.4041 + 0.0894·MEANRH + 0.1932·MEANAT".into(),
            "Probability = e^logit / (1 + e^logit) × 100".into(),
            "The model is inactive (0%) when MEANAT is below 10°C (50°F) or above 35°C (95°F)."
                .into(),
            "Tiers: < 10% Low · 10–19% Moderate · 20–39% High (the published 20% action \
             threshold) · ≥ 40% Severe."
                .into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};

    #[test]
    fn probability_matches_published_worked_example() {
        // UW Turfgrass Diagnostic Lab example: MEANAT 25°C, MEANRH 75% → 53%.
        assert!((probability_pct(25.0, 75.0) - 53.0).abs() < 0.5);
    }

    #[test]
    fn model_is_inactive_outside_temperature_range() {
        assert_eq!(probability_pct(9.9, 100.0), 0.0);
        assert_eq!(probability_pct(35.1, 100.0), 0.0);
    }

    #[test]
    fn uses_five_day_moving_average() {
        let today = date(9, 20);
        // Four cool dry days then one hot humid day: the average stays low.
        let mut series = run_of(today, 5, |d| day(d, 12.0, 8.0, 16.0, 50.0));
        series[4] = day(today, 28.0, 22.0, 33.0, 95.0);
        let risk = assess(&series, today).unwrap();
        assert_eq!(risk.tier, RiskTier::Low);
    }

    #[test]
    fn needs_three_days_of_history() {
        let today = date(9, 20);
        let series = run_of(today, 2, |d| day(d, 22.0, 18.0, 27.0, 85.0));
        assert!(assess(&series, today).is_none());
    }

    #[test]
    fn warm_humid_stretch_is_severe() {
        let today = date(9, 20);
        let series = run_of(today, 8, |d| day(d, 24.0, 19.0, 29.0, 85.0));
        let risk = assess(&series, today).unwrap();
        assert_eq!(risk.tier, RiskTier::Severe);
    }
}
