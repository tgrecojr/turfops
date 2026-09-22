//! Turns a disease's risk tier plus lawn history into a management plan: what to do
//! now, the cultural practices, preventative and curative fungicide programs with a
//! rotation-aware pick, and whether a logged application still provides protection.

mod headline;
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
    let owned: Vec<FracClass> = ctx
        .shelf
        .iter()
        .flat_map(|f| f.classes.iter().copied())
        .collect();
    let recommended = recommended_class(spec, last_used, false, &owned);
    let curative_pick = recommended_class(spec, last_used, true, &owned);
    let protection = active_protection(spec, &ctx.fungicide_apps, tier, today);
    let action = action_for(disease, tier, protection.is_some());
    let outlook = headline::outlook_escalation(daily, today).map(|day| headline::Outlook {
        tier: day.tier,
        date: day.date,
        protection: active_protection(spec, &ctx.fungicide_apps, day.tier, day.date),
    });

    let mut notes: Vec<String> = spec.notes.iter().map(|n| n.to_string()).collect();
    if let Some(warning) = &ctx.rotation_warning {
        notes.push(warning.clone());
    }
    notes.push(LABEL_NOTE.into());

    let mut headline = headline::headline(
        disease,
        tier,
        action,
        ctx,
        recommended,
        &protection,
        &outlook,
    );
    if action == ManagementAction::ApplyPreventative {
        headline.push_str(&shelf_note(ctx, recommended));
    }

    DiseaseManagement {
        action,
        headline,
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
            options: options(spec, last_used, recommended, false, ctx),
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
            options: options(spec, last_used, curative_pick, true, ctx),
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

/// Best unrestricted option that isn't the class used last. Among equals, one the user
/// owns wins; then list order. The shelf is only ever a tiebreak — it never promotes a
/// weaker class and never overrides rotation. With `curative_only`, the pick must itself
/// have curative activity.
fn recommended_class(
    spec: &ProgramSpec,
    last_used: Option<FracClass>,
    curative_only: bool,
    owned: &[FracClass],
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
    let mut top = spec
        .options
        .iter()
        .filter(eligible)
        .filter(|o| o.efficacy == best);
    top.clone()
        .find(|o| owned.contains(&o.class))
        .or_else(|| top.next())
        .map(|o| o.class)
}

/// "…you have Heritage G (FRAC 11) on hand." / "…nothing on the shelf covers FRAC 3 or 11."
fn shelf_note(ctx: &DiseaseContext, pick: Option<FracClass>) -> String {
    let Some(class) = pick else {
        return String::new();
    };
    let owned: Vec<&str> = ctx
        .shelf
        .iter()
        .filter(|f| f.classes.contains(&class))
        .map(|f| f.product.name.as_str())
        .collect();
    if owned.is_empty() {
        format!(" Nothing on your shelf is in {}.", class.as_str())
    } else {
        format!(" You have {} on hand.", owned.join(" / "))
    }
}

fn options(
    spec: &ProgramSpec,
    last_used: Option<FracClass>,
    pick: Option<FracClass>,
    curative_only: bool,
    ctx: &DiseaseContext,
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
            restricted: o.restricted,
            on_hand: ctx
                .shelf
                .iter()
                .filter(|f| f.classes.contains(&o.class))
                .map(|f| f.product.clone())
                .collect(),
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
/// residual window still covers today. Some chemistry is good enough at High pressure
/// but not trusted once pressure is Severe (`protects_at_severe`).
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
        if tier == RiskTier::Severe && !option.protects_at_severe {
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

#[cfg(test)]
mod tests;
