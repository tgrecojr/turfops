use super::{
    aeration::AerationRule, application_window::ApplicationWindowRule,
    broadleaf_herbicide::BroadleafHerbicideRule, fall_fertilization::FallFertilizationRule,
    fall_overseeding::FallOverseedingRule, fertilizer::FertilizerRule,
    grub_control::GrubControlRule, heat_stress::HeatStressRule,
    irrigation_forecast::IrrigationForecastRule, mowing_height::MowingHeightRule,
    pre_emergent::PreEmergentRule, rain_delay::RainDelayRule,
    soil_temp_forecast::SoilTempForecastRule, spring_nitrogen::SpringNitrogenRule, Rule,
};
use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};

pub struct RulesEngine {
    rules: Vec<Box<dyn Rule>>,
}

impl RulesEngine {
    pub fn new() -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            // Spring rules
            Box::new(PreEmergentRule),
            Box::new(SpringNitrogenRule),
            Box::new(BroadleafHerbicideRule),
            // Summer rules
            Box::new(GrubControlRule),
            Box::new(FertilizerRule),
            // Fall rules
            Box::new(FallOverseedingRule),
            Box::new(FallFertilizationRule),
            Box::new(AerationRule),
            // Forecast-based rules (year-round)
            Box::new(RainDelayRule),
            Box::new(IrrigationForecastRule),
            Box::new(HeatStressRule),
            Box::new(ApplicationWindowRule),
            Box::new(MowingHeightRule),
            // Proactive forecast-based rules
            Box::new(SoilTempForecastRule),
        ];

        Self { rules }
    }

    pub fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Vec<Recommendation> {
        self.rules
            .iter()
            .filter_map(|rule| rule.evaluate(env, profile, history))
            .collect()
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}
