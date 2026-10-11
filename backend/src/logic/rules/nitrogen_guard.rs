//! Cross-rule sanity for the recommendations that say "apply nitrogen". Each feeding rule
//! judges its own window; none of them sees the other rules or the annual budget. Left
//! alone they tell the user to feed the lawn next to "Avoid Fertilizer Application", and
//! their fixed fall program overshoots the annual target of a low-nitrogen grass
//! (3.25 lb vs. fine fescue's 3.0 lb maximum). The feed runs this pass once, after every
//! source, over the recommendations their source marked as feedings.

use crate::models::nitrogen_budget::annual_n_target;
use crate::models::{Application, DataSource, LawnProfile, Recommendation, Severity};
use chrono::{Datelike, NaiveDate};

/// Rules that say conditions are wrong for fertilizer right now.
const FERTILIZER_BLOCK_ID: &str = "fertilizer_block";
const HEAT_STRESS_ID: &str = "heat_stress_forecast";

/// The largest single feeding any rule suggests (lb N / 1000 sq ft).
const LARGEST_FEEDING_LBS: f64 = 1.0;
/// Below this much room the target counts as reached.
const BUDGET_EPSILON_LBS: f64 = 0.05;

/// Label of the data point that states a feeding's nitrogen rate.
pub const RATE_POINT: &str = "Recommended Rate";

