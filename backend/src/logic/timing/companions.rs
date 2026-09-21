//! Advice that hangs off the fall seeding decision. The rules engine sees weather, the
//! profile and the log but not the timing windows, so anything that must agree with the
//! seeding window lives here and is applied to the feed next to the window
//! recommendations: core aeration (same window as seeding) and the fall broadleaf
//! herbicide, which is only right for a lawn that is *not* being seeded.

use crate::models::timing::{TimingWindow, WindowId, WindowState};
use crate::models::{
    Application, ApplicationType, DataSource, LawnProfile, Recommendation, RecommendationCategory,
    Severity, SoilType,
};
use chrono::{Duration, NaiveDate};

/// Id of the rules engine's fall broadleaf herbicide recommendation.
const FALL_HERBICIDE_ID: &str = "broadleaf_fall";

fn fall_seeding(windows: &[TimingWindow]) -> Option<&TimingWindow> {
    windows.iter().find(|w| w.id == WindowId::FallSeeding)
}

/// Core aeration while the fall seeding window is usable. Silent once the lawn is seeded
/// (too late to pull cores) and when a fall pre-emergent is down (cores break the
/// barrier) — the same log that blocks seeding.
pub fn aeration(
    windows: &[TimingWindow],
    profile: &LawnProfile,
    history: &[Application],
    today: NaiveDate,
) -> Option<Recommendation> {
    let seeding = fall_seeding(windows)?;
    if !matches!(
        seeding.state,
        WindowState::Open | WindowState::Ideal | WindowState::Closing
    ) {
        return None;
    }
    let aerated_on = |since: NaiveDate| {
        history.iter().any(|app| {
            app.application_type == ApplicationType::Aeration
                && app.application_date >= since
                && app.application_date <= today
        })
    };
    if aerated_on(NaiveDate::from_ymd_opt(seeding.season_year, 7, 1)?) {
        return None;
    }

    let aerated_recently = aerated_on(today - Duration::days(365));
    let heavy_soil = profile
        .soil_type
        .filter(|s| matches!(s, SoilType::Clay | SoilType::ClayLoam));
    let severity = match (heavy_soil.is_some(), aerated_recently) {
        (true, false) => Severity::Warning,
        (false, false) => Severity::Advisory,
        (_, true) => Severity::Info,
    };
    let soil_note = heavy_soil
        .map(|s| {
            format!(
                " Your {} soil is prone to compaction — annual aeration is especially important.",
                s.as_str()
            )
        })
        .unwrap_or_default();

    Some(
        Recommendation::new(
            "aeration",
            RecommendationCategory::Aeration,
            severity,
            "Core Aeration Window",
            format!(
                "The fall seeding window is open ({}), which is also the time to core \
                 aerate: pull cores first, then seed into them.{soil_note}",
                seeding.headline
            ),
        )
        .with_explanation(
            "Core aeration relieves soil compaction, improves water/nutrient penetration, \
             and promotes root growth (Missouri Extension g6705). Early fall is the best \
             time for cool-season grasses because the grass recovers quickly during its \
             peak growth period, and the holes give seed ideal seed-to-soil contact. The \
             window is the fall seeding window from the Seed & Pre-Em Timing page. Clay \
             and clay loam soils benefit the most.",
        )
        .with_data_point(
            "Fall seeding window",
            &seeding.headline,
            DataSource::Calculated.as_str(),
        )
        .with_data_point(
            "Last Aeration",
            if aerated_recently {
                "Within 12 months"
            } else {
                "Over 12 months ago (or never)"
            },
            DataSource::History.as_str(),
        )
        .with_action(
            "Core aerate when soil is moist (not wet or dry). Make 2-3 passes with a core \
             aerator, pulling 2-3 inch plugs. Leave plugs on the surface to break down, \
             then overseed and water lightly.",
        ),
    )
}

