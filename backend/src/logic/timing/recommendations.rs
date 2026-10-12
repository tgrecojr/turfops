//! Bridges the timing windows into the recommendations feed so the dashboard and the
//! Recommendations page agree with the Timing page by construction. Replaces the old
//! 10 cm pre-emergent and fall-overseeding rules.

use super::text::short;
use crate::logic::rules::thresholds::OVERSEED_RATE_LBS_PER_KSQFT;
use crate::models::timing::{
    BoundaryView, SoilNow, TimingWindow, WindowId, WindowPriority, WindowState,
};
use crate::models::{
    DataSource, GrassType, HerbicideTiming, LawnProfile, ProductCategory, ProductNeed,
    ProductTarget, Recommendation, RecommendationCategory, Severity,
};
use chrono::NaiveDate;

/// How long after the spring pre-emergent window closes the "you missed it" advice stays up.
const MISSED_NOTICE_DAYS: i64 = 21;

// @spec TIME-FEED-001
/// One recommendation per primary window that is actionable now.
pub fn to_recommendations(
    windows: &[TimingWindow],
    soil: &SoilNow,
    profile: &LawnProfile,
    today: NaiveDate,
) -> Vec<Recommendation> {
    windows
        .iter()
        .filter(|w| w.priority == WindowPriority::Primary)
        .filter_map(|w| Some((w, severity_for(w, today)?)))
        .map(|(w, severity)| to_recommendation(w, severity, soil, profile))
        .collect()
}

/// `None` when the window needs no attention. Fall pre-emergent never rises above
/// Advisory: it is the alternative to seeding, not something to be pushed.
fn severity_for(window: &TimingWindow, today: NaiveDate) -> Option<Severity> {
    let is_fall_pre_em = window.id == WindowId::FallPreEmergent;
    match window.state {
        WindowState::OpeningSoon => Some(Severity::Info),
        WindowState::Open | WindowState::Ideal if is_fall_pre_em => Some(Severity::Info),
        WindowState::Open | WindowState::Ideal => Some(Severity::Advisory),
        WindowState::Closing if is_fall_pre_em => Some(Severity::Advisory),
        WindowState::Closing => Some(Severity::Warning),
        WindowState::Closed if window.id == WindowId::SpringPreEmergent => window
            .closes
            .date
            .filter(|closed| (today - *closed).num_days() <= MISSED_NOTICE_DAYS)
            .map(|_| Severity::Warning),
        _ => None,
    }
}

fn category_for(id: WindowId) -> RecommendationCategory {
    match id {
        WindowId::SpringPreEmergent | WindowId::FallPreEmergent => {
            RecommendationCategory::PreEmergent
        }
        _ => RecommendationCategory::Overseeding,
    }
}

fn boundary_point(rec: Recommendation, name: &str, boundary: &BoundaryView) -> Recommendation {
    let typical = boundary
        .typical
        .as_ref()
        .map(|s| format!("typically {}", short(s.median)));
    let value = match (boundary.date, typical) {
        (Some(date), Some(typical)) => format!("{} ({typical})", short(date)),
        (Some(date), None) => short(date),
        (None, Some(typical)) => format!("later than usual ({typical})"),
        (None, None) => return rec,
    };
    rec.with_data_point(name, value, DataSource::Calculated.as_str())
}

/// Seed per 1,000 sq ft, and the total for the lawn only when its size is recorded: a
/// total for an assumed size would read as the owner's lawn.
// @spec TIME-FEED-003, TIME-FEED-009
fn seeding_rate_note(profile: &LawnProfile) -> String {
    match profile.lawn_size_sqft {
        Some(sqft) => format!(
            "Overseeding at {OVERSEED_RATE_LBS_PER_KSQFT:.0} lbs/1000 sqft needs about {:.0} lbs of \
             seed for {sqft:.0} sqft.",
            sqft / 1000.0 * OVERSEED_RATE_LBS_PER_KSQFT
        ),
        None => format!(
            "Overseed at {OVERSEED_RATE_LBS_PER_KSQFT:.0} lbs/1000 sqft. Set your lawn size in \
             Settings to see the total seed to buy."
        ),
    }
}

