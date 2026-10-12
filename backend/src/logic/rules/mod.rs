pub mod application_window;
pub mod broadleaf_herbicide;
pub mod engine;
pub mod fall_fertilization;
pub mod fertilizer;
pub mod grub_control;
pub mod heat_stress;
pub mod irrigation_forecast;
pub mod mowing_height;
pub mod nitrogen_guard;
pub mod plan_state;
pub mod rain_delay;
pub mod spring_nitrogen;
pub mod thresholds;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod forecast_rule_tests;
#[cfg(test)]
pub(crate) mod test_fixtures;
#[cfg(test)]
mod weather_rule_tests;

pub use engine::RulesEngine;

use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};

// @spec RULE-ENG-001, RULE-ENG-003
/// Trait for agronomic rules
pub trait Rule: Send + Sync {
    /// Evaluate the rule and return a recommendation if conditions are met
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation>;
}
