//! Gray leaf spot — composite temperature × leaf-wetness suitability index. No single
//! validated turf forecasting model exists; drivers follow Uddin et al. (2003).

use super::{
    amplify, fmt_temp_f, management, ramp, score_window, suitability_status, trailing_mean,
    trapezoid,
};
use crate::models::{
    DailyWeather, Disease, DiseaseContext, DiseaseRisk, FactorStatus, Methodology, RiskFactor,
    RiskScale, RiskTier,
};
use chrono::{Datelike, NaiveDate};

const WINDOW_DAYS: i64 = 3;
/// Daily mean temp (°C) suitability: none ≤18, optimal 25–30, none ≥35.
const TEMP_CURVE_C: (f64, f64, f64, f64) = (18.0, 25.0, 30.0, 35.0);
/// Leaf wetness (h) suitability: none ≤6, full ≥16.
const WETNESS_RAMP_H: (f64, f64) = (6.0, 16.0);
/// Spores blow in from the south each summer; Zone 7a pressure runs July–October.
const SEASON_MONTHS: std::ops::RangeInclusive<u32> = 7..=10;
/// Seedling turf is highly susceptible for roughly this long after overseeding.
const SEEDLING_DAYS: i64 = 60;

fn temp_suitability(day: &DailyWeather) -> f64 {
    let (lo, opt_lo, opt_hi, hi) = TEMP_CURVE_C;
    trapezoid(day.temp_mean_c, lo, opt_lo, opt_hi, hi)
}

fn wetness_suitability(day: &DailyWeather) -> f64 {
    ramp(day.leaf_wetness_hours, WETNESS_RAMP_H.0, WETNESS_RAMP_H.1)
}

/// Daily infection suitability 0–100; zero outside the inoculum season.
pub(super) fn daily_index(day: &DailyWeather) -> f64 {
    if !SEASON_MONTHS.contains(&day.date.month()) {
        return 0.0;
    }
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

    let seedlings = ctx.days_since_overseed.filter(|d| *d <= SEEDLING_DAYS);
    let weather_tier = scale.tier_for(index);
    let mut tier = weather_tier;
    // Only the headline is amplified: the daily series stays weather-only so its bars
    // agree with the scale's tier thresholds.
    if seedlings.is_some() {
        tier = amplify(tier);
    }

    let disease = Disease::GrayLeafSpot;
    let management = management::plan(disease, tier, &scored.daily, today, ctx);
    Some(DiseaseRisk {
        disease,
        slug: disease.slug().into(),
        name: disease.name().into(),
        pathogen: disease.pathogen().into(),
        tier,
        tier_note: (tier != weather_tier).then(|| {
            format!(
                "Raised from {} — recently overseeded turf is highly susceptible",
                weather_tier.as_str()
            )
        }),
        score: index,
        score_label: format!("{index:.0} / 100"),
        as_of: day.date,
        scale,
        daily: scored.daily,
        factors: factors(day, seedlings, tier != weather_tier),
        summary: summary(tier, index, seedlings.is_some()),
        methodology: methodology(),
        management,
    })
}

/// `raised` says whether the seedling amplifier actually moved the headline tier: it
/// only applies at Moderate or above, so at Low the note must not claim a raise.
fn factors(day: &DailyWeather, seedlings: Option<i64>, raised: bool) -> Vec<RiskFactor> {
    let in_season = SEASON_MONTHS.contains(&day.date.month());
    let temp_s = temp_suitability(day);
    let wet_s = wetness_suitability(day);
    let mut out = vec![
        RiskFactor {
            label: "Season".into(),
            value: if in_season {
                "In season"
            } else {
                "Out of season"
            }
            .into(),
            status: if in_season {
                FactorStatus::Favorable
            } else {
                FactorStatus::Unfavorable
            },
            note: "Inoculum is present July–October in Zone 7a".into(),
        },
        RiskFactor {
            label: "Daily mean air temp".into(),
            value: fmt_temp_f(day.temp_mean_c),
            status: suitability_status(temp_s),
            note: format!(
                "{:.0}% suitable — optimum 77–86°F, none below 64°F",
                temp_s * 100.0
            ),
        },
        RiskFactor {
            label: "Estimated leaf wetness".into(),
            value: format!("{:.0} h", day.leaf_wetness_hours),
            status: suitability_status(wet_s),
            note: format!(
                "{:.0}% suitable — infection needs >6 h, peaks at ≥16 h",
                wet_s * 100.0
            ),
        },
    ];
    if let Some(days) = seedlings {
        out.push(RiskFactor {
            label: "Seedling turf".into(),
            value: format!("Overseeded {days} d ago"),
            status: FactorStatus::Favorable,
            note: if raised {
                "Young seedlings are highly susceptible — risk raised one tier"
            } else {
                "Young seedlings are highly susceptible — the tier is raised once weather \
                 risk reaches Moderate"
            }
            .into(),
        });
    }
    out
}

