pub mod engine;
pub mod fertilizer;
pub mod fungicide;
pub mod grub_control;
pub mod pre_emergent;

pub use engine::RulesEngine;

use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};

/// Trait for agronomic rules
pub trait Rule: Send + Sync {
    /// Unique identifier for this rule
    fn id(&self) -> &'static str;

    /// Human-readable name
    fn name(&self) -> &'static str;

    /// Evaluate the rule and return a recommendation if conditions are met
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation>;
}
