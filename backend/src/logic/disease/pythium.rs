//! Pythium blight — daily threshold score built on the Nutter et al. (1983) forecasting
//! criteria (max > 30°C followed by ≥14 h of RH > 90% with min > 20°C).

use super::{fmt_temp_f, score_window};
use crate::models::{
    DailyWeather, Disease, DiseaseRisk, FactorStatus, Methodology, RiskFactor, RiskScale, RiskTier,
};
use chrono::NaiveDate;

/// Daily max (°C) for active P. aphanidermatum growth.
const HOT_DAY_C: f64 = 30.0;
/// Daily min (°C) above which nights no longer slow the pathogen.
const WARM_NIGHT_C: f64 = 20.0;
/// Hours of RH ≥ 90% that count as a prolonged wet period.
const WET_HOURS_PROLONGED: f64 = 6.0;
/// Hours of RH ≥ 90% that count as an extended wet period.
const WET_HOURS_EXTENDED: f64 = 10.0;
/// Nutter et al. saturation duration.
const WET_HOURS_SATURATED: f64 = 14.0;
/// Dew point (°C) marking oppressive air. Context only: a 5-year backtest showed it is
/// nearly redundant with a warm night, so scoring it inflated High/Severe days.
const MUGGY_DEW_POINT_C: f64 = 20.0;
/// Without any heat criterion, moisture alone can score at most this.
const NO_HEAT_CAP: f64 = 1.0;

/// Daily score 0–5: two heat points, three wet-duration points. Pythium blight is a
/// hot-weather disease, so moisture without heat is capped.
pub(super) fn daily_score(day: &DailyWeather) -> f64 {
    let heat = [day.temp_max_c >= HOT_DAY_C, day.temp_min_c >= WARM_NIGHT_C];
    let moisture = [
        day.hours_rh90 >= WET_HOURS_PROLONGED,
        day.hours_rh90 >= WET_HOURS_EXTENDED,
        day.hours_rh90 >= WET_HOURS_SATURATED,
    ];
    let heat_pts = heat.iter().filter(|met| **met).count() as f64;
    let moisture_pts = moisture.iter().filter(|met| **met).count() as f64;
    if heat_pts == 0.0 {
        moisture_pts.min(NO_HEAT_CAP)
    } else {
        heat_pts + moisture_pts
    }
}

fn scale() -> RiskScale {
    RiskScale {
        min: 0.0,
        max: 5.0,
        unit: "/ 5".into(),
        moderate_at: 2.0,
        high_at: 3.0,
        severe_at: 4.0,
    }
}

pub(super) fn assess(series: &[DailyWeather], today: NaiveDate) -> Option<DiseaseRisk> {
    let scale = scale();
    let scored = score_window(series, today, &scale, |i| Some(daily_score(&series[i])))?;
    let day = &series[scored.headline_idx];
    let score = scored.headline_value;
    let tier = scale.tier_for(score);

    let disease = Disease::PythiumBlight;
    Some(DiseaseRisk {
        disease,
        slug: disease.slug().into(),
        name: disease.name().into(),
        pathogen: disease.pathogen().into(),
        tier,
        score,
        score_label: format!("{score:.0} / 5"),
        as_of: day.date,
        scale,
        daily: scored.daily,
        factors: factors(day),
        summary: summary(tier, score),
        methodology: methodology(),
    })
}

fn met(condition: bool) -> FactorStatus {
    if condition {
        FactorStatus::Favorable
    } else {
        FactorStatus::Unfavorable
    }
}

