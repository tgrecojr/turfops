//! Season-long FRAC rotation advice from the fungicide log.

use super::{classes_of, FracClass};
use crate::models::{Application, ApplicationType};
use chrono::{Datelike, NaiveDate};

/// Result of analyzing a season's fungicide application history for rotation concerns.
#[derive(Debug, Clone)]
pub struct FungicideRotationAdvice {
    #[allow(dead_code)] // read in tests; clippy doesn't count test reads
    pub total_apps_this_season: usize,
    #[allow(dead_code)] // read in tests; clippy doesn't count test reads
    pub last_class: Option<FracClass>,
    #[allow(dead_code)] // read in tests; clippy doesn't count test reads
    pub consecutive_same_class: usize,
    #[allow(dead_code)] // read in tests; clippy doesn't count test reads
    pub recommended_next: Option<FracClass>,
    pub rotation_warning: Option<String>,
}

/// Analyze fungicide application history for the current season and produce
/// FRAC-class-aware rotation advice.
///
/// Filters to lawn fungicide apps in `today`'s year (future-dated entries ignored),
/// orders them oldest → newest whatever order the caller's query used, resolves product
/// names to FRAC classes, detects consecutive same-class usage (resistance risk at 2+),
/// and recommends the next class to rotate to.
pub fn analyze_fungicide_rotation(
    history: &[Application],
    today: NaiveDate,
) -> FungicideRotationAdvice {
    let mut season_apps: Vec<_> = history
        .iter()
        .filter(|app| {
            app.application_type == ApplicationType::Fungicide
                && app.is_turf()
                && app.application_date.year() == today.year()
                && app.application_date <= today
        })
        .collect();
    season_apps.sort_by_key(|app| app.application_date);

    let total_apps_this_season = season_apps.len();

    if total_apps_this_season == 0 {
        return FungicideRotationAdvice {
            total_apps_this_season: 0,
            last_class: None,
            consecutive_same_class: 0,
            recommended_next: None,
            rotation_warning: None,
        };
    }

    // Resolve each app's single-site FRAC classes (multi-site has no resistance risk)
    let resolved: Vec<Vec<FracClass>> = season_apps
        .iter()
        .map(|app| {
            classes_of(app)
                .into_iter()
                .filter(|c| !c.is_multisite())
                .collect()
        })
        .collect();

    // Last single-site class used
    let last_class = resolved.iter().rev().find_map(|c| c.first().copied());

    // Count consecutive applications containing that class, from the most recent back,
    // skipping unknown/multisite entries so they don't break the chain
    let consecutive_same_class = if let Some(last) = last_class {
        resolved
            .iter()
            .rev()
            .filter(|c| !c.is_empty())
            .take_while(|c| c.contains(&last))
            .count()
    } else {
        0
    };

    // Recommend a different class to rotate to
    let recommended_next = last_class.and_then(recommend_rotation);

    // Build warning if consecutive same-class >= 2
    let rotation_warning = if consecutive_same_class >= 2 {
        let last = last_class.unwrap(); // safe: consecutive >= 2 means last_class is Some
        let mut warning = format!(
            "Your last {} applications were {} — resistance risk increases with consecutive \
             same-class use.",
            consecutive_same_class, last
        );
        if let Some(next) = &recommended_next {
            let products = next.common_products();
            let product_example = products.first().copied().unwrap_or("(see label)");
            warning.push_str(&format!(" Rotate to {} (e.g., {}).", next, product_example));
        }
        Some(warning)
    } else if total_apps_this_season >= 3 && last_class.is_some() {
        // General heads-up at 3+ apps even without consecutive same-class
        let last = last_class.unwrap();
        Some(format!(
            "You have applied fungicide {} times this season (last used: {}). \
             Track FRAC classes to avoid resistance.",
            total_apps_this_season, last
        ))
    } else {
        None
    };

    FungicideRotationAdvice {
        total_apps_this_season,
        last_class,
        consecutive_same_class,
        recommended_next,
        rotation_warning,
    }
}

/// Recommend a single-site FRAC class to rotate to, given the last class used.
/// Prioritizes the most common residential turf classes: 11, 3, 1, 7.
pub(super) fn recommend_rotation(last: FracClass) -> Option<FracClass> {
    // Rotation order for common residential turf fungicides
    let rotation_order = [
        FracClass::Frac11,
        FracClass::Frac3,
        FracClass::Frac1,
        FracClass::Frac7,
    ];

    rotation_order.iter().find(|c| **c != last).copied()
}
