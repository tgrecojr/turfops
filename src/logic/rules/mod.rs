pub mod application_window;
pub mod disease_pressure;
pub mod engine;
pub mod fall_fertilization;
pub mod fall_overseeding;
pub mod fertilizer;
pub mod fungicide;
pub mod grub_control;
pub mod heat_stress;
pub mod irrigation_forecast;
pub mod pre_emergent;
pub mod rain_delay;
pub mod spring_nitrogen;

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