fn factors(day: &DailyWeather) -> Vec<RiskFactor> {
    let hot = day.temp_max_c >= HOT_DAY_C;
    let warm_night = day.temp_min_c >= WARM_NIGHT_C;
    let muggy = day.dew_point_max_c >= MUGGY_DEW_POINT_C;
    let (wet_status, wet_note) = if day.hours_rh90 >= WET_HOURS_SATURATED {
        (FactorStatus::Favorable, "Saturated ≥14 h (+3)")
    } else if day.hours_rh90 >= WET_HOURS_EXTENDED {
        (FactorStatus::Favorable, "Extended ≥10 h (+2)")
    } else if day.hours_rh90 >= WET_HOURS_PROLONGED {
        (FactorStatus::Marginal, "Prolonged ≥6 h (+1)")
    } else {
        (FactorStatus::Unfavorable, "Short wet period")
    };

    let mut out = vec![
        RiskFactor {
            label: "Daytime high".into(),
            value: fmt_temp_f(day.temp_max_c),
            status: met(hot),
            note: if hot {
                "Above 86°F (+1)"
            } else {
                "Below the 86°F threshold"
            }
            .into(),
        },
        RiskFactor {
            label: "Overnight low".into(),
            value: fmt_temp_f(day.temp_min_c),
            status: met(warm_night),
            note: if warm_night {
                "Above 68°F (+1)"
            } else {
                "Below the 68°F threshold"
            }
            .into(),
        },
        RiskFactor {
            label: "Hours at RH ≥ 90%".into(),
            value: format!("{:.0} h", day.hours_rh90),
            status: wet_status,
            note: wet_note.into(),
        },
        RiskFactor {
            label: "Peak dew point".into(),
            value: fmt_temp_f(day.dew_point_max_c),
            status: met(muggy),
            note: if muggy {
                "Oppressive (≥68°F) — slows overnight drying. Context only, not scored"
            } else {
                "Comfortable (<68°F). Context only, not scored"
            }
            .into(),
        },
    ];
    if !hot && !warm_night {
        out.push(RiskFactor {
            label: "Heat requirement".into(),
            value: "Not met".into(),
            status: FactorStatus::Unfavorable,
            note: "No heat criterion met — moisture alone is capped at 1 point".into(),
        });
    }
    out
}

fn summary(tier: RiskTier, score: f64) -> String {
    let verdict = match tier {
        RiskTier::Severe => {
            "Hot days, warm nights and saturated air together — Pythium blight can destroy \
             turf within 24–48 hours in these conditions"
        }
        RiskTier::High => "Heat and moisture are combining — conditions favor an outbreak",
        RiskTier::Moderate => {
            "Some heat and moisture criteria are met — watch low, wet areas closely"
        }
        RiskTier::Low => "Conditions are unfavorable — Pythium blight needs heat plus moisture",
    };
    format!("The Pythium risk score is {score:.0} of 5. {verdict}.")
}

fn methodology() -> Methodology {
    Methodology {
        model_name: "Pythium threshold score".into(),
        citation: "Criteria from Nutter, Cole & Schein (1983), Plant Disease 67:1126–1128, as \
                   used by Penn State and NC State Extension"
            .into(),
        validated: false,
        summary: "A daily 0–5 score counting the heat and moisture thresholds that the Nutter \
                  forecasting system and extension guidance associate with Pythium blight \
                  outbreaks. The thresholds are published; combining them into a point score is \
                  a decision-support heuristic."
            .into(),
        steps: vec![
            "+1 — daily maximum air temperature ≥ 86°F (30°C).".into(),
            "+1 — daily minimum air temperature ≥ 68°F (20°C).".into(),
            "+1 — at least 6 hours with RH ≥ 90%; +1 more at 10 hours; +1 more at 14 hours \
             (the Nutter saturation duration)."
                .into(),
            "Peak dew point (Magnus formula) is shown for context but not scored: a backtest \
             against five years of station data showed it is nearly redundant with a warm \
             night and inflated the number of High/Severe days."
                .into(),
            "If neither heat criterion is met the score is capped at 1: a cool humid day is not \
             Pythium weather."
                .into(),
            "Tiers: 0–1 Low · 2 Moderate · 3 High · 4–5 Severe.".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};

    fn wet(mut d: DailyWeather, hours: f64) -> DailyWeather {
        d.hours_rh90 = hours;
        d.leaf_wetness_hours = hours;
        d
    }

    #[test]
    fn hot_saturated_day_scores_five() {
        let d = wet(day(date(7, 20), 27.0, 22.0, 33.0, 92.0), 15.0);
        assert_eq!(daily_score(&d), 5.0);
    }

    #[test]
    fn cool_humid_day_is_capped() {
        // Humid September day: long wet period and high dew point, but no heat.
        let mut d = wet(day(date(9, 20), 21.0, 18.5, 24.0, 95.0), 16.0);
        d.dew_point_max_c = 21.0;
        assert_eq!(daily_score(&d), 1.0);
    }

    #[test]
    fn hot_dry_day_scores_heat_only() {
        let d = day(date(7, 20), 27.0, 21.0, 34.0, 40.0);
        assert_eq!(daily_score(&d), 2.0);
    }

    #[test]
    fn tier_follows_headline_day() {
        let today = date(7, 20);
        let series = run_of(today, 3, |d| wet(day(d, 27.0, 22.0, 33.0, 92.0), 15.0));
        let risk = assess(&series, today).unwrap();
        assert_eq!(risk.tier, RiskTier::Severe);
        assert_eq!(risk.score_label, "5 / 5");
    }
}
