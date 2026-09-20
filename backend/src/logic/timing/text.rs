use super::evaluate::WindowViews;
use super::series::SUSTAIN_DAYS;
use crate::models::timing::{BoundaryView, DateSource, WindowId, WindowState};
use chrono::NaiveDate;

pub fn short(date: NaiveDate) -> String {
    date.format("%b %-d").to_string()
}

/// "Sep 18", "around Sep 18" or "later than usual (typically Sep 18)".
fn when(boundary: &BoundaryView) -> String {
    match (boundary.date, boundary.source) {
        (Some(d), DateSource::Observed | DateSource::Tentative) => short(d),
        (Some(d), _) => format!("around {}", short(d)),
        (None, _) => match &boundary.typical {
            Some(stat) => format!("later than usual (typically {})", short(stat.median)),
            None => "when conditions allow".to_string(),
        },
    }
}

pub fn headline(state: WindowState, views: &WindowViews, done_on: Option<NaiveDate>) -> String {
    match state {
        WindowState::Done => match done_on {
            Some(d) => format!("Done — logged {}", short(d)),
            None => "Done".to_string(),
        },
        WindowState::Blocked => "Skip this season".to_string(),
        WindowState::NotYet => format!("Opens {}", when(&views.opens)),
        WindowState::OpeningSoon => format!("Opening soon — {}", when(&views.opens)),
        WindowState::Open => match &views.ideal_from {
            Some(ideal) => format!("Open — ideal from {}", when(ideal)),
            None => format!("Open — closes {}", when(&views.closes)),
        },
        WindowState::Ideal => {
            let until = views.ideal_until.as_ref().unwrap_or(&views.closes);
            format!("Ideal now — through {}", when(until))
        }
        WindowState::Closing => format!("Closing — last chance {}", when(&views.closes)),
        WindowState::Closed => "Closed for the season".to_string(),
    }
}

/// A sentence on where the deciding trigger stands, for the states that hinge on one.
pub fn detail(state: WindowState, views: &WindowViews) -> String {
    let deciding = match state {
        WindowState::NotYet | WindowState::OpeningSoon | WindowState::Open => &views.opens,
        WindowState::Ideal => views.ideal_from.as_ref().unwrap_or(&views.opens),
        WindowState::Closing => views.ideal_until.as_ref().unwrap_or(&views.closes),
        WindowState::Closed => &views.closes,
        WindowState::Done | WindowState::Blocked => return String::new(),
    };
    describe(deciding)
}

fn describe(boundary: &BoundaryView) -> String {
    let label = &boundary.label;
    match (boundary.date, boundary.source) {
        (Some(d), DateSource::Observed) => format!("Trigger met on {}: {label}.", short(d)),
        (Some(d), DateSource::Tentative) => format!(
            "{label}: reached on {} and held {} of {SUSTAIN_DAYS} days so far — \
             not yet confirmed as the seasonal shift.",
            short(d),
            boundary.days_held.unwrap_or(0),
        ),
        (Some(d), DateSource::Forecast) => {
            format!("{label}: the soil forecast puts this around {}.", short(d))
        }
        (Some(d), DateSource::Typical) if boundary.passed => {
            format!("{label}: typically {}.", short(d))
        }
        (Some(d), DateSource::Typical) => {
            format!("Waiting on: {label}. Typically {}.", short(d))
        }
        (None, _) => match &boundary.typical {
            Some(stat) => format!(
                "Waiting on: {label}. That is usually {} (as late as {}), so this season is \
                 running behind.",
                short(stat.median),
                short(stat.latest),
            ),
            None => format!("Waiting on: {label}. Not enough station history for a typical date."),
        },
    }
}

