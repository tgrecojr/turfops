use super::{
    application_window::ApplicationWindowRule, broadleaf_herbicide::BroadleafHerbicideRule,
    fall_fertilization::FallFertilizationRule, fertilizer::FertilizerRule,
    grub_control::GrubControlRule, heat_stress::HeatStressRule,
    irrigation_forecast::IrrigationForecastRule, mowing_height::MowingHeightRule,
    rain_delay::RainDelayRule, spring_nitrogen::SpringNitrogenRule, Rule,
};
use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};

pub struct RulesEngine {
    rules: Vec<Box<dyn Rule>>,
}

impl RulesEngine {
    pub fn new() -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            // Spring rules
            Box::new(SpringNitrogenRule),
            Box::new(BroadleafHerbicideRule),
            // Summer rules
            Box::new(GrubControlRule),
            Box::new(FertilizerRule),
            // Fall rules
            // (Core aeration follows the fall seeding window: logic/timing/companions.rs)
            Box::new(FallFertilizationRule),
            // Forecast-based rules (year-round)
            Box::new(RainDelayRule),
            Box::new(IrrigationForecastRule),
            Box::new(HeatStressRule),
            Box::new(ApplicationWindowRule),
            Box::new(MowingHeightRule),
        ];

        Self { rules }
    }

    pub fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Vec<Recommendation> {
        let mut recommendations: Vec<Recommendation> = self
            .rules
            .iter()
            .filter_map(|rule| rule.evaluate(env, profile, history))
            .collect();
        // No "feed the lawn" next to "avoid fertilizer", and none past the annual budget.
        super::nitrogen_guard::apply(
            &mut recommendations,
            profile,
            history,
            chrono::Local::now().date_naive(),
        );
        recommendations
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}
