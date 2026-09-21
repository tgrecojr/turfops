use crate::models::timing::WindowId;
use crate::models::{Application, ApplicationType};
use chrono::NaiveDate;

/// What the application log says about a window's season.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Logged {
    /// The window's own application is already down.
    pub done_on: Option<NaiveDate>,
    /// A logged application that rules this window out.
    pub conflict: Option<String>,
}

/// Inclusive date range as `(year_offset, month, day)` pairs relative to the season year.
type Span = ((i32, u32, u32), (i32, u32, u32));

/// Seed and pre-emergent never share a season: the herbicide that stops weed seed
/// stops grass seed too. For each window: the span in which its own application counts
/// as done, and the span in which the opposite application blocks it.
fn spans(id: WindowId) -> (ApplicationType, Span, Span) {
    use ApplicationType::{Overseed, PreEmergent};
    match id {
        WindowId::FallSeeding => (Overseed, ((0, 7, 1), (0, 11, 14)), ((0, 7, 1), (0, 12, 31))),
        // A fall barrier is still active when dormant seed tries to germinate in spring.
        WindowId::DormantSeeding => (Overseed, ((0, 11, 15), (1, 2, 28)), ((0, 7, 1), (1, 2, 28))),
        WindowId::SpringSeeding => (Overseed, ((0, 3, 1), (0, 6, 30)), ((0, 1, 1), (0, 6, 30))),
        // Dormant seed from the previous winter germinates in spring.
        WindowId::SpringPreEmergent => (
            PreEmergent,
            ((0, 1, 1), (0, 6, 30)),
            ((-1, 11, 15), (0, 6, 30)),
        ),
        WindowId::FallPreEmergent => (
            PreEmergent,
            ((0, 7, 1), (0, 12, 31)),
            ((0, 7, 1), (0, 12, 31)),
        ),
    }
}

fn latest_in(
    history: &[Application],
    kind: ApplicationType,
    span: Span,
    season_year: i32,
) -> Option<NaiveDate> {
    let edge = |(offset, month, day): (i32, u32, u32)| {
        NaiveDate::from_ymd_opt(season_year + offset, month, day)
    };
    let (from, to) = (edge(span.0)?, edge(span.1)?);
    history
        .iter()
        .filter(|a| a.application_type == kind)
        .map(|a| a.application_date)
        .filter(|d| *d >= from && *d <= to)
        .max()
}

pub fn check(id: WindowId, season_year: i32, history: &[Application]) -> Logged {
    let (own, done_span, conflict_span) = spans(id);
    let (opposite, message): (_, fn(NaiveDate) -> String) = match own {
        ApplicationType::Overseed => (ApplicationType::PreEmergent, |d| {
            format!(
                "Pre-emergent was applied on {} and will stop grass seed as well as weeds. \
                 Seed only after the product label's reseeding interval.",
                d.format("%b %-d")
            )
        }),
        _ => (ApplicationType::Overseed, |d| {
            format!(
                "Seed went down on {}. Pre-emergent would keep it from establishing, \
                 so skip this application.",
                d.format("%b %-d")
            )
        }),
    };
    Logged {
        done_on: latest_in(history, own, done_span, season_year),
        conflict: latest_in(history, opposite, conflict_span, season_year).map(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(kind: ApplicationType, year: i32, month: u32, day: u32) -> Application {
        Application {
            id: None,
            lawn_profile_id: 1,
            application_type: kind,
            product_name: None,
            application_date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: None,
            frac_classes: None,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn fall_overseed_completes_seeding_and_blocks_fall_pre_emergent() {
        let history = [applied(ApplicationType::Overseed, 2026, 9, 12)];

        let seeding = check(WindowId::FallSeeding, 2026, &history);
        assert_eq!(seeding.done_on, NaiveDate::from_ymd_opt(2026, 9, 12));
        assert_eq!(seeding.conflict, None);

        let pre_em = check(WindowId::FallPreEmergent, 2026, &history);
        assert_eq!(pre_em.done_on, None);
        assert!(pre_em.conflict.unwrap().contains("Sep 12"));

        // Seedlings are established by spring: next year's pre-emergent is unaffected.
        assert_eq!(
            check(WindowId::SpringPreEmergent, 2027, &history),
            Logged::default()
        );
    }

    #[test]
    fn dormant_seeding_blocks_the_following_spring_pre_emergent() {
        let history = [applied(ApplicationType::Overseed, 2026, 12, 5)];
        assert!(check(WindowId::DormantSeeding, 2026, &history)
            .done_on
            .is_some());
        assert!(check(WindowId::SpringPreEmergent, 2027, &history)
            .conflict
            .is_some());
        assert_eq!(
            check(WindowId::FallSeeding, 2026, &history),
            Logged::default()
        );
    }

    #[test]
    fn spring_pre_emergent_blocks_spring_seeding_but_not_fall() {
        let history = [applied(ApplicationType::PreEmergent, 2026, 4, 2)];
        assert!(check(WindowId::SpringPreEmergent, 2026, &history)
            .done_on
            .is_some());
        assert!(check(WindowId::SpringSeeding, 2026, &history)
            .conflict
            .is_some());
        assert_eq!(
            check(WindowId::FallSeeding, 2026, &history),
            Logged::default()
        );
    }

    #[test]
    fn other_seasons_and_types_are_ignored() {
        let history = [
            applied(ApplicationType::PreEmergent, 2025, 4, 2),
            applied(ApplicationType::Fertilizer, 2026, 9, 1),
        ];
        assert_eq!(
            check(WindowId::SpringPreEmergent, 2026, &history),
            Logged::default()
        );
        assert_eq!(
            check(WindowId::FallSeeding, 2026, &history),
            Logged::default()
        );
    }
}