// @spec TIME-FEED-002
fn to_recommendation(
    window: &TimingWindow,
    severity: Severity,
    soil: &SoilNow,
    profile: &LawnProfile,
) -> Recommendation {
    let mut action = window.guidance.join("\n");
    if window.id == WindowId::FallSeeding {
        action = format!("{action}\n{}", seeding_rate_note(profile));
    }
    let explanation = format!(
        "Opens when {}; closes when {}. Soil triggers use the 5-day mean at 5 cm and must \
         hold for 5 days. Typical dates are the median across the station's history. \
         See the Seed & Pre-Em Timing page for the full picture.",
        window.opens.label, window.closes.label
    );

    let mut rec = Recommendation::new(
        format!("timing_{}_{}", window.id.slug(), window.season_year),
        category_for(window.id),
        severity,
        format!("{} — {}", window.name, window.headline),
        &window.detail,
    )
    .with_explanation(explanation)
    .with_action(action);
    // @spec TIME-FEED-008
    // A missed spring pre-emergent points to post-emergent control: nothing to buy for it.
    if window.state != WindowState::Closed {
        rec = rec.with_need(need_for(window.id, profile.grass_type));
    }

    if let Some(avg) = soil.avg_5day_f {
        rec = rec.with_data_point(
            "5 cm soil (5-day mean)",
            format!("{avg:.1}°F"),
            DataSource::SoilData.as_str(),
        );
    }
    rec = boundary_point(rec, "Opens", &window.opens);
    if let Some(ideal) = &window.ideal_until {
        rec = boundary_point(rec, "Ideal until", ideal);
    }
    boundary_point(rec, "Closes", &window.closes)
}

