//! Landscape-maintenance logic: turns the cached per-plant maintenance plan into
//! Recommendation entries (when a window is open) and PlannedActivity entries
//! (for calendar / seasonal plan overlays). Pure functions — no IO.

use crate::models::plant::{MaintenanceTask, Plant, TaskType};
use crate::models::seasonal_plan::{
    ActivityDetails, ActivityStatus, DateWindow, PlannedActivity, WindowConfidence,
};
use crate::models::{
    Application, ApplicationType, DataSource, Recommendation, RecommendationCategory,
};
use chrono::{Datelike, NaiveDate};

/// Recommendation lead-in: fire N days before window opens.
const WINDOW_LEAD_DAYS: i64 = 7;

/// Parse "MM-DD" (e.g. "03-15") into a NaiveDate in the given year.
/// Clamps invalid day-of-month (e.g. 02-30 → 02-28).
fn parse_month_day(mmdd: &str, year: i32) -> Option<NaiveDate> {
    let parts: Vec<&str> = mmdd.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Try requested day, fall back to last valid day of month.
    (1..=day)
        .rev()
        .find_map(|d| NaiveDate::from_ymd_opt(year, month, d))
}

/// Resolve a task's MM-DD window to an absolute date range for the target year.
/// If the window wraps year-end (e.g. start=11-15, end=02-15), end is pushed into year+1.
fn resolve_window(task: &MaintenanceTask, year: i32) -> Option<(NaiveDate, NaiveDate)> {
    let start = parse_month_day(&task.window_start_month_day, year)?;
    let end_same_year = parse_month_day(&task.window_end_month_day, year)?;
    let end = if end_same_year < start {
        parse_month_day(&task.window_end_month_day, year + 1)?
    } else {
        end_same_year
    };
    Some((start, end))
}

/// Is there a completion application for this task within `days_back` of `today`?
fn task_completed(
    plant: &Plant,
    task: &MaintenanceTask,
    applications: &[Application],
    today: NaiveDate,
    days_back: i64,
) -> bool {
    let Some(plant_id) = plant.id else {
        return false;
    };
    let allowed_types = task.task_type.matching_application_types();
    if allowed_types.is_empty() {
        return false;
    }
    let earliest = today - chrono::Duration::days(days_back);
    applications.iter().any(|app| {
        app.plant_id == Some(plant_id)
            && allowed_types.contains(&app.application_type)
            && app.application_date >= earliest
            && app.application_date <= today
    })
}

fn task_recommendation_id(plant: &Plant, task_idx: usize, year: i32) -> String {
    format!("plant_{}_{}_{}", plant.id.unwrap_or(0), task_idx, year)
}

/// Emit one Recommendation per plant task when today is inside the window
/// (with a lead-in) and the task hasn't been logged recently.
pub fn generate_plant_maintenance_recommendations(
    plants: &[Plant],
    applications: &[Application],
    today: NaiveDate,
) -> Vec<Recommendation> {
    let mut recs = Vec::new();
    let year = today.year();

    for plant in plants {
        for (idx, task) in plant.maintenance_plan.tasks.iter().enumerate() {
            let Some((start, end)) = resolve_window(task, year) else {
                continue;
            };
            let lead_in = start - chrono::Duration::days(WINDOW_LEAD_DAYS);
            if today < lead_in || today > end {
                continue;
            }
            if task_completed(plant, task, applications, today, 300) {
                continue;
            }

            let title = format!(
                "{} — {} ({})",
                plant.common_name, task.task_type, plant.plant_type,
            );

            let description = task.description.clone();
            let explanation = format!(
                "Recommended window: {} through {}. This is a general homeowner-level \
                 guideline tuned to your USDA zone; adjust if your microclimate runs \
                 warmer or cooler.",
                start.format("%b %-d"),
                end.format("%b %-d")
            );

            let mut rec = Recommendation::new(
                task_recommendation_id(plant, idx, year),
                RecommendationCategory::PlantMaintenance,
                task.severity,
                title,
                description,
            )
            .with_explanation(explanation)
            .with_data_point("Plant", &plant.common_name, DataSource::Manual.as_str())
            .with_data_point(
                "Window",
                format!("{} – {}", start.format("%b %-d"), end.format("%b %-d")),
                DataSource::OpenRouter.as_str(),
            );

            if let Some(sci) = &plant.scientific_name {
                rec = rec.with_data_point("Scientific Name", sci, DataSource::OpenRouter.as_str());
            }
            if let Some(zone) = &task.zone_note {
                rec = rec.with_data_point("Zone Note", zone, DataSource::OpenRouter.as_str());
            }

            rec = rec.with_action(format!(
                "Complete {} on {} within the window, then log it under Applications \
                 (type: {}) linked to this plant so this reminder clears.",
                task.task_type,
                plant.common_name,
                suggested_application_type(task.task_type),
            ));

            recs.push(rec);
        }
    }

    recs
}