/// Pounds, without trailing zeros: 0.5, 0.75, 1.
fn lbs(x: f64) -> String {
    let s = format!("{x:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A feeding's rate as every feeding source states it: "0.75 lb N/1000 sq ft".
pub fn rate_text(n_lbs_per_ksqft: f64) -> String {
    format!("{} lb N/1000 sq ft", lbs(n_lbs_per_ksqft))
}

/// A feeding's amount as every feeding source states it in its text: "about 3.75 lb of
/// nitrogen for your 5000 sq ft lawn (0.75 lb N/1000 sq ft)". The guard re-renders this
/// exact phrase when it caps a feeding.
// @spec RULE-ENG-009
pub fn amount_text(n_lbs_per_ksqft: f64, lawn_sqft: Option<f64>) -> String {
    // With no lawn size recorded, a total would be invented: state the rate alone.
    let Some(lawn_sqft) = lawn_sqft else {
        return rate_text(n_lbs_per_ksqft);
    };
    format!(
        "about {} lb of nitrogen for your {:.0} sq ft lawn ({})",
        lbs(lawn_sqft / 1000.0 * n_lbs_per_ksqft),
        lawn_sqft,
        rate_text(n_lbs_per_ksqft)
    )
}

/// Re-state a feeding's amount as `capped` wherever the source stated its own `n`.
fn cap_amount(rec: &mut Recommendation, n: f64, capped: f64, lawn_sqft: Option<f64>) {
    let restate = |text: &str| -> String {
        text.replace(&amount_text(n, lawn_sqft), &amount_text(capped, lawn_sqft))
            .replace(&rate_text(n), &rate_text(capped))
    };
    rec.suggested_action = rec.suggested_action.as_deref().map(restate);
    rec.description = restate(&rec.description);
    for point in rec.data_points.iter_mut().filter(|d| d.label == RATE_POINT) {
        point.value = rate_text(capped);
    }
    rec.feeding = Some(crate::models::Feeding {
        n_lbs_per_ksqft: capped,
        target_reached: false,
    });
}

/// Lawn nitrogen applied in `today`'s year (lb N / 1000 sq ft).
fn applied_this_year(history: &[Application], today: NaiveDate) -> f64 {
    history
        .iter()
        .filter(|a| a.application_date.year() == today.year() && a.application_date <= today)
        .filter_map(Application::turf_nitrogen_lbs)
        .sum()
}

// @spec RULE-GUARD-001, RULE-GUARD-003, RULE-GUARD-004, RULE-GUARD-006, RULE-GUARD-007, RULE-GUARD-008
pub fn apply(
    recommendations: &mut [Recommendation],
    profile: &LawnProfile,
    history: &[Application],
    today: NaiveDate,
) {
    let blocked = recommendations.iter().any(|r| {
        r.id == FERTILIZER_BLOCK_ID || (r.id == HEAT_STRESS_ID && r.severity >= Severity::Warning)
    });
    let target = annual_n_target(profile.grass_type);
    let applied = applied_this_year(history, today);
    let remaining = target.recommended_lbs_per_1000sqft - applied;

    let lawn_sqft = profile.lawn_size_sqft;
    for rec in recommendations.iter_mut() {
        let Some(feeding) = rec.feeding else {
            continue;
        };
        rec.data_points.push(crate::models::DataPoint::new(
            "N budget remaining",
            format!(
                "{:.2} of {:.1} lb/1000 sqft",
                remaining.max(0.0),
                target.recommended_lbs_per_1000sqft
            ),
            DataSource::Calculated.as_str(),
        ));

        if remaining < BUDGET_EPSILON_LBS {
            rec.severity = Severity::Info;
            rec.title = format!("{} — annual nitrogen target reached", rec.title);
            rec.description = format!(
                "You have applied {applied:.2} lb N/1000 sqft this year against a target of \
                 {:.1} for {}. Skip this feeding, or use a product without nitrogen.",
                target.recommended_lbs_per_1000sqft,
                profile.grass_type.as_str()
            );
            rec.suggested_action = None;
            rec.needs.clear();
            // @spec RULE-GUARD-011
            rec.feeding = Some(crate::models::Feeding {
                target_reached: true,
                ..feeding
            });
        } else if remaining < LARGEST_FEEDING_LBS {
            if remaining < feeding.n_lbs_per_ksqft {
                cap_amount(rec, feeding.n_lbs_per_ksqft, remaining, lawn_sqft);
            }
            rec.description = format!(
                "{} Keep this application at or under {remaining:.2} lb N/1000 sqft to stay \
                 within this year's target.",
                rec.description
            );
        }

        if blocked {
            rec.severity = Severity::Info;
            rec.title = format!("On hold — {}", rec.title);
            rec.description = format!(
                "Hold off for now: current conditions rule out fertilizer (see the heat / \
                 soil-moisture alert). {}",
                rec.description
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ApplicationType, GrassType, RecommendationCategory};

    fn profile(grass_type: GrassType) -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type,
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 5).unwrap()
    }

    /// A feeding as the sources build one: marked, with its amount stated through the
    /// shared phrases (5000 sq ft lawn, 1 lb N/1000 sq ft).
    fn feeding() -> Recommendation {
        Recommendation::new(
            "fall_fert_mid",
            RecommendationCategory::Fertilizer,
            Severity::Warning,
            "Main Fall Feeding",
            format!("Feed now: {}.", amount_text(1.0, Some(5000.0))),
        )
        .with_data_point(RATE_POINT, rate_text(1.0), DataSource::Agronomic.as_str())
        .with_action(format!(
            "Apply {}. Use slow-release nitrogen.",
            amount_text(1.0, Some(5000.0))
        ))
        .with_feeding(1.0)
    }

    fn block() -> Recommendation {
        Recommendation::new(
            FERTILIZER_BLOCK_ID,
            RecommendationCategory::Fertilizer,
            Severity::Warning,
            "Avoid Fertilizer Application",
            "Saturated soil.",
        )
    }

    // @spec RULE-GUARD-001
    #[test]
    fn the_guard_acts_on_the_mark_not_the_id() {
        // A feeding from any source, under any id, is held…
        let mut marked = feeding();
        marked.id = "disease_red_thread".into();
        // …and a rule id that happens to be a feeding id but is not marked is left alone.
        let mut unmarked = feeding();
        unmarked.feeding = None;
        unmarked.id = "fall_fert_early".into();
        let mut recs = vec![marked, unmarked, block()];
        apply(&mut recs, &profile(GrassType::TallFescue), &[], today());
        assert_eq!(recs[0].severity, Severity::Info);
        assert!(recs[0].title.starts_with("On hold"));
        assert_eq!(recs[1].severity, Severity::Warning);
        assert!(!recs[1].title.starts_with("On hold"));
        assert!(recs[1]
            .data_points
            .iter()
            .all(|d| d.label != "N budget remaining"));
    }

    // @spec RULE-GUARD-007
    #[test]
    fn a_capped_feeding_states_no_more_nitrogen_than_remains() {
        // Tall fescue target 3.5; 3.0 down leaves 0.5 — half the suggested 1.0.
        let history = [fed(1.5, 4), fed(1.5, 9)];
        let mut recs = vec![feeding()];
        apply(
            &mut recs,
            &profile(GrassType::TallFescue),
            &history,
            today(),
        );
        let rec = &recs[0];
        let original = amount_text(1.0, Some(5000.0));
        let capped = amount_text(0.5, Some(5000.0));
        let action = rec.suggested_action.as_deref().unwrap_or_default();
        assert!(!rec.description.contains(&original), "{}", rec.description);
        assert!(!action.contains(&original), "{action}");
        assert!(action.contains(&capped), "{action}");
        assert!(
            action.contains("slow-release"),
            "other advice is kept: {action}"
        );
        let rate = rec
            .data_points
            .iter()
            .find(|d| d.label == RATE_POINT)
            .unwrap();
        assert_eq!(rate.value, rate_text(0.5));
        assert_eq!(rec.severity, Severity::Warning);
    }

    fn fed(lbs_n: f64, month: u32) -> Application {
        Application {
            id: None,
            lawn_profile_id: 1,
            application_type: ApplicationType::Fertilizer,
            product_name: None,
            application_date: NaiveDate::from_ymd_opt(2026, month, 1).unwrap(),
            rate_per_1000sqft: Some(lbs_n * 100.0 / 20.0),
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            nitrogen_pct: Some(20.0),
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: None,
            frac_classes: None,
            product_id: None,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn feeding_is_put_on_hold_while_fertilizer_is_blocked() {
        let mut recs = vec![
            feeding(),
            Recommendation::new(
                FERTILIZER_BLOCK_ID,
                RecommendationCategory::Fertilizer,
                Severity::Warning,
                "Avoid Fertilizer Application",
                "Saturated soil.",
            ),
        ];
        apply(&mut recs, &profile(GrassType::TallFescue), &[], today());
        assert_eq!(recs[0].severity, Severity::Info);
        assert!(recs[0].title.starts_with("On hold"));
        assert_eq!(recs[1].severity, Severity::Warning);
    }

    #[test]
    fn feeding_stops_at_the_grass_types_annual_target() {
        // Fine fescue: 2.0 lb target. 2.0 already down → no more "apply 1.0 lb N".
        let history = [fed(1.0, 4), fed(1.0, 9)];
        let mut recs = vec![feeding()];
        apply(
            &mut recs,
            &profile(GrassType::FineFescue),
            &history,
            today(),
        );
        assert_eq!(recs[0].severity, Severity::Info);
        assert!(recs[0].title.contains("target reached"));
        assert!(recs[0].suggested_action.is_none());
    }

    // @spec RULE-GUARD-011
    #[test]
    fn target_reached_is_marked_on_the_feeding_itself() {
        let mut spent = vec![feeding()];
        let history = [fed(1.0, 4), fed(1.0, 9)];
        apply(
            &mut spent,
            &profile(GrassType::FineFescue),
            &history,
            today(),
        );
        assert!(spent[0].feeding.is_some_and(|f| f.target_reached));

        let mut capped = vec![feeding()];
        let history = [fed(1.0, 4), fed(0.5, 9)];
        apply(
            &mut capped,
            &profile(GrassType::FineFescue),
            &history,
            today(),
        );
        assert!(capped[0].feeding.is_some_and(|f| !f.target_reached));
    }

    #[test]
    fn a_nearly_spent_budget_caps_the_feeding() {
        let history = [fed(1.0, 4), fed(0.5, 9)]; // fine fescue: 0.5 lb left
        let mut recs = vec![feeding()];
        apply(
            &mut recs,
            &profile(GrassType::FineFescue),
            &history,
            today(),
        );
        assert_eq!(recs[0].severity, Severity::Warning);
        assert!(recs[0].description.contains("at or under 0.50"));
    }

    #[test]
    fn plenty_of_budget_changes_nothing_but_the_data_point() {
        let mut recs = vec![feeding()];
        apply(&mut recs, &profile(GrassType::TallFescue), &[], today());
        assert_eq!(recs[0].severity, Severity::Warning);
        assert_eq!(recs[0].description, feeding().description);
        assert!(recs[0]
            .data_points
            .iter()
            .any(|d| d.label == "N budget remaining"));
    }

    // @spec RULE-ENG-009
    #[test]
    fn without_a_lawn_size_the_amount_is_per_thousand_square_feet_only() {
        let text = amount_text(0.75, None);
        assert_eq!(text, rate_text(0.75));
        assert!(!text.contains("lawn"), "{text}");
    }
}
