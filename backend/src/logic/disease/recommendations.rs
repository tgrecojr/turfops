//! Bridges disease risk into the recommendations feed so the dashboard and the
//! Recommendations page agree with the Disease Risk page by construction.

use crate::models::{
    DataSource, Disease, DiseaseContext, DiseaseRisk, ManagementAction, ProductCategory,
    ProductNeed, Recommendation, RecommendationCategory, RiskTier, Severity,
};

/// One recommendation per disease at High or Severe; lower tiers stay on the Disease
/// Risk page only. `ctx` is the lawn history the risks were assessed with; it decides
/// whether red thread's nitrogen remedy is still outstanding.
pub fn to_recommendations(risks: &[DiseaseRisk], ctx: &DiseaseContext) -> Vec<Recommendation> {
    risks
        .iter()
        .filter(|risk| risk.tier >= RiskTier::High)
        .map(|risk| to_recommendation(risk, ctx))
        .collect()
}

fn severity_for(risk: &DiseaseRisk) -> Severity {
    match (risk.management.action, risk.tier) {
        (ManagementAction::Protected, _) => Severity::Info,
        (ManagementAction::Cultural, _) => Severity::Advisory,
        (_, RiskTier::Severe) => Severity::Critical,
        _ => Severity::Warning,
    }
}

fn to_recommendation(risk: &DiseaseRisk, ctx: &DiseaseContext) -> Recommendation {
    let methodology = &risk.methodology;
    let explanation = format!(
        "{} ({}). {}\n\n{}",
        methodology.model_name,
        methodology.citation,
        methodology.summary,
        methodology.steps.join("\n")
    );
    let action = format!(
        "{}\n\nCultural practices:\n- {}",
        risk.management.headline,
        risk.management.cultural.join("\n- ")
    );

    let mut rec = Recommendation::new(
        format!("disease_{}", risk.slug.replace('-', "_")),
        RecommendationCategory::DiseasePressure,
        severity_for(risk),
        format!("{} Risk — {}", risk.name, risk.tier.as_str()),
        &risk.summary,
    )
    .with_explanation(explanation)
    .with_data_point(
        "Risk score",
        &risk.score_label,
        DataSource::Calculated.as_str(),
    );
    for factor in &risk.factors {
        rec = rec.with_data_point(
            &factor.label,
            &factor.value,
            DataSource::Calculated.as_str(),
        );
    }
    rec = rec.with_action(action);
    match need_for(risk, ctx) {
        Some(need) => rec.with_need(need),
        None => rec,
    }
}

/// Red thread's remedy is nitrogen — but only while the lawn is nitrogen-hungry; once
/// fed, the advice is "stay the course" and nothing needs buying. Every other disease
/// wants a fungicide in one of the program's eligible classes (unrestricted, not the
/// class used last).
fn need_for(risk: &DiseaseRisk, ctx: &DiseaseContext) -> Option<ProductNeed> {
    if risk.disease == Disease::RedThread {
        return super::red_thread::nitrogen_hungry(ctx)
            .then(|| ProductNeed::new("nitrogen fertilizer", ProductCategory::Fertilizer));
    }
    let classes: Vec<_> = risk
        .management
        .preventative
        .options
        .iter()
        .filter(|o| !o.restricted && !o.last_used)
        .map(|o| o.frac_class)
        .collect();
    let label = format!(
        "fungicide ({})",
        classes
            .iter()
            .map(|c| c.as_str().split(' ').take(2).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Some(ProductNeed::new(label, ProductCategory::Fungicide).with_frac_classes(classes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};
    use crate::logic::disease::{assess_all, context_from_history};
    use crate::models::{DailyWeather, DiseaseContext};

    fn hot_humid(today: chrono::NaiveDate) -> Vec<DailyWeather> {
        run_of(today, 8, |d| {
            let mut w = day(d, 27.0, 22.0, 33.0, 92.0);
            w.hours_rh90 = 15.0;
            w.leaf_wetness_hours = 15.0;
            w
        })
    }

    #[test]
    fn only_high_and_severe_diseases_are_surfaced() {
        let today = date(7, 20);
        let ctx = DiseaseContext::default();
        let risks = assess_all(&hot_humid(today), today, &ctx);
        let recs = to_recommendations(&risks, &ctx);
        let ids: Vec<&str> = recs.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"disease_brown_patch"));
        assert!(ids.contains(&"disease_pythium_blight"));
        // Too hot for red thread → Low → not surfaced.
        assert!(!ids.contains(&"disease_red_thread"));
        assert!(recs.iter().all(|r| r.severity == Severity::Critical));
    }

    #[test]
    fn nothing_is_surfaced_in_cool_dry_weather() {
        let today = date(11, 20);
        let series = run_of(today, 8, |d| day(d, 5.0, 0.0, 10.0, 55.0));
        let ctx = DiseaseContext::default();
        let risks = assess_all(&series, today, &ctx);
        assert!(to_recommendations(&risks, &ctx).is_empty());
    }

    #[test]
    fn protected_lawn_downgrades_to_info() {
        let today = date(7, 20);
        let ctx = DiseaseContext {
            fungicide_apps: vec![crate::models::FungicideRecord {
                date: date(7, 15),
                product: "Heritage G".into(),
                class: crate::models::frac_class_for_product("Heritage G"),
            }],
            ..context_from_history(&[], today)
        };
        let risks = assess_all(&hot_humid(today), today, &ctx);
        let recs = to_recommendations(&risks, &ctx);
        let brown_patch = recs.iter().find(|r| r.id == "disease_brown_patch").unwrap();
        assert_eq!(brown_patch.severity, Severity::Info);
        assert!(brown_patch
            .suggested_action
            .as_deref()
            .unwrap()
            .starts_with("Protected"));
    }

    /// Cool, wet weather: red thread High on its own.
    fn cool_wet(today: chrono::NaiveDate) -> Vec<DailyWeather> {
        run_of(today, 8, |d| {
            let mut w = day(d, 16.0, 12.0, 20.0, 92.0);
            w.hours_rh90 = 16.0;
            w.leaf_wetness_hours = 16.0;
            w
        })
    }

    fn red_thread_needs(ctx: &DiseaseContext) -> Vec<String> {
        let today = date(10, 5);
        let risks = assess_all(&cool_wet(today), today, ctx);
        let recs = to_recommendations(&risks, ctx);
        let rec = recs
            .iter()
            .find(|r| r.id == "disease_red_thread")
            .expect("red thread surfaced");
        rec.needs.iter().map(|n| n.label.clone()).collect()
    }

    #[test]
    fn red_thread_wants_nitrogen_only_while_the_lawn_is_hungry() {
        let hungry = DiseaseContext {
            days_since_fertilizer: Some(75),
            ..Default::default()
        };
        assert_eq!(red_thread_needs(&hungry), vec!["nitrogen fertilizer"]);
        assert_eq!(
            red_thread_needs(&DiseaseContext::default()),
            vec!["nitrogen fertilizer"]
        );

        let fed = DiseaseContext {
            days_since_fertilizer: Some(3),
            ..Default::default()
        };
        assert!(red_thread_needs(&fed).is_empty());
    }
}