fn summary(tier: RiskTier, index: f64, seedlings: bool) -> String {
    let verdict = match tier {
        RiskTier::Severe => "Warm, prolonged leaf wetness strongly favors infection",
        RiskTier::High => "Conditions favor infection",
        RiskTier::Moderate => "Conditions are marginally favorable",
        RiskTier::Low => "Conditions do not currently favor infection",
    };
    let seedling_note = if seedlings && tier >= RiskTier::High {
        " Recently overseeded turf is especially vulnerable and can be lost quickly."
    } else {
        ""
    };
    format!("The gray leaf spot suitability index is {index:.0} of 100. {verdict}.{seedling_note}")
}

fn methodology() -> Methodology {
    Methodology {
        model_name: "Gray leaf spot suitability index".into(),
        citation: "Drivers from Uddin, Serlemitsos & Viji (2003), Phytopathology 93:336–343; \
                   seasonality per NC State and Penn State Extension"
            .into(),
        validated: false,
        summary: "Experimental composite index. No single peer-reviewed forecasting model for \
                  gray leaf spot on turf is in general use, so this combines the two \
                  best-established drivers — air temperature and leaf wetness duration — into a \
                  0–100 suitability score. Use it alongside scouting."
            .into(),
        steps: vec![
            "Temperature suitability (0–1) from the daily mean air temp: 0 at ≤ 64°F (18°C), \
             rising to 1 across 77–86°F (25–30°C), back to 0 at ≥ 95°F (35°C)."
                .into(),
            "Leaf wetness is estimated as hours with RH ≥ 90% or measurable precipitation. \
             Wetness suitability (0–1): 0 at ≤ 6 h, rising linearly to 1 at ≥ 16 h."
                .into(),
            "Daily index = 100 × temperature suitability × wetness suitability; zero outside \
             July–October, when inoculum is absent in Zone 7a."
                .into(),
            "The score is the 3-day trailing mean of the daily index. Tiers: < 25 Low · 25–49 \
             Moderate · 50–74 High · ≥ 75 Severe."
                .into(),
            "If the lawn was overseeded within 60 days, a current risk at or above Moderate is \
             raised one tier — seedling turf is far more susceptible. Daily bars stay \
             weather-only."
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

    #[test]
    fn index_is_zero_out_of_season() {
        let d = wet(day(date(5, 20), 27.0, 22.0, 32.0, 90.0), 18.0);
        assert_eq!(daily_index(&d), 0.0);
    }

    #[test]
    fn warm_wet_august_day_is_maximal() {
        let d = wet(day(date(8, 10), 27.0, 22.0, 32.0, 90.0), 18.0);
        assert_eq!(daily_index(&d), 100.0);
    }

    #[test]
    fn overseeding_raises_tier_only_when_weather_is_at_least_moderate() {
        let today = date(9, 10);
        let ctx = DiseaseContext {
            days_since_overseed: Some(14),
            ..Default::default()
        };
        // temp 21.5°C → 0.5, wetness 16 h → 1.0 → index 50 → High → Severe with seedlings.
        let favorable = run_of(today, 4, |d| wet(day(d, 21.5, 18.0, 26.0, 90.0), 16.0));
        let baseline = assess(&favorable, today, &DiseaseContext::default()).unwrap();
        assert_eq!(baseline.tier, RiskTier::High);
        assert_eq!(
            assess(&favorable, today, &ctx).unwrap().tier,
            RiskTier::Severe
        );

        let dry = run_of(today, 4, |d| day(d, 21.5, 18.0, 26.0, 60.0));
        assert_eq!(assess(&dry, today, &ctx).unwrap().tier, RiskTier::Low);
    }

    #[test]
    fn seedling_factor_note_only_claims_a_raise_when_one_happened() {
        let today = date(9, 10);
        let ctx = DiseaseContext {
            days_since_overseed: Some(14),
            ..Default::default()
        };
        let note = |series: &[DailyWeather]| {
            assess(series, today, &ctx)
                .unwrap()
                .factors
                .into_iter()
                .find(|f| f.label == "Seedling turf")
                .unwrap()
                .note
        };
        let favorable = run_of(today, 4, |d| wet(day(d, 21.5, 18.0, 26.0, 90.0), 16.0));
        assert!(note(&favorable).ends_with("risk raised one tier"));
        let dry = run_of(today, 4, |d| day(d, 21.5, 18.0, 26.0, 60.0));
        assert!(note(&dry).ends_with("once weather risk reaches Moderate"));
    }

    #[test]
    fn old_overseeding_is_ignored() {
        let today = date(9, 10);
        let ctx = DiseaseContext {
            days_since_overseed: Some(90),
            ..Default::default()
        };
        let series = run_of(today, 4, |d| wet(day(d, 21.5, 18.0, 26.0, 90.0), 16.0));
        assert_eq!(assess(&series, today, &ctx).unwrap().tier, RiskTier::High);
    }
}
