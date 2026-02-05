use super::{
    fertilizer::FertilizerRule, fungicide::FungicideRule, grub_control::GrubControlRule,
    pre_emergent::PreEmergentRule, Rule,
};
use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};

pub struct RulesEngine {
    rules: Vec<Box<dyn Rule>>,
}

impl RulesEngine {
    pub fn new() -> Self {
        let rules: Vec<Box<dyn Rule>> = vec![
            Box::new(PreEmergentRule),
            Box::new(GrubControlRule),
            Box::new(FertilizerRule),
            Box::new(FungicideRule),
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
