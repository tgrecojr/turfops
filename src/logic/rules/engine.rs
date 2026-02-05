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

    pub fn evaluate_rule(
        &self,
        rule_id: &str,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        self.rules
            .iter()
            .find(|r| r.id() == rule_id)
            .and_then(|rule| rule.evaluate(env, profile, history))
    }

    pub fn list_rules(&self) -> Vec<(&'static str, &'static str)> {
        self.rules.iter().map(|r| (r.id(), r.name())).collect()
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}
