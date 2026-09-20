//! Bridges the timing windows into the recommendations feed so the dashboard and the
//! Recommendations page agree with the Timing page by construction. Replaces the old
//! 10 cm pre-emergent and fall-overseeding rules.

use super::text::short;
use crate::logic::rules::thresholds::{DEFAULT_LAWN_SIZE_SQFT, OVERSEED_RATE_LBS_PER_KSQFT};
use crate::models::timing::{
    BoundaryView, SoilNow, TimingWindow, WindowId, WindowPriority, WindowState,
};
use crate::models::{DataSource, LawnProfile, Recommendation, RecommendationCategory, Severity};
use chrono::NaiveDate;

/// How long after the spring pre-emergent window closes the "you missed it" advice stays up.
const MISSED_NOTICE_DAYS: i64 = 21;

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

fn seeding_rate_note(profile: &LawnProfile) -> String {
    let sqft = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);
    format!(
        "Overseeding at {OVERSEED_RATE_LBS_PER_KSQFT:.0} lbs/1000 sqft needs about {:.0} lbs of \
         seed for {sqft:.0} sqft.",
        sqft / 1000.0 * OVERSEED_RATE_LBS_PER_KSQFT
    )
}

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
}
