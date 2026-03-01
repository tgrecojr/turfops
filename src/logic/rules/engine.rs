use super::{
    aeration::AerationRule, application_window::ApplicationWindowRule,
    broadleaf_herbicide::BroadleafHerbicideRule, disease_pressure::DiseasePressureRule,
    fall_fertilization::FallFertilizationRule, fall_overseeding::FallOverseedingRule,
    fertilizer::FertilizerRule, fungicide::FungicideRule, gray_leaf_spot::GrayLeafSpotRule,
    grub_control::GrubControlRule, heat_stress::HeatStressRule,
    irrigation_forecast::IrrigationForecastRule, mowing_height::MowingHeightRule,
    pre_emergent::PreEmergentRule, pythium_blight::PythiumBlightRule, rain_delay::RainDelayRule,
    red_thread::RedThreadRule, spring_nitrogen::SpringNitrogenRule, Rule,
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
            Box::new(FungicideRule),
            // Fall rules
            Box::new(FallOverseedingRule),
            Box::new(FallFertilizationRule),
            Box::new(AerationRule),
            // Disease rules (year-round)
            Box::new(DiseasePressureRule),
            Box::new(GrayLeafSpotRule),
            Box::new(PythiumBlightRule),
            Box::new(RedThreadRule),
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
