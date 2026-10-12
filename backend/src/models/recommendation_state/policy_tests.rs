//! When a stored answer hides a recommendation, and for how long.

use super::*;

fn rec(id: &str, category: RecommendationCategory, severity: Severity) -> Recommendation {
    Recommendation::new(id, category, severity, "t", "d")
}

fn state(
    dismissed: bool,
    addressed: bool,
    severity: Option<Severity>,
    days_ago: i64,
) -> RecommendationState {
    RecommendationState {
        dismissed,
        addressed,
        severity,
        updated_at: Utc::now() - Duration::days(days_ago),
    }
}

fn seasonal(severity: Severity) -> Recommendation {
    rec("aeration", RecommendationCategory::Aeration, severity)
}

// @spec FEED-STATE-002
#[test]
fn an_answer_hides_only_while_set_not_escalated_and_in_its_episode() {
    let now = Utc::now();
    let advisory = seasonal(Severity::Advisory);
    assert!(state(true, false, Some(Severity::Advisory), 10).suppresses(&advisory, now));
    // Each condition failing on its own brings it back.
    assert!(!state(false, false, Some(Severity::Advisory), 10).suppresses(&advisory, now));
    assert!(!state(true, false, Some(Severity::Info), 10).suppresses(&advisory, now));
    assert!(!state(true, false, Some(Severity::Advisory), 121).suppresses(&advisory, now));
}

// @spec FEED-STATE-003
#[test]
fn a_rec_more_severe_than_its_answer_is_active() {
    let now = Utc::now();
    let answered = state(false, true, Some(Severity::Advisory), 0);
    for severity in [Severity::Warning, Severity::Critical] {
        assert!(!answered.suppresses(&seasonal(severity), now), "{severity}");
    }
}

// @spec FEED-STATE-004
#[test]
fn a_rec_less_severe_than_its_answer_stays_hidden_until_the_episode_ends() {
    let now = Utc::now();
    let calmer = seasonal(Severity::Info);
    assert!(state(true, false, Some(Severity::Warning), 100).suppresses(&calmer, now));
    assert!(!state(true, false, Some(Severity::Warning), 121).suppresses(&calmer, now));
}

// @spec FEED-STATE-005
#[test]
fn an_answer_without_a_severity_never_escalates_and_only_expires() {
    let now = Utc::now();
    for severity in [Severity::Info, Severity::Critical] {
        assert!(state(true, false, None, 119).suppresses(&seasonal(severity), now));
        assert!(!state(true, false, None, 121).suppresses(&seasonal(severity), now));
    }
}

// @spec FEED-STATE-006
#[test]
fn episodes_follow_the_first_matching_rule() {
    use RecommendationCategory as C;
    let cases: [(&str, C, Option<i64>); 13] = [
        // An id rule beats any category.
        ("application_followup_7", C::DiseasePressure, None),
        ("fertilizer_block", C::Fertilizer, Some(7)),
        ("irrigation_forecast", C::Irrigation, Some(7)),
        ("heat_stress_forecast", C::HeatStress, Some(7)),
        ("frost", C::FrostWarning, Some(7)),
        ("mowing_height", C::Mowing, Some(7)),
        ("rain_delay", C::ApplicationTiming, Some(7)),
        ("plant_3_watering_2026-07-01", C::PlantMaintenance, Some(7)),
        ("disease_dollar_spot", C::DiseasePressure, Some(21)),
        ("fall_fert_early", C::Fertilizer, Some(120)),
        ("timing_fall_seeding_2026", C::Overseeding, Some(120)),
        ("plant_3_pruning_2026-03-01", C::PlantMaintenance, Some(120)),
        ("anything_else", C::General, Some(120)),
    ];
    for (id, category, days) in cases {
        assert_eq!(
            episode_days(&rec(id, category, Severity::Info)),
            days,
            "{id}"
        );
    }
}

// @spec FEED-STATE-007
#[test]
fn the_episode_runs_from_the_last_answer() {
    let rain = rec(
        "rain_delay",
        RecommendationCategory::ApplicationTiming,
        Severity::Warning,
    );
    let answered_at = Utc::now() - Duration::days(30);
    let row = RecommendationState {
        updated_at: answered_at,
        ..state(true, false, Some(Severity::Warning), 0)
    };
    assert!(row.suppresses(&rain, answered_at + Duration::days(6)));
    assert!(!row.suppresses(&rain, answered_at + Duration::days(7)));
}

// @spec FEED-STATE-008
#[test]
fn dismissed_and_addressed_follow_the_same_policy() {
    let now = Utc::now();
    let recs = [
        seasonal(Severity::Info),
        seasonal(Severity::Critical),
        rec(
            "disease_brown_patch",
            RecommendationCategory::DiseasePressure,
            Severity::Warning,
        ),
    ];
    for days_ago in [0, 10, 25, 200] {
        for severity in [None, Some(Severity::Advisory)] {
            let dismissed = state(true, false, severity, days_ago);
            let addressed = state(false, true, severity, days_ago);
            for r in &recs {
                assert_eq!(dismissed.suppresses(r, now), addressed.suppresses(r, now));
            }
        }
    }
}