/// Make the fall broadleaf herbicide follow the seeding decision in the log:
/// seeded this fall → dropped (herbicide injures seedlings); fall pre-emergent logged
/// (so no seeding) or the seeding window over → unchanged; undecided while the window is
/// still usable → Advisory with the caveat, since herbicide now rules out seeding for weeks.
pub fn reconcile_fall_herbicide(
    recommendations: &mut Vec<Recommendation>,
    windows: &[TimingWindow],
) {
    let Some(seeding) = fall_seeding(windows) else {
        return;
    };
    match seeding.state {
        WindowState::Done => recommendations.retain(|r| r.id != FALL_HERBICIDE_ID),
        WindowState::NotYet
        | WindowState::OpeningSoon
        | WindowState::Open
        | WindowState::Ideal
        | WindowState::Closing => {
            for rec in recommendations
                .iter_mut()
                .filter(|r| r.id == FALL_HERBICIDE_ID)
            {
                rec.severity = rec.severity.min(Severity::Advisory);
                rec.description = format!(
                    "Only if you are NOT seeding this fall. {} The fall seeding window is \
                     still usable ({}); most broadleaf herbicides need about 3-4 weeks \
                     before seed can go down.",
                    rec.description, seeding.headline
                );
            }
        }
        WindowState::Blocked | WindowState::Closed => {}
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{applied, date, station};
    use super::super::{assess, Inputs};
    use super::*;
    use crate::models::GrassType;

    fn profile(soil: Option<SoilType>) -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type: GrassType::TallFescue,
            usda_zone: "7a".into(),
            soil_type: soil,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn windows_on(today: NaiveDate, history: &[Application]) -> Vec<TimingWindow> {
        let days = station(today.pred_opt().unwrap(), 0.0);
        assess(&Inputs {
            today,
            days: &days,
            forecast: &[],
            grass: GrassType::TallFescue,
            history,
        })
        .windows
    }

    fn herbicide() -> Vec<Recommendation> {
        vec![Recommendation::new(
            FALL_HERBICIDE_ID,
            RecommendationCategory::Herbicide,
            Severity::Warning,
            "Fall Broadleaf Herbicide",
            "Best window of the year.",
        )]
    }

    #[test]
    fn aeration_follows_the_seeding_window() {
        // Sep 5: 10 cm soil is still far above the old rule's 65°F gate, but seeding is open.
        let today = date(2026, 9, 5);
        let rec = aeration(
            &windows_on(today, &[]),
            &profile(Some(SoilType::Clay)),
            &[],
            today,
        )
        .expect("seeding window open → aerate");
        assert_eq!(rec.severity, Severity::Warning);

        // Oct 12: the old calendar window (to Oct 15) was still open; seeding is not.
        let late = date(2026, 10, 12);
        assert!(aeration(&windows_on(late, &[]), &profile(None), &[], late).is_none());
    }

    #[test]
    fn aeration_is_silent_once_the_decision_is_logged() {
        let today = date(2026, 9, 20);
        for logged in [
            applied(ApplicationType::PreEmergent, date(2026, 9, 5)), // barrier down
            applied(ApplicationType::Overseed, date(2026, 9, 10)),   // already seeded
            applied(ApplicationType::Aeration, date(2026, 9, 1)),    // already done
        ] {
            let history = [logged];
            let windows = windows_on(today, &history);
            assert!(aeration(&windows, &profile(None), &history, today).is_none());
        }
    }

    #[test]
    fn herbicide_follows_the_log() {
        let today = date(2026, 9, 25);

        let mut undecided = herbicide();
        reconcile_fall_herbicide(&mut undecided, &windows_on(today, &[]));
        assert_eq!(undecided[0].severity, Severity::Advisory);
        assert!(undecided[0]
            .description
            .starts_with("Only if you are NOT seeding"));

        let seeded = [applied(ApplicationType::Overseed, date(2026, 9, 10))];
        let mut recs = herbicide();
        reconcile_fall_herbicide(&mut recs, &windows_on(today, &seeded));
        assert!(recs.is_empty());

        let barrier = [applied(ApplicationType::PreEmergent, date(2026, 9, 5))];
        let mut recs = herbicide();
        reconcile_fall_herbicide(&mut recs, &windows_on(today, &barrier));
        assert_eq!(recs[0].severity, Severity::Warning);

        let after = date(2026, 10, 20); // seeding closed
        let mut recs = herbicide();
        reconcile_fall_herbicide(&mut recs, &windows_on(after, &[]));
        assert_eq!(recs[0].severity, Severity::Warning);
    }
}
