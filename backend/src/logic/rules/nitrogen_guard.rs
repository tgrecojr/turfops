//! Cross-rule sanity for the recommendations that say "apply nitrogen". Each feeding rule
//! judges its own window; none of them sees the other rules or the annual budget. Left
//! alone they tell the user to feed the lawn next to "Avoid Fertilizer Application", and
//! their fixed fall program overshoots the annual target of a low-nitrogen grass
//! (3.25 lb vs. fine fescue's 3.0 lb maximum). This pass runs after the rules.

use crate::models::nitrogen_budget::annual_n_target;
use crate::models::{Application, DataSource, LawnProfile, Recommendation, Severity};
use chrono::{Datelike, NaiveDate};

/// Recommendations whose action is "apply nitrogen now".
const FEEDING_IDS: [&str; 4] = [
    "fall_fert_early",
    "fall_fert_mid",
    "fall_fert_winterizer",
    "spring_n_ready",
];
/// Rules that say conditions are wrong for fertilizer right now.
const FERTILIZER_BLOCK_ID: &str = "fertilizer_block";
const HEAT_STRESS_ID: &str = "heat_stress_forecast";

/// The largest single feeding any rule suggests (lb N / 1000 sq ft).
const LARGEST_FEEDING_LBS: f64 = 1.0;
/// Below this much room the target counts as reached.
const BUDGET_EPSILON_LBS: f64 = 0.05;

fn is_feeding(rec: &Recommendation) -> bool {
    FEEDING_IDS.contains(&rec.id.as_str())
}

/// Lawn nitrogen applied in `today`'s year (lb N / 1000 sq ft).
fn applied_this_year(history: &[Application], today: NaiveDate) -> f64 {
    history
        .iter()
        .filter(|a| a.application_date.year() == today.year() && a.application_date <= today)
        .filter_map(Application::turf_nitrogen_lbs)
        .sum()
}

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

    for rec in recommendations.iter_mut().filter(|r| is_feeding(r)) {
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
        } else if remaining < LARGEST_FEEDING_LBS {
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

    fn feeding() -> Recommendation {
        Recommendation::new(
            "fall_fert_mid",
            RecommendationCategory::Fertilizer,
            Severity::Warning,
            "Main Fall Feeding",
            "Apply 1.0 lb N.",
        )
        .with_action("Apply now")
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
        assert_eq!(recs[0].description, "Apply 1.0 lb N.");
        assert!(recs[0]
            .data_points
            .iter()
            .any(|d| d.label == "N budget remaining"));
    }
}
