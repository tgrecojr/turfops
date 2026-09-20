//! Red thread — composite cool-and-wet suitability index, amplified by low nitrogen.
//! No validated forecasting model exists; drivers follow extension guidance.

use super::{
    amplify, fmt_temp_f, management, ramp, score_window, suitability_status, trailing_mean,
    trapezoid,
};
use crate::models::{
    DailyWeather, Disease, DiseaseContext, DiseaseRisk, FactorStatus, Methodology, RiskFactor,
    RiskScale, RiskTier,
};
use chrono::NaiveDate;

const WINDOW_DAYS: i64 = 5;
/// Daily mean temp (°C) suitability: none ≤4, optimal 15–22, none ≥27 (≈40–80°F).
const TEMP_CURVE_C: (f64, f64, f64, f64) = (4.0, 15.0, 22.0, 27.0);
/// Leaf wetness (h) suitability: none ≤4, full ≥12.
const WETNESS_RAMP_H: (f64, f64) = (4.0, 12.0);
/// A rain day (mm) keeps the canopy wet enough to count as at least half suitable.
const RAIN_DAY_MM: f64 = 2.5;
const RAIN_DAY_SUITABILITY: f64 = 0.5;
/// No fertilizer within this many days ⇒ treat the lawn as nitrogen-hungry.
const NITROGEN_DEFICIT_DAYS: i64 = 60;

fn temp_suitability(day: &DailyWeather) -> f64 {
    let (lo, opt_lo, opt_hi, hi) = TEMP_CURVE_C;
    trapezoid(day.temp_mean_c, lo, opt_lo, opt_hi, hi)
}

fn wetness_suitability(day: &DailyWeather) -> f64 {
    let from_wetness = ramp(day.leaf_wetness_hours, WETNESS_RAMP_H.0, WETNESS_RAMP_H.1);
    if day.precip_mm >= RAIN_DAY_MM {
        from_wetness.max(RAIN_DAY_SUITABILITY)
    } else {
        from_wetness
    }
}

/// Daily suitability 0–100.
pub(super) fn daily_index(day: &DailyWeather) -> f64 {
    100.0 * temp_suitability(day) * wetness_suitability(day)
}

fn scale() -> RiskScale {
    RiskScale {
        min: 0.0,
        max: 100.0,
        unit: "/ 100".into(),
        moderate_at: 25.0,
        high_at: 50.0,
        severe_at: 75.0,
    }
}

/// `None` = no fertilizer on record, which also counts as nitrogen-hungry.
fn nitrogen_hungry(ctx: &DiseaseContext) -> bool {
    ctx.days_since_fertilizer
        .is_none_or(|days| days > NITROGEN_DEFICIT_DAYS)
}

pub(super) fn assess(
    series: &[DailyWeather],
    today: NaiveDate,
    ctx: &DiseaseContext,
) -> Option<DiseaseRisk> {
    let scale = scale();
    let value_at = |i: usize| trailing_mean(series, i, WINDOW_DAYS, 1, daily_index);
    let scored = score_window(series, today, &scale, value_at)?;
    let day = &series[scored.headline_idx];
    let index = scored.headline_value;

    let hungry = nitrogen_hungry(ctx);
    let weather_tier = scale.tier_for(index);
    let mut tier = weather_tier;
    // Only the headline is amplified: the daily series stays weather-only so its bars
    // agree with the scale's tier thresholds.
    if hungry {
        tier = amplify(tier);
    }

    let disease = Disease::RedThread;
    let management = management::plan(disease, tier, &scored.daily, today, ctx);
    Some(DiseaseRisk {
        disease,
        slug: disease.slug().into(),
        name: disease.name().into(),
        pathogen: disease.pathogen().into(),
        tier,
        tier_note: (tier != weather_tier).then(|| {
            format!(
                "Raised from {} — no nitrogen logged in 60+ days",
                weather_tier.as_str()
            )
        }),
        score: index,
        score_label: format!("{index:.0} / 100"),
        as_of: day.date,
        scale,
        daily: scored.daily,
        factors: factors(day, ctx, hungry),
        summary: summary(tier, index, hungry),
        methodology: methodology(),
        management,
    })
}

