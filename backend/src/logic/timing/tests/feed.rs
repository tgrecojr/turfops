//! What the windows put into the recommendations feed.

use super::*;
use crate::logic::timing::companions::aeration;
use crate::logic::timing::recommendations::to_recommendations;
use crate::models::timing::{BoundaryView, SoilNow, WindowPriority};
use crate::models::{
    HerbicideTiming, LawnProfile, ProductCategory, ProductTarget, RecommendationCategory, Severity,
    SoilType,
};

fn view(label: &str, on: NaiveDate, passed: bool) -> BoundaryView {
    BoundaryView {
        label: label.into(),
        typical: None,
        date: Some(on),
        source: DateSource::Observed,
        passed,
        days_held: None,
    }
}

fn timing_window(id: WindowId, state: WindowState, closes_on: NaiveDate) -> TimingWindow {
    let priority = match id {
        WindowId::SpringSeeding => WindowPriority::Secondary,
        WindowId::DormantSeeding => WindowPriority::Optional,
        _ => WindowPriority::Primary,
    };
    TimingWindow {
        id,
        name: "The Window".into(),
        priority,
        season_year: 2026,
        state,
        headline: "The headline".into(),
        detail: "The detail.".into(),
        opens: view("opens label", date(2026, 3, 24), true),
        ideal_from: None,
        ideal_until: Some(view("ideal label", date(2026, 4, 7), false)),
        closes: view("closes label", closes_on, state == WindowState::Closed),
        done_on: None,
        conflict: None,
        guidance: vec!["Line one.".into(), "Line two.".into()],
    }
}

fn soil() -> SoilNow {
    SoilNow {
        avg_5day_f: Some(52.34),
        as_of: Some(date(2026, 5, 1)),
        depth_cm: 5,
    }
}

fn severity(id: WindowId, state: WindowState, closes_on: NaiveDate) -> Option<Severity> {
    let w = timing_window(id, state, closes_on);
    let recs = to_recommendations(&[w], &soil(), &LawnProfile::default(), date(2026, 5, 1));
    assert!(recs.len() <= 1);
    recs.first().map(|r| r.severity)
}

// @spec TIME-FEED-001
#[test]
fn primary_windows_reach_the_feed_at_their_states_severity() {
    use Severity::{Advisory, Info, Warning};
    use WindowState::*;
    let close = date(2026, 4, 21);
    let expected = [
        (WindowId::SpringPreEmergent, OpeningSoon, Some(Info)),
        (WindowId::SpringPreEmergent, Open, Some(Advisory)),
        (WindowId::SpringPreEmergent, Ideal, Some(Advisory)),
        (WindowId::SpringPreEmergent, Closing, Some(Warning)),
        (WindowId::FallSeeding, Ideal, Some(Advisory)),
        (WindowId::FallSeeding, Closing, Some(Warning)),
        (WindowId::FallPreEmergent, OpeningSoon, Some(Info)),
        (WindowId::FallPreEmergent, Open, Some(Info)),
        (WindowId::FallPreEmergent, Ideal, Some(Info)),
        (WindowId::FallPreEmergent, Closing, Some(Advisory)),
        (WindowId::FallPreEmergent, Closed, None),
        (WindowId::FallSeeding, Closed, None),
        (WindowId::FallSeeding, NotYet, None),
        (WindowId::FallSeeding, Done, None),
        (WindowId::FallSeeding, Blocked, None),
        // Secondary and optional windows never reach the feed.
        (WindowId::SpringSeeding, Ideal, None),
        (WindowId::DormantSeeding, Closing, None),
    ];
    for (id, state, want) in expected {
        assert_eq!(severity(id, state, close), want, "{id:?} {state:?}");
    }
    // A missed spring pre-emergent is a Warning for 21 days after its close (today May 1).
    let missed = |closed| severity(WindowId::SpringPreEmergent, Closed, closed);
    assert_eq!(missed(date(2026, 4, 10)), Some(Warning)); // 21 days ago
    assert_eq!(missed(date(2026, 4, 9)), None); // 22 days ago
}

// @spec TIME-FEED-002
#[test]
fn a_window_recommendation_carries_the_windows_words_and_dates() {
    let w = timing_window(
        WindowId::SpringPreEmergent,
        WindowState::Ideal,
        date(2026, 4, 21),
    );
    let rec = &to_recommendations(&[w], &soil(), &LawnProfile::default(), date(2026, 4, 10))[0];
    assert_eq!(rec.id, "timing_spring_pre_emergent_2026");
    assert_eq!(rec.category, RecommendationCategory::PreEmergent);
    assert_eq!(rec.title, "The Window — The headline");
    assert_eq!(rec.description, "The detail.");
    assert_eq!(
        rec.suggested_action.as_deref(),
        Some("Line one.\nLine two.")
    );
    let points: Vec<(&str, &str)> = rec
        .data_points
        .iter()
        .map(|p| (p.label.as_str(), p.value.as_str()))
        .collect();
    assert_eq!(
        points,
        vec![
            ("5 cm soil (5-day mean)", "52.3°F"),
            ("Opens", "Mar 24"),
            ("Ideal until", "Apr 7"),
            ("Closes", "Apr 21"),
        ]
    );

    let seeding = timing_window(WindowId::FallSeeding, WindowState::Ideal, date(2026, 10, 6));
    let rec = &to_recommendations(
        &[seeding],
        &soil(),
        &LawnProfile::default(),
        date(2026, 9, 20),
    )[0];
    assert_eq!(rec.id, "timing_fall_seeding_2026");
    assert_eq!(rec.category, RecommendationCategory::Overseeding);
}

