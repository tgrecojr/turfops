//! Brown patch — Fidanza-Dernoeden E-index (Fidanza, Dernoeden & Grybauskas 1996,
//! Phytopathology 86:385-390).

use super::{fmt_temp_f, management, score_window, trailing_mean};
use crate::models::{
    DailyWeather, Disease, DiseaseContext, DiseaseRisk, FactorStatus, Methodology, RiskFactor,
    RiskScale, RiskTier,
};
use chrono::NaiveDate;

/// Revised warning threshold (Vincelli, Univ. of Kentucky) — fewer missed infections.
const E_WARNING: f64 = 5.0;
/// Original Fidanza et al. warning threshold.
const E_HIGH_WARNING: f64 = 6.0;
/// Night temp (°C) above which R. solani is most active.
const NIGHT_ACTIVE_C: f64 = 18.0;
/// Night temp (°C) where activity begins.
const NIGHT_ONSET_C: f64 = 15.0;

/// `E = −21.5 + 0.15·RH + 1.4·Tmin − 0.033·Tmin²` (Tmin in °C, RH = daily mean %).
pub(super) fn e_index(rh_mean: f64, temp_min_c: f64) -> f64 {
    -21.5 + 0.15 * rh_mean + 1.4 * temp_min_c - 0.033 * temp_min_c * temp_min_c
}

fn scale() -> RiskScale {
    RiskScale {
        min: -5.0,
        max: 10.0,
        unit: "E-index".into(),
        moderate_at: 0.0,
        high_at: E_WARNING,
        severe_at: E_HIGH_WARNING,
    }
}

pub(super) fn assess(
    series: &[DailyWeather],
    today: NaiveDate,
    ctx: &DiseaseContext,
) -> Option<DiseaseRisk> {
    let scale = scale();
    let value_at = |i: usize| Some(e_index(series[i].rh_mean, series[i].temp_min_c));
    let scored = score_window(series, today, &scale, value_at)?;
    let day = &series[scored.headline_idx];
    let e = scored.headline_value;
    let tier = scale.tier_for(e);
    let week_avg = trailing_mean(series, scored.headline_idx, 7, 1, |d| {
        e_index(d.rh_mean, d.temp_min_c)
    });

    let disease = Disease::BrownPatch;
    let management = management::plan(disease, tier, &scored.daily, today, ctx);
    Some(DiseaseRisk {
        disease,
        slug: disease.slug().into(),
        name: disease.name().into(),
        pathogen: disease.pathogen().into(),
        tier,
        tier_note: None,
        score: e,
        score_label: format!("E-index {e:.1}"),
        as_of: day.date,
        scale,
        daily: scored.daily,
        factors: factors(day, e, week_avg),
        summary: summary(tier, e),
        methodology: methodology(),
        management,
    })
}

fn factors(day: &DailyWeather, e: f64, week_avg: Option<f64>) -> Vec<RiskFactor> {
    let (night_status, night_note) = if day.temp_min_c >= NIGHT_ACTIVE_C {
        (
            FactorStatus::Favorable,
            "Warm night — pathogen fully active (≥64°F)",
        )
    } else if day.temp_min_c >= NIGHT_ONSET_C {
        (
            FactorStatus::Marginal,
            "Mild night — some activity (59–64°F)",
        )
    } else {
        (
            FactorStatus::Unfavorable,
            "Cool night suppresses infection (<59°F)",
        )
    };
    let (rh_status, rh_note) = if day.rh_mean >= 90.0 {
        (FactorStatus::Favorable, "Saturated air (≥90%)")
    } else if day.rh_mean >= 75.0 {
        (FactorStatus::Marginal, "Elevated (75–89%)")
    } else {
        (FactorStatus::Unfavorable, "Dry air limits mycelial growth")
    };
    let (e_status, e_note) = if e >= E_WARNING {
        (FactorStatus::Favorable, "At or above warning threshold (5)")
    } else if e >= 0.0 {
        (
            FactorStatus::Marginal,
            "Caution range (0–5) — infection possible",
        )
    } else {
        (FactorStatus::Unfavorable, "Below 0 — infection unlikely")
    };

    let mut out = vec![
        RiskFactor {
            label: "Overnight low".into(),
            value: fmt_temp_f(day.temp_min_c),
            status: night_status,
            note: night_note.into(),
        },
        RiskFactor {
            label: "Mean relative humidity".into(),
            value: format!("{:.0}%", day.rh_mean),
            status: rh_status,
            note: rh_note.into(),
        },
        RiskFactor {
            label: "E-index".into(),
            value: format!("{e:.2}"),
            status: e_status,
            note: e_note.into(),
        },
    ];
    if let Some(avg) = week_avg {
        let trend = if e > avg + 0.5 {
            "Rising — above the recent average"
        } else if e < avg - 0.5 {
            "Easing — below the recent average"
        } else {
            "In line with the recent average"
        };
        out.push(RiskFactor {
            label: "7-day average E-index".into(),
            value: format!("{avg:.2}"),
            status: scale_status(avg),
            note: trend.into(),
        });
    }
    out
}

