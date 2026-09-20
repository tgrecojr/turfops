//! The one-line "what to do" headline for a management plan.

use crate::models::{
    DailyRisk, Disease, DiseaseContext, FracClass, ManagementAction, ProtectionStatus, RiskTier,
};
use chrono::{Duration, NaiveDate};

/// How far ahead a forecast High/Severe day triggers a heads-up.
const OUTLOOK_DAYS: i64 = 3;

/// First forecast day within the outlook window at High or above.
fn outlook_escalation(daily: &[DailyRisk], today: NaiveDate) -> Option<&DailyRisk> {
    daily.iter().find(|d| {
        d.date > today && d.date <= today + Duration::days(OUTLOOK_DAYS) && d.tier >= RiskTier::High
    })
}

fn pick_text(recommended: Option<FracClass>) -> String {
    match recommended {
        Some(class) => {
            let example = class
                .common_products()
                .first()
                .copied()
                .unwrap_or("see label");
            format!("{class}, e.g. {example}")
        }
        None => "a labeled fungicide".into(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn headline(
    disease: Disease,
    tier: RiskTier,
    action: ManagementAction,
    daily: &[DailyRisk],
    today: NaiveDate,
    ctx: &DiseaseContext,
    recommended: Option<FracClass>,
    protection: &Option<ProtectionStatus>,
) -> String {
    let name = disease.name().to_lowercase();
    match action {
        ManagementAction::NoAction => {
            format!("No action needed — conditions don't favor {name}.")
        }
        ManagementAction::Monitor => match (outlook_escalation(daily, today), protection) {
            (Some(day), Some(p)) => format!(
                "Risk is forecast to reach {} on {}, but {} ({}) applied {} covers {name} \
                 through about {}. No action needed until then.",
                day.tier.as_str(),
                day.date.format("%-m/%-d"),
                p.product,
                p.class_label,
                p.applied_on.format("%-m/%-d"),
                p.protected_through.format("%-m/%-d")
            ),
            (Some(day), None) => format!(
                "No fungicide needed yet, but risk is forecast to reach {} on {}. Tighten up \
                 cultural practices and have a preventative on hand: {}.",
                day.tier.as_str(),
                day.date.format("%-m/%-d"),
                pick_text(recommended)
            ),
            (None, _) => "No fungicide needed. Tighten up cultural practices and keep an eye on \
                          the outlook."
                .into(),
        },
        ManagementAction::Cultural => {
            let fed_recently = ctx.days_since_fertilizer.is_some_and(|d| d <= 60);
            if fed_recently {
                "Recently fed turf usually outgrows red thread — stay the course. Consider a \
                 fungicide only if it persists for several weeks."
                    .into()
            } else {
                "Feed the lawn: about 0.5 lb N/1000 sq ft is the fix for red thread. A \
                 fungicide is rarely needed."
                    .into()
            }
        }
        ManagementAction::Protected => {
            let p = protection.as_ref().expect("Protected implies a status");
            let tail = if tier == RiskTier::Severe {
                " Pressure is severe, so scout anyway and don't stretch the interval."
            } else {
                ""
            };
            format!(
                "Protected — {} ({}) applied {} covers {name} through about {}. Reapply then \
                 if risk is still High, rotating to {}.{tail}",
                p.product,
                p.class_label,
                p.applied_on.format("%-m/%-d"),
                p.protected_through.format("%-m/%-d"),
                pick_text(recommended)
            )
        }
        ManagementAction::ApplyPreventative => {
            let scope = if disease == Disease::PythiumBlight && tier == RiskTier::High {
                "Protect vulnerable areas — new seedlings and low, wet spots — with"
            } else if tier == RiskTier::Severe {
                "Apply now:"
            } else {
                "Apply a preventative:"
            };
            let symptoms = if tier == RiskTier::Severe {
                " If you already see symptoms, follow the curative program instead."
            } else {
                ""
            };
            format!("{scope} {}.{symptoms}", pick_text(recommended))
        }
    }
}
