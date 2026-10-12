//! A weather-lake outage costs the feed its lake-backed sources, never the feed.

use super::fixtures::{profile, state_without_lake, unconnected_pool};
use crate::clock::Clock;
use crate::models::{Recommendation, RecommendationCategory, Severity};

// @spec FEED-PIPE-015
#[tokio::test]
async fn a_lake_outage_leaves_the_feed_without_disease_advice() {
    let state = state_without_lake(unconnected_pool(), Clock::System).await;
    let recs = crate::api::disease_risk::recommendations(&state, &profile()).await;
    assert!(
        recs.is_empty(),
        "degrades to no disease alerts, not an error"
    );
}

// @spec FEED-PIPE-016
#[tokio::test]
async fn a_lake_outage_adds_no_timing_advice_and_leaves_the_fall_herbicide_alone() {
    let state = state_without_lake(unconnected_pool(), Clock::System).await;
    let broadleaf = Recommendation::new(
        "broadleaf_fall",
        RecommendationCategory::Herbicide,
        Severity::Warning,
        "Fall broadleaf control",
        "Spray broadleaf weeds now.",
    );
    let mut recs = vec![broadleaf.clone()];
    crate::api::timing::apply_to_feed(&state, &profile(), &[], &mut recs).await;

    assert_eq!(recs.len(), 1, "no timing windows and no aeration");
    let after = &recs[0];
    assert_eq!(after.id, broadleaf.id);
    assert_eq!(after.severity, broadleaf.severity);
    assert_eq!(after.title, broadleaf.title);
    assert_eq!(
        after.description, broadleaf.description,
        "no seeding caveat"
    );
}