fn scale_status(e: f64) -> FactorStatus {
    if e >= E_WARNING {
        FactorStatus::Favorable
    } else if e >= 0.0 {
        FactorStatus::Marginal
    } else {
        FactorStatus::Unfavorable
    }
}

fn summary(tier: RiskTier, e: f64) -> String {
    let verdict = match tier {
        RiskTier::Severe => {
            "exceeds the original Fidanza warning threshold (6). Infection is likely on \
             unprotected tall fescue"
        }
        RiskTier::High => "exceeds the revised warning threshold (5). Conditions favor infection",
        RiskTier::Moderate => {
            "is in the caution range. Infection is possible if nights stay warm and humid"
        }
        RiskTier::Low => "is below zero. Nights are too cool or too dry for brown patch",
    };
    format!("The brown patch E-index is {e:.1}, which {verdict}.")
}

fn methodology() -> Methodology {
    Methodology {
        model_name: "Fidanza-Dernoeden E-index".into(),
        citation: "Fidanza, Dernoeden & Grybauskas (1996), Phytopathology 86:385–390".into(),
        validated: true,
        summary: "A field-validated brown patch warning model built on the two conditions \
                  that drive Rhizoctonia solani infection: warm nights and humid air."
            .into(),
        steps: vec![
            "Daily minimum air temperature (Tmin, °C) and daily mean relative humidity (RH, %) \
             are taken from hourly station observations."
                .into(),
            "E = −21.5 + 0.15·RH + 1.4·Tmin − 0.033·Tmin²".into(),
            "Tiers: E < 0 Low · 0–4.9 Moderate (caution) · 5–5.9 High (revised Vincelli \
             warning threshold) · ≥ 6 Severe (original Fidanza warning threshold)."
                .into(),
            "Forecast days apply the same formula to forecast lows and humidity.".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};

    #[test]
    fn e_index_matches_published_example() {
        // Tmin 18.6°C, RH 86.3% → E = 6.07 (hand-computed; reference tool shows 6.08
        // from unrounded inputs).
        assert!((e_index(86.3, 18.6) - 6.07).abs() < 0.02);
    }

    #[test]
    fn warm_humid_nights_are_severe() {
        let today = date(7, 20);
        let series = run_of(today, 8, |d| day(d, 26.0, 21.0, 32.0, 90.0));
        let risk = assess(&series, today, &DiseaseContext::default()).unwrap();
        assert_eq!(risk.tier, RiskTier::Severe);
        assert_eq!(risk.as_of, today);
        assert_eq!(risk.daily.len(), 7);
    }

    #[test]
    fn cool_nights_are_low() {
        let today = date(10, 20);
        let series = run_of(today, 8, |d| day(d, 10.0, 5.0, 15.0, 80.0));
        let risk = assess(&series, today, &DiseaseContext::default()).unwrap();
        assert_eq!(risk.tier, RiskTier::Low);
        assert!(risk.score < 0.0);
    }

    #[test]
    fn forecast_days_are_flagged() {
        let today = date(7, 20);
        let mut series = run_of(date(7, 22), 8, |d| day(d, 26.0, 21.0, 32.0, 90.0));
        for d in series.iter_mut().filter(|d| d.date > today) {
            d.is_forecast = true;
        }
        let risk = assess(&series, today, &DiseaseContext::default()).unwrap();
        assert_eq!(risk.as_of, today);
        assert_eq!(risk.daily.iter().filter(|d| d.is_forecast).count(), 2);
    }
}