fn needs_of(id: WindowId, state: WindowState, grass: GrassType) -> Vec<crate::models::ProductNeed> {
    let profile = LawnProfile {
        grass_type: grass,
        ..LawnProfile::default()
    };
    let w = timing_window(id, state, date(2026, 10, 6));
    to_recommendations(&[w], &soil(), &profile, date(2026, 9, 20))[0]
        .needs
        .clone()
}

// @spec TIME-FEED-004
#[test]
fn window_recommendations_declare_what_they_call_for() {
    for id in [WindowId::SpringPreEmergent, WindowId::FallPreEmergent] {
        let needs = needs_of(id, WindowState::Ideal, GrassType::TallFescue);
        assert_eq!(needs.len(), 1);
        assert_eq!(needs[0].categories, vec![ProductCategory::Herbicide]);
        assert_eq!(
            needs[0].herbicide_timing,
            Some(HerbicideTiming::PreEmergent)
        );
    }

    let seed = |grass| needs_of(WindowId::FallSeeding, WindowState::Ideal, grass);
    let kbg = seed(GrassType::KentuckyBluegrass);
    assert_eq!(kbg[0].categories, vec![ProductCategory::Seed]);
    assert_eq!(kbg[0].targets, vec![ProductTarget::Kbg]);
    assert_eq!(
        seed(GrassType::TallFescue)[0].targets,
        vec![ProductTarget::Ttf]
    );
    assert_eq!(
        seed(GrassType::Mixed)[0].targets,
        vec![
            ProductTarget::Kbg,
            ProductTarget::Ttf,
            ProductTarget::Prg,
            ProductTarget::FineFescue
        ]
    );
}

fn profile(soil_type: Option<SoilType>) -> LawnProfile {
    LawnProfile {
        soil_type,
        ..LawnProfile::default()
    }
}

// @spec TIME-FEED-005
#[test]
fn aeration_rides_on_the_fall_seeding_window() {
    let today = date(2026, 9, 20);
    let at = |state| {
        [timing_window(
            WindowId::FallSeeding,
            state,
            date(2026, 10, 6),
        )]
    };
    let clay = profile(Some(SoilType::Clay));
    let loam = profile(Some(SoilType::Loam));
    let aerated = |on| [applied(ApplicationType::Aeration, on)];

    for state in [WindowState::Open, WindowState::Ideal, WindowState::Closing] {
        let rec = aeration(&at(state), &clay, &[], today).expect("open window");
        assert_eq!(rec.id, "aeration");
        assert_eq!(rec.severity, Severity::Warning, "clay, never aerated");
    }
    let clay_loam = profile(Some(SoilType::ClayLoam));
    assert_eq!(
        aeration(&at(WindowState::Ideal), &clay_loam, &[], today)
            .unwrap()
            .severity,
        Severity::Warning
    );
    assert_eq!(
        aeration(&at(WindowState::Ideal), &loam, &[], today)
            .unwrap()
            .severity,
        Severity::Advisory
    );
    // Aerated last fall (within 12 months, before this season's Jul 1): Info.
    let last_fall = aerated(date(2025, 10, 1));
    assert_eq!(
        aeration(&at(WindowState::Ideal), &clay, &last_fall, today)
            .unwrap()
            .severity,
        Severity::Info
    );
    // Aerated since Jul 1 of the season year: silent.
    let this_season = aerated(date(2026, 7, 1));
    assert!(aeration(&at(WindowState::Ideal), &clay, &this_season, today).is_none());

    for state in [
        WindowState::NotYet,
        WindowState::OpeningSoon,
        WindowState::Closed,
        WindowState::Done,
        WindowState::Blocked,
    ] {
        assert!(
            aeration(&at(state), &clay, &[], today).is_none(),
            "{state:?}"
        );
    }
}

// @spec TIME-FEED-006
#[test]
fn the_fall_herbicide_follows_every_fall_seeding_state() {
    use crate::logic::timing::companions::reconcile_fall_herbicide;
    use crate::models::Recommendation;
    let herbicide = |severity| {
        vec![Recommendation::new(
            "broadleaf_fall",
            RecommendationCategory::Herbicide,
            severity,
            "Fall Broadleaf Herbicide",
            "Best window of the year.",
        )]
    };
    let reconciled = |state, severity| {
        let mut recs = herbicide(severity);
        let windows = [timing_window(
            WindowId::FallSeeding,
            state,
            date(2026, 10, 6),
        )];
        reconcile_fall_herbicide(&mut recs, &windows);
        recs
    };

    for state in [
        WindowState::NotYet,
        WindowState::OpeningSoon,
        WindowState::Open,
        WindowState::Ideal,
        WindowState::Closing,
    ] {
        let recs = reconciled(state, Severity::Warning);
        assert_eq!(recs[0].severity, Severity::Advisory, "{state:?}");
        assert!(recs[0]
            .description
            .starts_with("Only if you are NOT seeding"));
        // Capped, never raised.
        assert_eq!(
            reconciled(state, Severity::Info)[0].severity,
            Severity::Info
        );
    }
    assert!(reconciled(WindowState::Done, Severity::Warning).is_empty());
    for state in [WindowState::Blocked, WindowState::Closed] {
        let recs = reconciled(state, Severity::Warning);
        assert_eq!(recs[0].severity, Severity::Warning);
        assert_eq!(recs[0].description, "Best window of the year.");
    }
}