fn suggested_application_type(task: TaskType) -> &'static str {
    match task {
        TaskType::Pruning => ApplicationType::Pruning.as_str(),
        TaskType::Fertilizing => ApplicationType::PlantFertilizer.as_str(),
        TaskType::Mulching => ApplicationType::Mulching.as_str(),
        TaskType::Deadheading => ApplicationType::Deadheading.as_str(),
        TaskType::WinterProtection => ApplicationType::WinterProtection.as_str(),
        TaskType::Watering | TaskType::PestInspection | TaskType::Other => {
            ApplicationType::Other.as_str()
        }
    }
}

/// Emit one PlannedActivity per plant task for the requested year,
/// with status derived from logged applications.
pub fn build_plant_activities(
    plants: &[Plant],
    applications: &[Application],
    year: i32,
    today: NaiveDate,
) -> Vec<PlannedActivity> {
    let mut out = Vec::new();

    for plant in plants {
        for (idx, task) in plant.maintenance_plan.tasks.iter().enumerate() {
            let Some((start, end)) = resolve_window(task, year) else {
                continue;
            };

            let completed = task_completed(plant, task, applications, today, 365);
            let status = if completed {
                ActivityStatus::Completed
            } else if today > end {
                ActivityStatus::Missed
            } else if today >= start {
                ActivityStatus::Active
            } else {
                ActivityStatus::Upcoming
            };

            let product_suggestions = match task.task_type {
                TaskType::Fertilizing => {
                    vec!["Balanced slow-release granular (e.g., 10-10-10)".into()]
                }
                TaskType::Mulching => vec!["Shredded hardwood or pine bark mulch, 2-3 in".into()],
                _ => vec![],
            };

            out.push(PlannedActivity {
                id: format!("plant_{}_{}_{}", plant.id.unwrap_or(0), idx, year),
                name: format!("{} — {}", plant.common_name, task.task_type),
                category: "Plant Maintenance".into(),
                description: task.description.clone(),
                date_window: DateWindow {
                    predicted_start: start,
                    predicted_end: end,
                    earliest_historical: None,
                    latest_historical: None,
                    confidence: WindowConfidence::Medium,
                },
                status,
                details: ActivityDetails {
                    soil_temp_trigger: None,
                    product_suggestions,
                    rate: None,
                    notes: task.zone_note.clone(),
                },
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::plant::{
        IdentificationConfidence, MaintenanceTask, PlantMaintenancePlan, PlantType, TaskFrequency,
    };
    use crate::models::Severity;
    use chrono::Utc;

    fn make_task(
        task_type: TaskType,
        start_mmdd: &str,
        end_mmdd: &str,
        severity: Severity,
    ) -> MaintenanceTask {
        MaintenanceTask {
            task_type,
            window_start_month_day: start_mmdd.into(),
            window_end_month_day: end_mmdd.into(),
            frequency: TaskFrequency::Once,
            description: "Test task".into(),
            severity,
            zone_note: None,
        }
    }

    fn make_plant(id: i64, tasks: Vec<MaintenanceTask>) -> Plant {
        Plant {
            id: Some(id),
            lawn_profile_id: 1,
            common_name: "Test Hydrangea".into(),
            scientific_name: Some("Hydrangea paniculata".into()),
            plant_type: PlantType::Shrub,
            location: None,
            planting_date: None,
            notes: None,
            maintenance_plan: PlantMaintenancePlan {
                identified_name: "Test Hydrangea".into(),
                scientific_name: Some("Hydrangea paniculata".into()),
                identification_confidence: IdentificationConfidence::High,
                summary: "Test plan".into(),
                tasks,
                warnings: vec![],
            },
            plan_generated_at: Utc::now(),
            plan_model: "test-model".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn pruning_app(plant_id: i64, date: NaiveDate) -> Application {
        Application {
            id: Some(1),
            lawn_profile_id: 1,
            application_type: ApplicationType::Pruning,
            product_name: None,
            application_date: date,
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            nitrogen_pct: None,
            phosphorus_pct: None,
            potassium_pct: None,
            plant_id: Some(plant_id),
            follow_up_date: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn parse_month_day_clamps_invalid_day() {
        assert_eq!(
            parse_month_day("02-30", 2026),
            Some(NaiveDate::from_ymd_opt(2026, 2, 28).unwrap())
        );
    }

    #[test]
    fn resolve_window_handles_wrap() {
        let task = make_task(TaskType::WinterProtection, "11-15", "02-15", Severity::Info);
        let (start, end) = resolve_window(&task, 2026).unwrap();
        assert_eq!(start.year(), 2026);
        assert_eq!(end.year(), 2027);
    }

    #[test]
    fn recommendation_emitted_in_window() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let recs = generate_plant_maintenance_recommendations(&[plant], &[], today);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].category, RecommendationCategory::PlantMaintenance);
        assert!(recs[0].title.contains("Test Hydrangea"));
    }

    #[test]
    fn recommendation_suppressed_when_completed() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let apps = vec![pruning_app(
            42,
            NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
        )];
        let recs = generate_plant_maintenance_recommendations(&[plant], &apps, today);
        assert!(recs.is_empty(), "Should suppress after pruning was logged");
    }

    #[test]
    fn recommendation_not_emitted_before_lead_in() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        // Well before lead-in window.
        let today = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let recs = generate_plant_maintenance_recommendations(&[plant], &[], today);
        assert!(recs.is_empty());
    }

    #[test]
    fn recommendation_not_emitted_after_end() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        let today = NaiveDate::from_ymd_opt(2026, 4, 15).unwrap();
        let recs = generate_plant_maintenance_recommendations(&[plant], &[], today);
        assert!(recs.is_empty());
    }

    #[test]
    fn build_plant_activities_status_active() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        let today = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
        let acts = build_plant_activities(&[plant], &[], 2026, today);
        assert_eq!(acts.len(), 1);
        assert!(matches!(acts[0].status, ActivityStatus::Active));
        assert_eq!(acts[0].category, "Plant Maintenance");
    }

    #[test]
    fn build_plant_activities_status_completed_with_app() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        let today = NaiveDate::from_ymd_opt(2026, 4, 5).unwrap();
        let apps = vec![pruning_app(
            42,
            NaiveDate::from_ymd_opt(2026, 3, 10).unwrap(),
        )];
        let acts = build_plant_activities(&[plant], &apps, 2026, today);
        assert!(matches!(acts[0].status, ActivityStatus::Completed));
    }

    #[test]
    fn build_plant_activities_status_missed() {
        let task = make_task(TaskType::Pruning, "03-01", "03-31", Severity::Advisory);
        let plant = make_plant(42, vec![task]);
        let today = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let acts = build_plant_activities(&[plant], &[], 2026, today);
        assert!(matches!(acts[0].status, ActivityStatus::Missed));
    }
}
