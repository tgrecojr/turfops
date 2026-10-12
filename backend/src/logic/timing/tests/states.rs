//! A window's weather state from its resolved boundaries, and the words it is shown with.

use super::*;
use crate::logic::timing::evaluate::WindowViews;
use crate::logic::timing::text::{detail, guidance, headline};
use crate::models::seasonal_plan::WindowConfidence;
use crate::models::timing::{BoundaryView, DateStat};

const TODAY: (u32, u32) = (9, 20);

fn today() -> NaiveDate {
    date(2026, TODAY.0, TODAY.1)
}

fn b(label: &str, on: Option<(u32, u32)>, source: DateSource, passed: bool) -> BoundaryView {
    BoundaryView {
        label: label.into(),
        typical: None,
        date: on.map(|(m, d)| date(2026, m, d)),
        source,
        passed,
        days_held: (source == DateSource::Tentative).then_some(2),
    }
}

fn passed(label: &str) -> BoundaryView {
    b(label, Some((9, 1)), DateSource::Observed, true)
}

fn ahead(label: &str, on: (u32, u32)) -> BoundaryView {
    b(label, Some(on), DateSource::Typical, false)
}

fn views(
    opens: BoundaryView,
    ideal_from: Option<BoundaryView>,
    ideal_until: Option<BoundaryView>,
    closes: BoundaryView,
) -> WindowViews {
    WindowViews {
        opens,
        ideal_from,
        ideal_until,
        closes,
    }
}

// @spec TIME-ENG-033, TIME-ENG-034
#[test]
fn the_weather_state_follows_the_boundaries_in_order() {
    let state = |v: WindowViews| v.weather_state(today());
    let tentative_close = || b("close", Some((9, 19)), DateSource::Tentative, true);

    // Closed once a firm close has passed, whatever the open says.
    assert_eq!(
        state(views(ahead("o", (10, 1)), None, None, passed("c"))),
        WindowState::Closed
    );
    // Not open yet: NotYet, or OpeningSoon within 7 days or on a forecast.
    assert_eq!(
        state(views(ahead("o", (9, 28)), None, None, ahead("c", (11, 1)))),
        WindowState::NotYet
    );
    assert_eq!(
        state(views(ahead("o", (9, 27)), None, None, ahead("c", (11, 1)))),
        WindowState::OpeningSoon
    );
    let forecast_open = b("o", Some((10, 15)), DateSource::Forecast, false);
    assert_eq!(
        state(views(forecast_open, None, None, ahead("c", (11, 1)))),
        WindowState::OpeningSoon
    );
    // A tentative close never closes the window: it reads Closing.
    assert_eq!(
        state(views(passed("o"), None, None, tentative_close())),
        WindowState::Closing
    );
    assert_eq!(
        state(views(
            passed("o"),
            Some(ahead("if", (10, 1))),
            None,
            tentative_close()
        )),
        WindowState::Closing
    );
    // Open before an ideal-from; Ideal after it; Closing past an ideal-until.
    let close = || ahead("c", (11, 1));
    assert_eq!(
        state(views(
            passed("o"),
            Some(ahead("if", (10, 1))),
            None,
            close()
        )),
        WindowState::Open
    );
    assert_eq!(
        state(views(passed("o"), Some(passed("if")), None, close())),
        WindowState::Ideal
    );
    assert_eq!(
        state(views(
            passed("o"),
            None,
            Some(ahead("iu", (10, 1))),
            close()
        )),
        WindowState::Ideal
    );
    assert_eq!(
        state(views(passed("o"), None, Some(passed("iu")), close())),
        WindowState::Closing
    );
    // No ideal stretch at all: simply Open.
    assert_eq!(
        state(views(passed("o"), None, None, close())),
        WindowState::Open
    );
}

fn stat(median: (u32, u32), p90: (u32, u32)) -> DateStat {
    let d = |(m, day): (u32, u32)| date(2026, m, day);
    DateStat {
        median: d(median),
        p10: d(median) - Duration::days(10),
        p90: d(p90),
        earliest: d(median) - Duration::days(14),
        latest: d(p90) + Duration::days(2),
        sample_count: 12,
        confidence: WindowConfidence::High,
    }
}

