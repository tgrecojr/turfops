//! The recommendation feed. The dashboard's "Active Alerts" and the Recommendations page
//! both come from [`active`], so the top 3 on the dashboard are always a subset of the feed.

use crate::db::{plant_queries, product_queries, queries};
use crate::error::TurfOpsError;
use crate::logic::follow_up::generate_follow_up_recommendations;
use crate::logic::inventory::enrich;
use crate::logic::plant_maintenance::generate_plant_maintenance_recommendations;
use crate::logic::rules::nitrogen_guard;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Product, Recommendation, RecommendationState,
};
use crate::state::AppState;
use chrono::Utc;
use std::collections::HashMap;

/// How much application history the rules see. Mowing is an application, so a short window
/// (the dashboard once used 10 rows) drops this season's treatments within weeks.
const RULE_HISTORY_LIMIT: i64 = 1000;

/// Every active recommendation, most severe first.
pub async fn active(
    state: &AppState,
    profile: &LawnProfile,
    summary: &EnvironmentalSummary,
) -> Result<Vec<Recommendation>, TurfOpsError> {
    let mut recommendations = all(state, profile, summary).await?;
    recommendations.retain(|r| r.is_active());
    Ok(recommendations)
}

/// Every current recommendation. One the user has answered carries `dismissed` /
/// `addressed` — but only while that answer still holds (see `RecommendationState`).
pub async fn all(
    state: &AppState,
    profile: &LawnProfile,
    summary: &EnvironmentalSummary,
) -> Result<Vec<Recommendation>, TurfOpsError> {
    Ok(all_with_history(state, profile, summary).await?.0)
}

// @spec FEED-PIPE-001, FEED-PIPE-002, FEED-PIPE-003, FEED-PIPE-012, FEED-PIPE-014, FEED-PIPE-018
/// Every current recommendation together with the application log it was judged on, for
/// readers that must judge the same log (the seasonal plan's rule states).
pub async fn all_with_history(
    state: &AppState,
    profile: &LawnProfile,
    summary: &EnvironmentalSummary,
) -> Result<(Vec<Recommendation>, Vec<Application>), TurfOpsError> {
    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let apps =
        queries::get_applications_for_profile(&state.pool, profile_id, RULE_HISTORY_LIMIT, 0)
            .await?;

    let mut recommendations = state.rules_engine.evaluate(summary, profile, &apps);

    // Per-disease risk (High/Severe tiers) and the seeding / pre-emergent windows.
    recommendations.extend(super::disease_risk::recommendations(state, profile).await);
    super::timing::apply_to_feed(state, profile, &apps, &mut recommendations).await;

    // Plant maintenance for landscape plants.
    let plants = plant_queries::list_plants_for_profile(&state.pool, profile_id).await?;
    let today = state.clock.today();
    recommendations.extend(generate_plant_maintenance_recommendations(
        &plants, &apps, summary, today,
    ));

    // Follow-up reminders for applications that scheduled one.
    let plants_by_id: std::collections::HashMap<i64, &crate::models::plant::Plant> = plants
        .iter()
        .filter_map(|p| p.id.map(|id| (id, p)))
        .collect();
    recommendations.extend(generate_follow_up_recommendations(
        &apps,
        &plants_by_id,
        today,
    ));

    // A shelf that can't be read costs the inventory badges, not the feed.
    let products = product_queries::list_products_for_profile(&state.pool, profile_id, None, false)
        .await
        .inspect_err(|e| tracing::warn!("Skipping inventory in the feed: {e}"))
        .ok();
    let rec_states = queries::get_recommendation_states(&state.pool).await?;
    let feed = assemble(
        recommendations,
        &Stages {
            profile,
            history: &apps,
            today,
            products: products.as_deref(),
            answers: &rec_states,
            now: Utc::now(),
        },
    );
    Ok((feed, apps))
}

/// What the stages after the sources read.
pub(crate) struct Stages<'a> {
    pub profile: &'a LawnProfile,
    pub history: &'a [Application],
    pub today: chrono::NaiveDate,
    /// `None` when the shelf could not be read: the inventory pass is skipped.
    pub products: Option<&'a [Product]>,
    pub answers: &'a HashMap<String, RecommendationState>,
    pub now: chrono::DateTime<Utc>,
}

