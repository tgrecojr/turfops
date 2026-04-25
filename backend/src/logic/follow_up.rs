//! Follow-up recommendations: when an application sets follow_up_date, surface a
//! Recommendation as that date approaches/passes — until a subsequent application
//! of the same type (and same plant target, if any) is logged.

use crate::models::plant::Plant;
use crate::models::{Application, DataSource, Recommendation, RecommendationCategory, Severity};
use chrono::NaiveDate;
use std::collections::HashMap;

const LEAD_DAYS: i64 = 7;
const STALE_DAYS: i64 = 90;

/// True if a later application of the same type targeting the same plant (or same
/// turf-scope) has been logged after `app.application_date` — i.e. the follow-up
/// has been "done again."
fn follow_up_completed(app: &Application, applications: &[Application]) -> bool {
    applications.iter().any(|other| {
        other.id != app.id
            && other.application_type == app.application_type
            && other.plant_id == app.plant_id
            && other.application_date > app.application_date
    })
}

fn severity_for(today: NaiveDate, follow_up: NaiveDate) -> Severity {
    if today >= follow_up {
        Severity::Warning
    } else {
        Severity::Advisory
    }
}

pub fn generate_follow_up_recommendations(
    applications: &[Application],
    plants_by_id: &HashMap<i64, &Plant>,
    today: NaiveDate,
) -> Vec<Recommendation> {
    let mut out = Vec::new();

    for app in applications {
        let Some(follow_up) = app.follow_up_date else {
            continue;
        };
        let Some(app_id) = app.id else {
            continue;
        };

        let lead_in = follow_up - chrono::Duration::days(LEAD_DAYS);
        let stale_after = follow_up + chrono::Duration::days(STALE_DAYS);
        if today < lead_in || today > stale_after {
            continue;
        }
        if follow_up_completed(app, applications) {
            continue;
        }

        let target_label: String = match app.plant_id.and_then(|pid| plants_by_id.get(&pid)) {
            Some(plant) => plant.common_name.clone(),
            None => "Lawn".into(),
        };

        let product = app.product_name.as_deref().unwrap_or("application");
        let title = format!(
            "Follow-up: {} on {} ({})",
            app.application_type, target_label, product
        );

        let days_diff = (follow_up - today).num_days();
        let timing_blurb = if days_diff > 0 {
            format!("scheduled in {} day(s)", days_diff)
        } else if days_diff == 0 {
            "scheduled for today".to_string()
        } else {
            format!("overdue by {} day(s)", -days_diff)
        };

        let description = format!(
            "Follow-up {} for {} on {}. Originally applied on {}.",
            timing_blurb,
            app.application_type,
            target_label,
            app.application_date.format("%b %-d, %Y"),
        );

        let mut rec = Recommendation::new(
            format!("application_followup_{}", app_id),
            RecommendationCategory::ApplicationTiming,
            severity_for(today, follow_up),
            title,
            description,
        )
        .with_explanation(
            "You scheduled this follow-up when logging the original application. It will \
             clear once you log a subsequent application of the same type for the same target."
                .to_string(),
        )
        .with_data_point("Target", &target_label, DataSource::Manual.as_str())
        .with_data_point(
            "Original",
            app.application_date.format("%b %-d, %Y").to_string(),
            DataSource::Manual.as_str(),
        )
        .with_data_point(
            "Follow-up",
            follow_up.format("%b %-d, %Y").to_string(),
            DataSource::Manual.as_str(),
        );

        if let Some(name) = &app.product_name {
            rec = rec.with_data_point("Product", name, DataSource::Manual.as_str());
        }

        rec = rec.with_action(format!(
            "Re-apply {} to {} and log it under Applications so this reminder clears.",
            app.application_type, target_label,
        ));

        out.push(rec);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ApplicationType;
    use chrono::Utc;

    fn app(
        id: i64,
        kind: ApplicationType,
        date: NaiveDate,
        plant_id: Option<i64>,
        follow_up: Option<NaiveDate>,
    ) -> Application {
        Application {
            id: Some(id),
            lawn_profile_id: 1,
            application_type: kind,
            product_name: Some("TestProduct".into()),
            application_date: date,
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id,
            follow_up_date: follow_up,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn emits_when_inside_lead_in_window() {
        let original = app(
            1,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
            Some(42),
            Some(NaiveDate::from_ymd_opt(2026, 5, 8).unwrap()),
        );
        let today = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();
        let recs = generate_follow_up_recommendations(&[original], &HashMap::new(), today);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].severity, Severity::Advisory);
    }

    #[test]
    fn warning_severity_when_overdue() {
        let original = app(
            1,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
            None,
            Some(NaiveDate::from_ymd_opt(2026, 5, 8).unwrap()),
        );
        let today = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let recs = generate_follow_up_recommendations(&[original], &HashMap::new(), today);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].severity, Severity::Warning);
    }

    #[test]
    fn suppressed_when_subsequent_application_logged() {
        let original = app(
            1,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
            Some(42),
            Some(NaiveDate::from_ymd_opt(2026, 5, 8).unwrap()),
        );
        let later = app(
            2,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 5, 9).unwrap(),
            Some(42),
            None,
        );
        let today = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let recs = generate_follow_up_recommendations(&[original, later], &HashMap::new(), today);
        assert!(recs.is_empty());
    }

    #[test]
    fn different_plant_does_not_clear_followup() {
        let original = app(
            1,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
            Some(42),
            Some(NaiveDate::from_ymd_opt(2026, 5, 8).unwrap()),
        );
        let other_plant = app(
            2,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 5, 9).unwrap(),
            Some(99),
            None,
        );
        let today = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let recs =
            generate_follow_up_recommendations(&[original, other_plant], &HashMap::new(), today);
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn dropped_after_stale_window() {
        let original = app(
            1,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            None,
            Some(NaiveDate::from_ymd_opt(2026, 1, 8).unwrap()),
        );
        let today = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let recs = generate_follow_up_recommendations(&[original], &HashMap::new(), today);
        assert!(recs.is_empty());
    }

    #[test]
    fn skipped_when_no_follow_up_set() {
        let original = app(
            1,
            ApplicationType::Insecticide,
            NaiveDate::from_ymd_opt(2026, 4, 24).unwrap(),
            None,
            None,
        );
        let today = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let recs = generate_follow_up_recommendations(&[original], &HashMap::new(), today);
        assert!(recs.is_empty());
    }
}