/// Standing advice per window. Never rates — the product label governs.
pub fn guidance(id: WindowId, state: WindowState) -> Vec<String> {
    let lines: &[&str] = match (id, state) {
        (_, WindowState::Done | WindowState::Blocked) => &[],
        (WindowId::SpringPreEmergent, WindowState::Closed) => &[
            "Crabgrass has likely begun germinating. A pre-emergent with early post-emergent \
             activity can still catch seedlings; otherwise plan on post-emergent control.",
        ],
        (WindowId::SpringPreEmergent, _) => &[
            "The barrier has to be down and watered in before crabgrass germinates at about \
             55°F in the top 2 inches of soil.",
            "Skip it wherever you plan to seed this spring.",
            "Follow the product label for rate and any split application.",
        ],
        (WindowId::FallPreEmergent, WindowState::Closed) => &[
            "Most winter annuals (Poa annua, henbit, chickweed) have germinated. \
             Post-emergent control is the better option now.",
        ],
        (WindowId::FallPreEmergent, _) => &[
            "Targets Poa annua and other winter annuals, which germinate as soil cools \
             through 70°F. Earlier in the window is better.",
            "Do not apply if you are seeding this fall — it stops grass seed too.",
        ],
        (WindowId::FallSeeding, WindowState::Closed) => &[
            "New seedlings no longer have time to establish before freezing weather. \
             Wait for dormant seeding or next fall.",
        ],
        (WindowId::FallSeeding, _) => &[
            "The best seeding season: cooling soil, fading weed pressure, and two cool \
             growing seasons before summer.",
            "Keep the seedbed moist until germination; hot afternoons may need extra light waterings.",
            "No pre-emergent this fall, and hold broadleaf herbicides until the new grass \
             has been mowed a few times.",
        ],
        (WindowId::SpringSeeding, _) => &[
            "A fallback, not a first choice: spring seedlings meet summer heat before they \
             are established.",
            "Seeding means skipping spring pre-emergent on that area, so expect more crabgrass.",
        ],
        (WindowId::DormantSeeding, _) => &[
            "Seed sown cold lies dormant and germinates when soil warms in spring. \
             Use it for thin or bare areas after a missed fall window.",
            "Expect some loss over winter: seed a little heavier and get good seed-to-soil contact.",
            "Rules out pre-emergent the following spring.",
        ],
    };
    lines.iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(date: Option<NaiveDate>, source: DateSource, passed: bool) -> BoundaryView {
        BoundaryView {
            label: "5 cm soil cools to 70°F".into(),
            typical: None,
            date,
            source,
            passed,
            days_held: (source == DateSource::Tentative).then_some(3),
        }
    }

    fn views(opens: BoundaryView, closes: BoundaryView) -> WindowViews {
        WindowViews {
            opens,
            ideal_from: None,
            ideal_until: None,
            closes,
        }
    }

    #[test]
    fn typical_dates_are_hedged_and_observed_dates_are_not() {
        let sep18 = NaiveDate::from_ymd_opt(2026, 9, 18);
        let nov1 = NaiveDate::from_ymd_opt(2026, 11, 1);
        let v = views(
            boundary(sep18, DateSource::Observed, true),
            boundary(nov1, DateSource::Typical, false),
        );
        assert_eq!(
            headline(WindowState::Open, &v, None),
            "Open — closes around Nov 1"
        );
        assert!(detail(WindowState::Open, &v).starts_with("Trigger met on Sep 18"));
    }

    #[test]
    fn tentative_crossing_reports_days_held() {
        let sep18 = NaiveDate::from_ymd_opt(2026, 9, 18);
        let v = views(
            boundary(sep18, DateSource::Tentative, true),
            boundary(None, DateSource::Typical, false),
        );
        assert!(detail(WindowState::Open, &v).contains("held 3 of 5 days"));
    }

    #[test]
    fn overdue_trigger_has_no_invented_date() {
        let v = views(
            boundary(None, DateSource::Typical, false),
            boundary(None, DateSource::Typical, false),
        );
        assert_eq!(
            headline(WindowState::NotYet, &v, None),
            "Opens when conditions allow"
        );
    }

    #[test]
    fn guidance_never_gives_rates_and_is_empty_when_done() {
        assert!(guidance(WindowId::FallSeeding, WindowState::Done).is_empty());
        for id in [
            WindowId::FallSeeding,
            WindowId::SpringPreEmergent,
            WindowId::FallPreEmergent,
        ] {
            let text = guidance(id, WindowState::Ideal).join(" ");
            assert!(!text.contains("lb"), "{id:?} guidance mentions a rate");
        }
    }
}
