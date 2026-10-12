//! Seed vs. pre-emergent: what the application log says about each window.

use super::*;
use crate::logic::timing::log::check;

fn logged(kind: ApplicationType, on: NaiveDate) -> [Application; 1] {
    [applied(kind, on)]
}

/// `(window, own application, first and last day of the done span)` for season 2026.
fn done_spans() -> [(WindowId, ApplicationType, NaiveDate, NaiveDate); 5] {
    use ApplicationType::{Overseed, PreEmergent};
    [
        (
            WindowId::FallSeeding,
            Overseed,
            date(2026, 7, 1),
            date(2026, 11, 14),
        ),
        (
            WindowId::DormantSeeding,
            Overseed,
            date(2026, 11, 15),
            date(2027, 2, 28),
        ),
        (
            WindowId::SpringSeeding,
            Overseed,
            date(2026, 3, 1),
            date(2026, 6, 30),
        ),
        (
            WindowId::SpringPreEmergent,
            PreEmergent,
            date(2026, 1, 1),
            date(2026, 6, 30),
        ),
        (
            WindowId::FallPreEmergent,
            PreEmergent,
            date(2026, 7, 1),
            date(2026, 12, 31),
        ),
    ]
}

/// `(window, opposite application, first and last day of the conflict span)`.
fn conflict_spans() -> [(WindowId, ApplicationType, NaiveDate, NaiveDate); 5] {
    use ApplicationType::{Overseed, PreEmergent};
    [
        (
            WindowId::FallSeeding,
            PreEmergent,
            date(2026, 7, 1),
            date(2026, 12, 31),
        ),
        (
            WindowId::DormantSeeding,
            PreEmergent,
            date(2026, 7, 1),
            date(2027, 2, 28),
        ),
        (
            WindowId::SpringSeeding,
            PreEmergent,
            date(2026, 1, 1),
            date(2026, 6, 30),
        ),
        (
            WindowId::SpringPreEmergent,
            Overseed,
            date(2025, 11, 15),
            date(2026, 6, 30),
        ),
        (
            WindowId::FallPreEmergent,
            Overseed,
            date(2026, 7, 1),
            date(2026, 12, 31),
        ),
    ]
}

// @spec TIME-LOG-001
#[test]
fn a_windows_own_application_inside_its_span_marks_it_done() {
    for (id, kind, first, last) in done_spans() {
        for inside in [first, last] {
            assert_eq!(
                check(id, 2026, &logged(kind, inside)).done_on,
                Some(inside),
                "{id:?} on {inside}"
            );
        }
        for outside in [first - Duration::days(1), last + Duration::days(1)] {
            assert_eq!(
                check(id, 2026, &logged(kind, outside)).done_on,
                None,
                "{id:?} on {outside}"
            );
        }
        // The latest one in the span is the date shown.
        let both = [applied(kind, first), applied(kind, last)];
        assert_eq!(check(id, 2026, &both).done_on, Some(last));
    }
}

// @spec TIME-LOG-002
#[test]
fn the_opposite_application_inside_the_conflict_span_blocks_the_window() {
    for (id, kind, first, last) in conflict_spans() {
        for inside in [first, last] {
            let message = check(id, 2026, &logged(kind, inside)).conflict;
            let shown = inside.format("%b %-d").to_string();
            assert!(
                message.as_ref().is_some_and(|m| m.contains(&shown)),
                "{id:?} on {inside}: {message:?}"
            );
        }
        for outside in [first - Duration::days(1), last + Duration::days(1)] {
            assert_eq!(
                check(id, 2026, &logged(kind, outside)).conflict,
                None,
                "{id:?} on {outside}"
            );
        }
    }
}

// @spec TIME-LOG-003
#[test]
fn done_and_blocked_override_the_weather_with_done_first() {
    // Sep 20 on the typical curve: both fall windows are Ideal on weather alone.
    let today = date(2026, 9, 20);
    let days = station(date(2026, 9, 19), 0.0);
    let state = |history: &[Application], id| window(&run(today, &days, history), id).state;
    assert_eq!(state(&[], WindowId::FallSeeding), WindowState::Ideal);
    assert_eq!(state(&[], WindowId::FallPreEmergent), WindowState::Ideal);

    let pre_em = logged(ApplicationType::PreEmergent, date(2026, 9, 5));
    assert_eq!(state(&pre_em, WindowId::FallSeeding), WindowState::Blocked);
    assert_eq!(state(&pre_em, WindowId::FallPreEmergent), WindowState::Done);

    // Both logged: each window's own application wins over the conflict.
    let both = [
        applied(ApplicationType::PreEmergent, date(2026, 9, 5)),
        applied(ApplicationType::Overseed, date(2026, 9, 12)),
    ];
    assert_eq!(state(&both, WindowId::FallSeeding), WindowState::Done);
    assert_eq!(state(&both, WindowId::FallPreEmergent), WindowState::Done);

    // Even a window the weather has closed reads Done once its application is logged.
    let late = date(2026, 10, 20);
    let closed_days = station(date(2026, 10, 19), 0.0);
    let seeded = logged(ApplicationType::Overseed, date(2026, 9, 12));
    assert_eq!(
        window(&run(late, &closed_days, &[]), WindowId::FallSeeding).state,
        WindowState::Closed
    );
    assert_eq!(
        window(&run(late, &closed_days, &seeded), WindowId::FallSeeding).state,
        WindowState::Done
    );
}

// @spec TIME-LOG-004
#[test]
fn without_a_logged_application_no_window_is_ever_blocked() {
    for grass in [
        GrassType::TallFescue,
        GrassType::KentuckyBluegrass,
        GrassType::Mixed,
    ] {
        for (m, d) in [(1, 15), (3, 30), (4, 12), (9, 5), (9, 20), (11, 20)] {
            let today = date(2026, m, d);
            let days = station(today.pred_opt().unwrap(), 0.0);
            let result = assess(&Inputs {
                today,
                days: &days,
                forecast: &[],
                grass,
                history: &[],
            });
            assert!(
                result
                    .windows
                    .iter()
                    .all(|w| w.state != WindowState::Blocked && w.conflict.is_none()),
                "{grass:?} {today}"
            );
        }
    }
    // The log alone decides: the same day with a fall pre-emergent logged blocks seeding.
    let today = date(2026, 9, 20);
    let days = station(date(2026, 9, 19), 0.0);
    let history = logged(ApplicationType::PreEmergent, date(2026, 9, 5));
    assert_eq!(
        window(&run(today, &days, &history), WindowId::FallSeeding).state,
        WindowState::Blocked
    );
}
