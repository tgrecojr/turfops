//! The one-line "what to do" headline for a management plan.

use crate::models::{
    DailyRisk, Disease, DiseaseContext, FracClass, ManagementAction, ProtectionStatus, RiskTier,
};
use chrono::{Duration, NaiveDate};

/// How far ahead a forecast High/Severe day triggers a heads-up.
const OUTLOOK_DAYS: i64 = 3;

/// A forecast High/Severe day, and whether a logged fungicide will still cover the lawn
/// *on that day at that tier* — Severe pressure shortens the residual window and rules
/// out some chemistry, so today's (lower-tier) protection status says nothing about it.
pub(super) struct Outlook {
    pub tier: RiskTier,
    pub date: NaiveDate,
    pub protection: Option<ProtectionStatus>,
}

/// First forecast day within the outlook window at High or above.
pub(super) fn outlook_escalation(daily: &[DailyRisk], today: NaiveDate) -> Option<&DailyRisk> {
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
    ctx: &DiseaseContext,
    recommended: Option<FracClass>,
    protection: &Option<ProtectionStatus>,
    outlook: &Option<Outlook>,
) -> String {
    let name = disease.name().to_lowercase();
    // What a forecast High/Severe day means for someone who needs nothing today.
    let heads_up = |lead: &str| {
        outlook.as_ref().map(|o| match &o.protection {
            Some(p) => format!(
                "Risk is forecast to reach {} on {}, but {} ({}) applied {} covers {name} \
                 through about {}. No action needed until then.",
                o.tier.as_str(),
                o.date.format("%-m/%-d"),
                p.product,
                p.class_label,
                p.applied_on.format("%-m/%-d"),
                p.protected_through.format("%-m/%-d")
            ),
            None => format!(
                "{lead}, but risk is forecast to reach {} on {}. Tighten up cultural practices \
                 and have a preventative on hand: {}.",
                o.tier.as_str(),
                o.date.format("%-m/%-d"),
                pick_text(recommended)
            ),
        })
    };
    match action {
        // Red thread's remedy is nitrogen, so a forecast never calls for a fungicide.
        ManagementAction::NoAction if disease != Disease::RedThread => {
            heads_up("No action needed today")
                .unwrap_or_else(|| format!("No action needed — conditions don't favor {name}."))
        }
        ManagementAction::NoAction => {
            format!("No action needed — conditions don't favor {name}.")
        }
        ManagementAction::Monitor => heads_up("No fungicide needed yet").unwrap_or_else(|| {
            "No fungicide needed. Tighten up cultural practices and keep an eye on the outlook."
                .into()
        }),
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