fn factors(day: &DailyWeather, ctx: &DiseaseContext, hungry: bool) -> Vec<RiskFactor> {
    let temp_s = temp_suitability(day);
    let wet_s = wetness_suitability(day);
    let nitrogen_value = match ctx.days_since_fertilizer {
        Some(days) => format!("Fertilized {days} d ago"),
        None => "No fertilizer on record".into(),
    };
    vec![
        RiskFactor {
            label: "Daily mean air temp".into(),
            value: fmt_temp_f(day.temp_mean_c),
            status: suitability_status(temp_s),
            note: format!(
                "{:.0}% suitable — optimum 59–72°F, active 40–80°F",
                temp_s * 100.0
            ),
        },
        RiskFactor {
            label: "Estimated leaf wetness".into(),
            value: format!("{:.0} h", day.leaf_wetness_hours),
            status: suitability_status(wet_s),
            note: format!(
                "{:.0}% suitable — prolonged dew, drizzle or fog drives infection",
                wet_s * 100.0
            ),
        },
        RiskFactor {
            label: "Rainfall".into(),
            value: format!("{:.2} in", day.precip_mm / 25.4),
            status: if day.precip_mm >= RAIN_DAY_MM {
                FactorStatus::Favorable
            } else {
                FactorStatus::Unfavorable
            },
            note: "A rain day (≥0.1 in) counts as at least 50% wetness suitability".into(),
        },
        RiskFactor {
            label: "Nitrogen status".into(),
            value: nitrogen_value,
            status: if hungry {
                FactorStatus::Favorable
            } else {
                FactorStatus::Unfavorable
            },
            note: if hungry {
                "No nitrogen in 60+ days — slow-growing turf can't outgrow infection; risk \
                 raised one tier"
            } else {
                "Recently fed turf outgrows red thread"
            }
            .into(),
        },
    ]
}

fn summary(tier: RiskTier, index: f64, hungry: bool) -> String {
    let verdict = match tier {
        RiskTier::Severe | RiskTier::High => "Cool, wet weather favors red thread",
        RiskTier::Moderate => "Conditions are marginally favorable",
        RiskTier::Low => "Conditions do not currently favor red thread",
    };
    let nitrogen_note = if hungry && tier >= RiskTier::Moderate {
        " The lawn looks under-fertilized, which is the main reason red thread takes hold."
    } else {
        ""
    };
    format!("The red thread suitability index is {index:.0} of 100. {verdict}.{nitrogen_note}")
}

fn methodology() -> Methodology {
    Methodology {
        model_name: "Red thread suitability index".into(),
        citation: "Drivers per Penn State, Cornell and NC State Extension red thread guidance"
            .into(),
        validated: false,
        summary: "Experimental composite index. Red thread has no validated forecasting model, \
                  so this combines its well-established drivers — cool temperatures, prolonged \
                  leaf wetness, and low nitrogen — into a 0–100 suitability score."
            .into(),
        steps: vec![
            "Temperature suitability (0–1) from the daily mean air temp: 0 at ≤ 40°F (4°C), \
             rising to 1 across 59–72°F (15–22°C), back to 0 at ≥ 80°F (27°C)."
                .into(),
            "Leaf wetness is estimated as hours with RH ≥ 90% or measurable precipitation. \
             Wetness suitability (0–1): 0 at ≤ 4 h, rising linearly to 1 at ≥ 12 h; a day \
             with ≥ 0.1 in of rain counts as at least 0.5."
                .into(),
            "Daily index = 100 × temperature suitability × wetness suitability.".into(),
            "The score is the 5-day trailing mean of the daily index. Tiers: < 25 Low · 25–49 \
             Moderate · 50–74 High · ≥ 75 Severe."
                .into(),
            "If no fertilizer has been logged in the last 60 days, a current risk at or above \
             Moderate is raised one tier — nitrogen-deficient turf is the primary risk factor. \
             Daily bars stay weather-only."
                .into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};

    fn wet(mut d: DailyWeather, hours: f64) -> DailyWeather {
        d.leaf_wetness_hours = hours;
        d
    }

    fn fed(days: i64) -> DiseaseContext {
        DiseaseContext {
            days_since_fertilizer: Some(days),
            ..Default::default()
        }
    }

    #[test]
    fn hot_weather_is_unsuitable() {
        let d = wet(day(date(7, 20), 28.0, 23.0, 33.0, 95.0), 20.0);
        assert_eq!(daily_index(&d), 0.0);
    }

    #[test]
    fn rain_day_floors_wetness_suitability() {
        let mut d = day(date(5, 1), 18.0, 12.0, 22.0, 70.0);
        d.precip_mm = 5.0;
        assert_eq!(daily_index(&d), 50.0);
    }

    #[test]
    fn nitrogen_deficit_raises_tier() {
        let today = date(5, 10);
        // temp 18°C → 1.0, wetness 8 h → 0.5 → index 50 → High.
        let series = run_of(today, 6, |d| wet(day(d, 18.0, 12.0, 22.0, 85.0), 8.0));
        let fed_recently = assess(&series, today, &fed(20)).unwrap();
        assert_eq!(fed_recently.tier, RiskTier::High);
        assert!(fed_recently.tier_note.is_none());
        let hungry = assess(&series, today, &fed(75)).unwrap();
        assert_eq!(hungry.tier, RiskTier::Severe);
        assert!(hungry.tier_note.unwrap().starts_with("Raised from High"));
        let unknown = DiseaseContext::default();
        assert_eq!(
            assess(&series, today, &unknown).unwrap().tier,
            RiskTier::Severe
        );
    }

    #[test]
    fn nitrogen_deficit_never_manufactures_risk() {
        let today = date(7, 20);
        let series = run_of(today, 6, |d| day(d, 28.0, 23.0, 33.0, 60.0));
        assert_eq!(
            assess(&series, today, &fed(90)).unwrap().tier,
            RiskTier::Low
        );
    }
}
