//! What a click on "Addressed" / "Dismissed" means, and for how long.
//!
//! A click answers *this* alert at *this* severity, not the rule forever. The alert comes
//! back when it escalates past what was answered (Info "opening soon" → Warning "closing",
//! or a protected lawn's Info → Severe once the fungicide lapses), or when the episode the
//! click belonged to is over. Policy lives here so no rule has to encode it in its id.

use super::{Recommendation, RecommendationCategory, Severity};
use chrono::{DateTime, Duration, Utc};

/// Persisted answer to one recommendation id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecommendationState {
    pub dismissed: bool,
    pub addressed: bool,
    /// Severity shown when the user clicked. `None` for rows written before this was
    /// tracked; those can only expire, never escalate.
    pub severity: Option<Severity>,
    pub updated_at: DateTime<Utc>,
}

/// Weather episodes (rain, heat, dry spell) are over within a week.
const WEATHER_EPISODE_DAYS: i64 = 7;
/// A systemic fungicide's residual window — after it the disease question is open again.
const DISEASE_EPISODE_DAYS: i64 = 21;
/// Seasonal work (fall feeding, aeration, herbicide): long enough to cover the window it
/// was answered in, short enough to be gone before the same season next year.
const SEASON_DAYS: i64 = 120;

/// How long an answer to `rec` holds. `None` = until it escalates (ids that already name
/// one event, e.g. a follow-up for one application).
fn episode_days(rec: &Recommendation) -> Option<i64> {
    if rec.id.starts_with("application_followup_") {
        return None;
    }
    if rec.id == "fertilizer_block" {
        return Some(WEATHER_EPISODE_DAYS);
    }
    Some(match rec.category {
        RecommendationCategory::Irrigation
        | RecommendationCategory::HeatStress
        | RecommendationCategory::FrostWarning
        | RecommendationCategory::Mowing
        | RecommendationCategory::ApplicationTiming => WEATHER_EPISODE_DAYS,
        RecommendationCategory::DiseasePressure => DISEASE_EPISODE_DAYS,
        _ => SEASON_DAYS,
    })
}

impl RecommendationState {
    /// Does this stored answer still hide `rec`?
    pub fn suppresses(&self, rec: &Recommendation, now: DateTime<Utc>) -> bool {
        if !self.dismissed && !self.addressed {
            return false;
        }
        if self
            .severity
            .is_some_and(|answered| rec.severity > answered)
        {
            return false;
        }
        match episode_days(rec) {
            Some(days) => now - self.updated_at < Duration::days(days),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, category: RecommendationCategory, severity: Severity) -> Recommendation {
        Recommendation::new(id, category, severity, "t", "d")
    }

    fn answered(severity: Option<Severity>, days_ago: i64) -> RecommendationState {
        RecommendationState {
            dismissed: true,
            addressed: false,
            severity,
            updated_at: Utc::now() - Duration::days(days_ago),
        }
    }

    #[test]
    fn escalation_brings_an_alert_back() {
        let state = answered(Some(Severity::Info), 1);
        let opening = rec(
            "timing_fall_seeding_2026",
            RecommendationCategory::Overseeding,
            Severity::Info,
        );
        let closing = Recommendation {
            severity: Severity::Warning,
            ..opening.clone()
        };
        assert!(state.suppresses(&opening, Utc::now()));
        assert!(!state.suppresses(&closing, Utc::now()));
    }

    #[test]
    fn disease_answer_lapses_with_the_residual_window() {
        let high = rec(
            "disease_brown_patch",
            RecommendationCategory::DiseasePressure,
            Severity::Warning,
        );
        assert!(answered(Some(Severity::Warning), 20).suppresses(&high, Utc::now()));
        assert!(!answered(Some(Severity::Warning), 22).suppresses(&high, Utc::now()));
    }

    #[test]
    fn seasonal_answer_is_gone_by_next_year() {
        let feeding = rec(
            "fall_fert_early",
            RecommendationCategory::Fertilizer,
            Severity::Warning,
        );
        assert!(answered(Some(Severity::Warning), 30).suppresses(&feeding, Utc::now()));
        assert!(!answered(Some(Severity::Warning), 365).suppresses(&feeding, Utc::now()));
    }

    #[test]
    fn weather_answers_are_short_lived() {
        let rain = rec(
            "rain_delay",
            RecommendationCategory::ApplicationTiming,
            Severity::Warning,
        );
        let block = rec(
            "fertilizer_block",
            RecommendationCategory::Fertilizer,
            Severity::Warning,
        );
        for r in [&rain, &block] {
            assert!(answered(Some(Severity::Warning), 3).suppresses(r, Utc::now()));
            assert!(!answered(Some(Severity::Warning), 8).suppresses(r, Utc::now()));
        }
    }

    #[test]
    fn follow_up_for_one_application_stays_answered() {
        let follow_up = rec(
            "application_followup_42",
            RecommendationCategory::ApplicationTiming,
            Severity::Advisory,
        );
        assert!(answered(Some(Severity::Advisory), 200).suppresses(&follow_up, Utc::now()));
    }

    #[test]
    fn rows_from_before_severity_was_tracked_only_expire() {
        let severe = rec(
            "disease_brown_patch",
            RecommendationCategory::DiseasePressure,
            Severity::Critical,
        );
        assert!(answered(None, 5).suppresses(&severe, Utc::now()));
        assert!(!answered(None, 400).suppresses(&severe, Utc::now()));
    }

    #[test]
    fn a_cleared_row_suppresses_nothing() {
        let state = RecommendationState {
            dismissed: false,
            addressed: false,
            ..answered(Some(Severity::Critical), 0)
        };
        let r = rec("aeration", RecommendationCategory::Aeration, Severity::Info);
        assert!(!state.suppresses(&r, Utc::now()));
    }
}
