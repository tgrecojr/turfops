use super::{
    application_window::ApplicationWindowRule, disease_pressure::DiseasePressureRule,
    fall_fertilization::FallFertilizationRule, fall_overseeding::FallOverseedingRule,
    fertilizer::FertilizerRule, fungicide::FungicideRule, grub_control::GrubControlRule,
    heat_stress::HeatStressRule, irrigation_forecast::IrrigationForecastRule,
    pre_emergent::PreEmergentRule, rain_delay::RainDelayRule, spring_nitrogen::SpringNitrogenRule,
    Rule,
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
            // Summer rules
            Box::new(GrubControlRule),
            Box::new(FertilizerRule),
            Box::new(FungicideRule),
            // Fall rules
            Box::new(FallOverseedingRule),
            Box::new(FallFertilizationRule),
            // Forecast-based rules (year-round)
            Box::new(RainDelayRule),
            Box::new(IrrigationForecastRule),
            Box::new(HeatStressRule),
            Box::new(ApplicationWindowRule),
            Box::new(DiseasePressureRule),
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