// @spec TIME-FEED-004
/// Pre-emergent windows want a pre-emergent herbicide; seeding windows want seed of the
/// lawn's species (a blend accepts any cool-season species).
fn need_for(id: WindowId, grass: GrassType) -> ProductNeed {
    match id {
        WindowId::SpringPreEmergent | WindowId::FallPreEmergent => {
            ProductNeed::new("pre-emergent herbicide", ProductCategory::Herbicide)
                .with_timing(HerbicideTiming::PreEmergent)
        }
        WindowId::FallSeeding | WindowId::SpringSeeding | WindowId::DormantSeeding => {
            let species = match grass {
                GrassType::KentuckyBluegrass => vec![ProductTarget::Kbg],
                GrassType::TallFescue => vec![ProductTarget::Ttf],
                GrassType::PerennialRyegrass => vec![ProductTarget::Prg],
                GrassType::FineFescue => vec![ProductTarget::FineFescue],
                GrassType::Bermuda => vec![ProductTarget::Bermuda],
                GrassType::Zoysia => vec![ProductTarget::Zoysia],
                GrassType::StAugustine => vec![],
                GrassType::Mixed => vec![
                    ProductTarget::Kbg,
                    ProductTarget::Ttf,
                    ProductTarget::Prg,
                    ProductTarget::FineFescue,
                ],
            };
            ProductNeed::new("grass seed", ProductCategory::Seed).with_targets(species)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timing::DateSource;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn boundary(on: Option<NaiveDate>, passed: bool) -> BoundaryView {
        BoundaryView {
            label: "5 cm soil cools to 70°F".into(),
            typical: None,
            date: on,
            source: DateSource::Observed,
            passed,
            days_held: None,
        }
    }

    fn window(id: WindowId, priority: WindowPriority, state: WindowState) -> TimingWindow {
        TimingWindow {
            id,
            name: "Window".into(),
            priority,
            season_year: 2026,
            state,
            headline: "Headline".into(),
            detail: "Detail".into(),
            opens: boundary(Some(date(9, 7)), true),
            ideal_from: None,
            ideal_until: None,
            closes: boundary(Some(date(4, 21)), state == WindowState::Closed),
            done_on: None,
            conflict: None,
            guidance: vec!["Do the thing.".into()],
        }
    }

    fn soil() -> SoilNow {
        SoilNow {
            avg_5day_f: Some(66.2),
            as_of: Some(date(9, 19)),
            depth_cm: 5,
        }
    }

    fn severities(windows: &[TimingWindow], today: NaiveDate) -> Vec<Severity> {
        to_recommendations(windows, &soil(), &LawnProfile::default(), today)
            .iter()
            .map(|r| r.severity)
            .collect()
    }

    // @spec TIME-FEED-001
    #[test]
    fn only_actionable_primary_windows_are_surfaced() {
        use WindowPriority::{Primary, Secondary};
        let windows = [
            window(WindowId::FallSeeding, Primary, WindowState::Closing),
            window(WindowId::FallPreEmergent, Primary, WindowState::Blocked),
            window(WindowId::SpringPreEmergent, Primary, WindowState::NotYet),
            window(WindowId::SpringSeeding, Secondary, WindowState::Open),
        ];
        let recs = to_recommendations(&windows, &soil(), &LawnProfile::default(), date(9, 28));
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].id, "timing_fall_seeding_2026");
        assert_eq!(recs[0].severity, Severity::Warning);
        assert_eq!(recs[0].category, RecommendationCategory::Overseeding);
        assert!(recs[0].suggested_action.as_ref().unwrap().contains("lbs"));
    }

    fn seed_note(lawn_size_sqft: Option<f64>) -> String {
        let profile = LawnProfile {
            lawn_size_sqft,
            ..LawnProfile::default()
        };
        let open = window(
            WindowId::FallSeeding,
            WindowPriority::Primary,
            WindowState::Open,
        );
        let recs = to_recommendations(&[open], &soil(), &profile, date(9, 10));
        recs[0].suggested_action.clone().unwrap()
    }

    // @spec TIME-FEED-003, TIME-FEED-009
    #[test]
    fn the_seed_total_needs_a_recorded_lawn_size() {
        let sized = seed_note(Some(8000.0));
        assert!(sized.contains("4 lbs/1000 sqft"), "{sized}");
        assert!(
            sized.contains("about 32 lbs of seed for 8000 sqft"),
            "{sized}"
        );

        let no_size = seed_note(None);
        assert!(no_size.contains("4 lbs/1000 sqft"), "{no_size}");
        assert!(!no_size.contains("5000"), "no assumed size: {no_size}");
        assert!(!no_size.contains("lbs of seed for"), "no total: {no_size}");
    }

    // @spec TIME-FEED-001
    #[test]
    fn fall_pre_emergent_is_never_pushed_harder_than_advisory() {
        use WindowPriority::Primary;
        let open = window(WindowId::FallPreEmergent, Primary, WindowState::Ideal);
        let closing = window(WindowId::FallPreEmergent, Primary, WindowState::Closing);
        assert_eq!(
            severities(&[open, closing], date(9, 20)),
            vec![Severity::Info, Severity::Advisory]
        );
    }

    // @spec TIME-FEED-001
    #[test]
    fn missed_spring_pre_emergent_is_flagged_for_three_weeks() {
        let missed = window(
            WindowId::SpringPreEmergent,
            WindowPriority::Primary,
            WindowState::Closed,
        );
        assert_eq!(
            severities(std::slice::from_ref(&missed), date(5, 1)),
            vec![Severity::Warning]
        );
        assert!(severities(&[missed], date(6, 1)).is_empty());
    }

    // @spec TIME-FEED-008
    #[test]
    fn a_missed_spring_pre_emergent_shops_for_nothing() {
        let missed = window(
            WindowId::SpringPreEmergent,
            WindowPriority::Primary,
            WindowState::Closed,
        );
        let recs = to_recommendations(&[missed], &soil(), &LawnProfile::default(), date(5, 1));
        assert_eq!(recs.len(), 1);
        assert!(recs[0].needs.is_empty(), "{:?}", recs[0].needs);
    }
}