// @spec TIME-ENG-035
#[test]
fn headlines_hedge_every_date_that_was_not_observed() {
    let close = |source| b("c", Some((11, 1)), source, false);
    let open = |c| views(passed("o"), None, None, c);
    assert_eq!(
        headline(WindowState::Open, &open(close(DateSource::Typical)), None),
        "Open — closes around Nov 1"
    );
    assert_eq!(
        headline(WindowState::Open, &open(close(DateSource::Forecast)), None),
        "Open — closes around Nov 1"
    );
    let observed = b("c", Some((11, 1)), DateSource::Observed, true);
    assert_eq!(
        headline(WindowState::Open, &open(observed), None),
        "Open — closes Nov 1"
    );

    // Undated: "later than usual (typically …)", or with no history at all, no date.
    let mut overdue = b("o", None, DateSource::Typical, false);
    overdue.typical = Some(stat((9, 7), (9, 25)));
    let v = views(overdue, None, None, ahead("c", (11, 1)));
    assert_eq!(
        headline(WindowState::NotYet, &v, None),
        "Opens later than usual (typically Sep 7)"
    );
    let unknown = views(
        b("o", None, DateSource::Typical, false),
        None,
        None,
        ahead("c", (11, 1)),
    );
    assert_eq!(
        headline(WindowState::NotYet, &unknown, None),
        "Opens when conditions allow"
    );
}

// @spec TIME-ENG-036
#[test]
fn the_detail_describes_the_deciding_boundary_and_saves_behind_for_past_p90() {
    let v = views(
        passed("soil opens"),
        Some(passed("soil ideal")),
        Some(ahead("freeze buffer", (10, 1))),
        ahead("close", (11, 1)),
    );
    assert!(detail(WindowState::NotYet, &v, today()).contains("soil opens"));
    assert!(detail(WindowState::Ideal, &v, today()).contains("soil ideal"));
    assert!(detail(WindowState::Closing, &v, today()).contains("freeze buffer"));
    assert!(detail(WindowState::Closed, &v, today()).contains("close"));
    assert_eq!(detail(WindowState::Done, &v, today()), "");
    assert_eq!(detail(WindowState::Blocked, &v, today()), "");

    let mut overdue = b("soil opens", None, DateSource::Typical, false);
    overdue.typical = Some(stat((9, 7), (9, 25)));
    let late = views(overdue, None, None, ahead("close", (11, 1)));
    let on_p90 = detail(WindowState::NotYet, &late, date(2026, 9, 25));
    assert!(!on_p90.contains("running behind"), "{on_p90}");
    let past_p90 = detail(WindowState::NotYet, &late, date(2026, 9, 26));
    assert!(past_p90.contains("running behind"), "{past_p90}");
}

const ALL_STATES: [WindowState; 8] = [
    WindowState::NotYet,
    WindowState::OpeningSoon,
    WindowState::Open,
    WindowState::Ideal,
    WindowState::Closing,
    WindowState::Closed,
    WindowState::Done,
    WindowState::Blocked,
];

const ALL_WINDOWS: [WindowId; 5] = [
    WindowId::SpringPreEmergent,
    WindowId::FallPreEmergent,
    WindowId::FallSeeding,
    WindowId::SpringSeeding,
    WindowId::DormantSeeding,
];

/// A number followed by an amount unit: "3 lbs", "1.5 oz", "0.5 lb/1000".
fn states_an_amount(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.windows(2).any(|pair| {
        let number = pair[0].trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        let unit = pair[1].to_ascii_lowercase();
        !number.is_empty()
            && number.parse::<f64>().is_ok()
            && [
                "lb", "oz", "gal", "qt", "pt", "fl", "ml", "kg", "g/", "gram",
            ]
            .iter()
            .any(|u| unit.starts_with(u))
    }) || text.contains("/1000")
        || text.to_ascii_lowercase().contains("per 1,000")
}

// @spec TIME-ENG-037
#[test]
fn guidance_is_fixed_per_window_and_state_and_never_states_a_rate() {
    assert!(states_an_amount("Apply 3 lbs per bag"), "detector works");
    for id in ALL_WINDOWS {
        for state in ALL_STATES {
            let lines = guidance(id, state);
            assert_eq!(lines, guidance(id, state), "fixed text");
            if matches!(state, WindowState::Done | WindowState::Blocked) {
                assert!(lines.is_empty(), "{id:?} {state:?}");
                continue;
            }
            assert!(!lines.is_empty(), "{id:?} {state:?}");
            let text = lines.join(" ");
            assert!(!states_an_amount(&text), "{id:?} {state:?}: {text}");
        }
    }
}
