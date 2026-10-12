//! What the spring nitrogen rule says, by soil temperature, GDD and the log.
//!
//! The rule reads its date from `chrono::Local`, so the decision table can only run in the
//! months it covers; on any other day the same inputs must produce nothing. The advice each
//! branch builds is checked on every run.

use super::*;
use crate::logic::rules::test_fixtures::{all_text, feeding_on, fescue, soil, today};

/// Feb–Apr: the soil/GDD branches run. May: the cutoff. Otherwise: nothing.
fn month() -> u32 {
    today().month()
}

fn decide(avg: f64, gdd: Option<f64>, fed: bool) -> Option<(String, Severity)> {
    let mut env = soil(Some(avg), None);
    env.gdd_base50_ytd = gdd;
    let history = if fed {
        vec![feeding_on(today())]
    } else {
        vec![]
    };
    SpringNitrogenRule
        .evaluate(
            &env,
            &fescue(),
            &history,
            crate::clock::Clock::System.moment(),
        )
        .map(|r| (r.id, r.severity))
}

fn expect(id: &str, severity: Severity) -> Option<(String, Severity)> {
    Some((id.to_string(), severity))
}

// @spec RULE-SPRING-001
#[test]
fn spring_nitrogen_runs_february_through_may_and_needs_the_soil_average() {
    let mut no_soil = soil(None, Some(58.0));
    no_soil.gdd_base50_ytd = Some(120.0);
    assert!(SpringNitrogenRule
        .evaluate(
            &no_soil,
            &fescue(),
            &[],
            crate::clock::Clock::System.moment()
        )
        .is_none());
    let fires = decide(58.0, Some(120.0), false).is_some();
    assert_eq!(fires, (2..=5).contains(&month()), "month {}", month());
}

// @spec RULE-SPRING-002
#[test]
fn may_without_a_spring_feeding_advises_skipping_spring_nitrogen() {
    let rec = build_may_cutoff_warning(62.0);
    assert_eq!(
        (rec.id.as_str(), rec.severity),
        ("spring_n_may_cutoff", Severity::Info)
    );
    assert!(all_text(&rec).contains("SKIPPING spring nitrogen"));
    assert!(
        rec.feeding.is_none(),
        "advice against feeding is not a feeding"
    );
    if month() == 5 {
        for avg in [45.0, 58.0, 70.0] {
            assert_eq!(
                decide(avg, None, false),
                expect("spring_n_may_cutoff", Severity::Info)
            );
            assert_eq!(decide(avg, None, true), None);
        }
    }
}

// @spec RULE-SPRING-003, RULE-SPRING-004
#[test]
fn cold_soil_warns_a_fed_lawn_and_tells_an_unfed_one_to_wait() {
    let too_early = build_too_early_warning(46.0);
    assert_eq!(
        (too_early.id.as_str(), too_early.severity),
        ("spring_n_too_early", Severity::Warning)
    );
    let wait = build_patience_advisory(46.0);
    assert_eq!(
        (wait.id.as_str(), wait.severity),
        ("spring_n_wait", Severity::Info)
    );
    if (2..=4).contains(&month()) {
        assert_eq!(
            decide(49.9, Some(80.0), true),
            expect("spring_n_too_early", Severity::Warning)
        );
        assert_eq!(
            decide(49.9, Some(80.0), false),
            expect("spring_n_wait", Severity::Info)
        );
    }
}

// @spec RULE-SPRING-005, RULE-SPRING-006, RULE-ENG-004
#[test]
fn near_fifty_five_gdd_decides_between_ready_and_almost() {
    let almost = build_almost_ready(52.0, None);
    assert_eq!(
        (almost.id.as_str(), almost.severity),
        ("spring_n_almost", Severity::Info)
    );
    assert!(almost.data_points.iter().all(|d| !d.label.contains("GDD")));
    let ready = build_ready_to_fertilize(52.0, &fescue(), Some(50.0));
    assert_eq!(
        (ready.id.as_str(), ready.severity),
        ("spring_n_ready", Severity::Advisory)
    );
    if (2..=4).contains(&month()) {
        assert_eq!(
            decide(50.0, Some(50.0), false),
            expect("spring_n_ready", Severity::Advisory)
        );
        assert_eq!(
            decide(54.9, Some(49.0), false),
            expect("spring_n_almost", Severity::Info)
        );
        // No GDD record: the soil still decides — "almost", never silence.
        assert_eq!(
            decide(52.0, None, false),
            expect("spring_n_almost", Severity::Info)
        );
        assert_eq!(decide(52.0, Some(80.0), true), None);
    }
}

// @spec RULE-SPRING-007, RULE-ENG-004
#[test]
fn fifty_five_to_sixty_five_is_ready_with_a_gdd_note() {
    let note = |gdd| build_ready_to_fertilize(58.0, &fescue(), gdd).explanation;
    assert!(note(Some(150.0)).contains("well established"));
    assert!(note(Some(60.0)).contains("confirms active growth"));
    assert!(!note(Some(60.0)).contains("well established"));
    let plain = note(None);
    assert!(!plain.contains("GDD"), "{plain}");
    if (2..=4).contains(&month()) {
        for avg in [55.0, 60.0, 65.0] {
            assert_eq!(
                decide(avg, None, false),
                expect("spring_n_ready", Severity::Advisory)
            );
            assert_eq!(decide(avg, None, true), None);
        }
        assert_eq!(decide(65.1, None, false), None);
    }
}

// @spec RULE-SPRING-009
#[test]
fn waiting_advice_points_to_the_timing_page_for_pre_emergent() {
    for rec in [
        build_patience_advisory(45.0),
        build_almost_ready(52.0, Some(30.0)),
        build_too_early_warning(45.0),
    ] {
        let text = all_text(&rec);
        assert!(
            text.contains("Seed & Pre-Em Timing page"),
            "{}: {text}",
            rec.id
        );
        let lower = text.to_lowercase();
        assert!(!lower.contains("apply pre-emergent"), "{}: {text}", rec.id);
        assert!(!lower.contains("crabgrass"), "{}: {text}", rec.id);
        assert!(rec.needs.is_empty(), "{} asks for no product", rec.id);
    }
}

// @spec RULE-ENG-008
#[test]
fn spring_feeding_states_the_rate_and_the_lawns_total() {
    let rec = build_ready_to_fertilize(58.0, &fescue(), None);
    let action = rec.suggested_action.clone().unwrap_or_default();
    assert!(
        action.contains("about 2.5 lb of nitrogen for your 5000 sq ft lawn (0.5 lb N/1000 sq ft)"),
        "{action}"
    );
    let mut unsized_lawn = fescue();
    unsized_lawn.lawn_size_sqft = None;
    let rec = build_ready_to_fertilize(58.0, &unsized_lawn, None);
    let action = rec.suggested_action.clone().unwrap_or_default();
    assert!(action.contains("Apply 0.5 lb N/1000 sq ft."), "{action}");
    assert!(!all_text(&rec).contains("label rate"));
}
