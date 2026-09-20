//! Turns a disease's risk tier plus lawn history into a management plan: what to do
//! now, the cultural practices, preventative and curative fungicide programs with a
//! rotation-aware pick, and whether a logged application still provides protection.

mod programs;

use crate::models::{
    DailyRisk, Disease, DiseaseContext, DiseaseManagement, Efficacy, FracClass, FungicideOption,
    FungicideProgram, FungicideRecord, ManagementAction, ProtectionStatus, RiskTier,
};
use chrono::{Datelike, Duration, NaiveDate};
use programs::{program_for, OptionSpec, ProgramSpec};

/// Residual protection (days) from a single-site systemic application.
const SYSTEMIC_PROTECTION_DAYS: i64 = 21;
/// Residual protection (days) from a contact/multi-site or phosphonate application.
const CONTACT_PROTECTION_DAYS: i64 = 14;
/// Under Severe pressure fungicides break down faster; shorten the window by this much.
const SEVERE_PRESSURE_PENALTY_DAYS: i64 = 7;
/// How far ahead a forecast High/Severe day triggers a heads-up.
const OUTLOOK_DAYS: i64 = 3;

const LABEL_NOTE: &str = "Efficacy ratings summarize university extension guidance. Rates are \
     product-specific, so none are given — always read and follow the product label.";

pub(super) fn plan(
    disease: Disease,
    tier: RiskTier,
    daily: &[DailyRisk],
    today: NaiveDate,
    ctx: &DiseaseContext,
) -> DiseaseManagement {
    let spec = program_for(disease);
    let last_used = last_used_class(spec, &ctx.fungicide_apps, today);
    let recommended = recommended_class(spec, last_used, false);
    let curative_pick = recommended_class(spec, last_used, true);
    let protection = active_protection(spec, &ctx.fungicide_apps, tier, today);
    let action = action_for(disease, tier, protection.is_some());

    let mut notes: Vec<String> = spec.notes.iter().map(|n| n.to_string()).collect();
    if let Some(warning) = &ctx.rotation_warning {
        notes.push(warning.clone());
    }
    notes.push(LABEL_NOTE.into());

    DiseaseManagement {
        action,
        headline: headline(
            disease,
            tier,
            action,
            daily,
            today,
            ctx,
            recommended,
            &protection,
        ),
        cultural: spec.cultural.iter().map(|c| c.to_string()).collect(),
        preventative: FungicideProgram {
            when: "Before symptoms, when risk reaches High or a High/Severe stretch is forecast."
                .into(),
            interval: spec.preventative_interval.into(),
            guidance: vec![
                "Use the label's preventative rate and the longer end of the interval while \
                 pressure is only High."
                    .into(),
                "Alternate FRAC classes between applications; never make more than two \
                 consecutive applications of the same single-site class."
                    .into(),
            ],
            options: options(spec, last_used, recommended, false),
        },
        curative: FungicideProgram {
            when: "Once you can see active symptoms, whatever the current risk tier.".into(),
            interval: spec.curative_interval.into(),
            guidance: vec![
                "Use the label's curative (higher) rate and the shorter interval until new \
                 growth is clean, then drop back to the preventative program."
                    .into(),
                "Fungicides stop the spread; they don't repair blighted leaves. Recovery \
                 comes from new growth, so keep up the cultural practices."
                    .into(),
            ],
            options: options(spec, last_used, curative_pick, true),
        },
        protection,
        notes,
    }
}

fn action_for(disease: Disease, tier: RiskTier, protected: bool) -> ManagementAction {
    match tier {
        RiskTier::Low => ManagementAction::NoAction,
        RiskTier::Moderate => ManagementAction::Monitor,
        // Red thread's remedy is nitrogen, not a spray.
        _ if disease == Disease::RedThread => ManagementAction::Cultural,
        _ if protected => ManagementAction::Protected,
        _ => ManagementAction::ApplyPreventative,
    }
}

/// Most recent single-site class from this disease's options applied this calendar year.
fn last_used_class(
    spec: &ProgramSpec,
    apps: &[FungicideRecord],
    today: NaiveDate,
) -> Option<FracClass> {
    apps.iter()
        .filter(|app| app.date.year() == today.year() && app.date <= today)
        .filter_map(|app| app.class)
        .find(|class| !class.is_multisite() && spec.options.iter().any(|o| o.class == *class))
}

/// Best unrestricted option that isn't the class used last; ties go to list order.
/// With `curative_only`, the pick must itself have curative activity.
fn recommended_class(
    spec: &ProgramSpec,
    last_used: Option<FracClass>,
    curative_only: bool,
) -> Option<FracClass> {
    let eligible = |o: &&OptionSpec| {
        !o.restricted && Some(o.class) != last_used && (!curative_only || o.curative)
    };
    let best = spec
        .options
        .iter()
        .filter(eligible)
        .map(|o| o.efficacy)
        .max()?;
    spec.options
        .iter()
        .filter(eligible)
        .find(|o| o.efficacy == best)
        .map(|o| o.class)
}

fn options(
    spec: &ProgramSpec,
    last_used: Option<FracClass>,
    pick: Option<FracClass>,
    curative_only: bool,
) -> Vec<FungicideOption> {
    spec.options
        .iter()
        .filter(|o| !curative_only || o.curative)
        .map(|o| FungicideOption {
            frac_class: o.class,
            class_label: o.class.as_str().into(),
            examples: o
                .class
                .common_products()
                .iter()
                .map(|p| p.to_string())
                .collect(),
            efficacy: o.efficacy,
            note: o.note.map(Into::into),
            last_used: Some(o.class) == last_used,
            recommended: Some(o.class) == pick,
        })
        .collect()
}

fn protection_days(class: FracClass, tier: RiskTier) -> i64 {
    let base = if class.is_multisite() || class == FracClass::FracP07 {
        CONTACT_PROTECTION_DAYS
    } else {
        SYSTEMIC_PROTECTION_DAYS
    };
    if tier == RiskTier::Severe {
        base - SEVERE_PRESSURE_PENALTY_DAYS
    } else {
        base
    }
}

/// The most recent logged application that is at least Good on this disease and whose
/// residual window still covers today.
fn active_protection(
    spec: &ProgramSpec,
    apps: &[FungicideRecord],
    tier: RiskTier,
    today: NaiveDate,
) -> Option<ProtectionStatus> {
    apps.iter().filter(|app| app.date <= today).find_map(|app| {
        let class = app.class?;
        let option = spec.options.iter().find(|o| o.class == class)?;
        if option.efficacy < Efficacy::Good {
            return None;
        }
        let through = app.date + Duration::days(protection_days(class, tier));
        (through >= today).then(|| ProtectionStatus {
            product: app.product.clone(),
            class_label: class.as_str().into(),
            applied_on: app.date,
            protected_through: through,
            days_remaining: (through - today).num_days(),
        })
    })
}

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
fn headline(
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

#[cfg(test)]
mod tests;
