//! What the application log says about the lawn: days since seeding / feeding, and the
//! fungicides that may still be protecting it.

use crate::models::{
    analyze_fungicide_rotation, classes_of, Application, ApplicationType, DiseaseContext,
    FungicideRecord,
};
use chrono::NaiveDate;

/// Derive the lawn-history modifiers from logged applications (future-dated entries
/// are ignored).
pub fn context_from_history(history: &[Application], today: NaiveDate) -> DiseaseContext {
    // A fungicide or fertilizer logged against a landscape plant says nothing about the lawn.
    let days_since = |kind: ApplicationType| {
        history
            .iter()
            .filter(|app| {
                app.application_type == kind && app.is_turf() && app.application_date <= today
            })
            .map(|app| (today - app.application_date).num_days())
            .min()
    };
    let mut fungicide_apps: Vec<FungicideRecord> = history
        .iter()
        .filter(|app| {
            app.application_type == ApplicationType::Fungicide
                && app.is_turf()
                && app.application_date <= today
        })
        // One record per FRAC class, so a premix protects (and counts toward rotation)
        // for every class in it; an unrecognised product keeps one class-less record.
        .flat_map(|app| {
            let product = app.product_name.clone().unwrap_or_default();
            let date = app.application_date;
            let classes = classes_of(app);
            let records: Vec<FungicideRecord> = if classes.is_empty() {
                vec![FungicideRecord {
                    date,
                    class: None,
                    product,
                }]
            } else {
                classes
                    .into_iter()
                    .map(|class| FungicideRecord {
                        date,
                        class: Some(class),
                        product: product.clone(),
                    })
                    .collect()
            };
            records
        })
        .collect();
    fungicide_apps.sort_by_key(|app| std::cmp::Reverse(app.date));

    DiseaseContext {
        days_since_overseed: days_since(ApplicationType::Overseed),
        days_since_fertilizer: days_since(ApplicationType::Fertilizer),
        fungicide_apps,
        rotation_warning: analyze_fungicide_rotation(history, today).rotation_warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FracClass;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap()
    }

    fn logged(kind: ApplicationType, product: &str, days_ago: i64) -> Application {
        Application {
            id: None,
            lawn_profile_id: 1,
            application_type: kind,
            product_name: Some(product.into()),
            application_date: today() - chrono::Duration::days(days_ago),
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: None,
            follow_up_date: None,
            frac_classes: None,
            product_id: None,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn a_premix_is_recorded_under_every_class_it_contains() {
        let history = [logged(ApplicationType::Fungicide, "Headway G", 5)];
        let ctx = context_from_history(&history, today());
        let classes: Vec<_> = ctx.fungicide_apps.iter().filter_map(|a| a.class).collect();
        assert_eq!(classes, vec![FracClass::Frac11, FracClass::Frac3]);
    }

    #[test]
    fn classes_read_off_the_label_make_an_unknown_product_count() {
        let mut app = logged(ApplicationType::Fungicide, "Store-brand lawn fungicide", 5);
        assert!(
            context_from_history(&[app.clone()], today()).fungicide_apps[0]
                .class
                .is_none()
        );
        app.frac_classes = Some(vec![FracClass::Frac11]);
        let ctx = context_from_history(&[app], today());
        assert_eq!(ctx.fungicide_apps[0].class, Some(FracClass::Frac11));
    }

    #[test]
    fn plant_applications_are_not_lawn_history() {
        let mut shrub_spray = logged(ApplicationType::Fungicide, "Daconil", 2);
        shrub_spray.plant_id = Some(3);
        let mut shrub_feed = logged(ApplicationType::Fertilizer, "Holly-tone", 2);
        shrub_feed.plant_id = Some(3);
        let ctx = context_from_history(&[shrub_spray, shrub_feed], today());
        assert!(ctx.fungicide_apps.is_empty());
        assert!(ctx.days_since_fertilizer.is_none());
    }
}