/// Everything after the sources, in order: the nitrogen guard over marked feedings, the
/// shelf, the owner's answers that still hold, then one stable sort by severity.
// @spec FEED-PIPE-004, FEED-PIPE-006, FEED-PIPE-007, FEED-PIPE-008, FEED-PIPE-009, FEED-PIPE-005, FEED-STATE-009
pub(crate) fn assemble(
    mut recommendations: Vec<Recommendation>,
    stages: &Stages,
) -> Vec<Recommendation> {
    // No "feed the lawn" next to "avoid fertilizer", and none past the annual budget,
    // whichever source the feeding came from.
    nitrogen_guard::apply(
        &mut recommendations,
        stages.profile,
        stages.history,
        stages.today,
    );
    // @spec INV-ENRICH-005
    if let Some(products) = stages.products {
        enrich(&mut recommendations, products);
    }
    for rec in &mut recommendations {
        if let Some(answer) = stages
            .answers
            .get(&rec.id)
            .filter(|a| a.suppresses(rec, stages.now))
        {
            rec.dismissed = answer.dismissed;
            rec.addressed = answer.addressed;
        }
    }
    // Most severe first; equal severities keep source order. Every surface uses this order.
    recommendations.sort_by_key(|r| std::cmp::Reverse(r.severity));
    recommendations
}

#[cfg(all(test, feature = "db-tests"))]
mod feed_db_tests;
#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod outage_tests;
#[cfg(test)]
mod stage_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GrassType, RecommendationCategory, Severity};

    fn profile() -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type: GrassType::TallFescue,
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn rec(id: &str, severity: Severity) -> Recommendation {
        Recommendation::new(id, RecommendationCategory::General, severity, id, id)
    }

    fn assembled(recs: Vec<Recommendation>) -> Vec<Recommendation> {
        assembled_with(recs, Some(&[]))
    }

    fn assembled_with(
        recs: Vec<Recommendation>,
        products: Option<&[Product]>,
    ) -> Vec<Recommendation> {
        let profile = profile();
        let answers = HashMap::new();
        assemble(
            recs,
            &Stages {
                profile: &profile,
                history: &[],
                today: chrono::NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
                products,
                answers: &answers,
                now: Utc::now(),
            },
        )
    }

    // @spec INV-ENRICH-005
    #[test]
    fn without_a_readable_shelf_the_feed_has_no_inventory_verdicts() {
        let need = crate::models::ProductNeed::new(
            "grub preventative",
            crate::models::ProductCategory::InsectControl,
        );
        let grubs = rec("grub_control_2026", Severity::Advisory).with_need(need);
        let shelf_down = assembled_with(vec![grubs.clone()], None);
        assert!(shelf_down[0].inventory.is_none(), "not \"need to buy\"");
        assert!(!shelf_down[0]
            .data_points
            .iter()
            .any(|d| d.label == "Not on shelf"));
        let empty_shelf = assembled_with(vec![grubs], Some(&[]));
        assert!(
            empty_shelf[0].inventory.is_some(),
            "an empty shelf is still a verdict"
        );
    }

    // @spec FEED-PIPE-004, FEED-PIPE-006
    #[test]
    fn the_guard_covers_a_feeding_from_any_source_once() {
        // A disease source's feeding, appended after the rules, still waits out the block.
        let red_thread = rec("disease_red_thread", Severity::Advisory).with_feeding(0.5);
        let block = rec("fertilizer_block", Severity::Warning);
        let out = assembled(vec![block, red_thread]);
        let red_thread = out.iter().find(|r| r.id == "disease_red_thread").unwrap();
        assert_eq!(red_thread.severity, Severity::Info);
        assert!(red_thread.title.starts_with("On hold"));
        let budget_points = red_thread
            .data_points
            .iter()
            .filter(|d| d.label == "N budget remaining")
            .count();
        assert_eq!(budget_points, 1);
    }

    // @spec FEED-PIPE-009
    #[test]
    fn the_feed_is_sorted_by_severity_keeping_source_order_for_ties() {
        let out = assembled(vec![
            rec("a", Severity::Info),
            rec("b", Severity::Critical),
            rec("c", Severity::Warning),
            rec("d", Severity::Info),
            rec("e", Severity::Warning),
        ]);
        let ids: Vec<_> = out.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c", "e", "a", "d"]);
    }
}
