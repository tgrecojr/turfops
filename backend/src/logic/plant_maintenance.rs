//! Landscape-maintenance logic: turns the cached per-plant maintenance plan into
//! Recommendation entries (when a window is open) and PlannedActivity entries
//! (for calendar / seasonal plan overlays). Pure functions — no IO.

use crate::models::plant::{MaintenanceTask, Plant, TaskType};
use crate::models::seasonal_plan::{
    ActivityDetails, ActivityStatus, DateWindow, PlannedActivity, WindowConfidence,
};
use crate::models::{
    Application, ApplicationType, DataSource, ProductCategory, ProductNeed, Recommendation,
    RecommendationCategory,
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

/// The task's window as it matters on `today`: a window that started last year and is still
/// open (dormant pruning, Dec 1 → Feb 28, seen from January) wins over this year's, which
/// would otherwise make the task vanish on Jan 1 with two months of its window left.
fn window_on(task: &MaintenanceTask, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    resolve_window(task, today.year() - 1)
        .filter(|(_, end)| today <= *end)
        .or_else(|| resolve_window(task, today.year()))
}

/// Doing the job a little ahead of its window still counts for that window.
const EARLY_COMPLETION_DAYS: i64 = 30;

/// Was this task logged for *this* window? Scoped to the window rather than "sometime in
/// the last year", so April's feeding doesn't complete September's.
fn task_completed(
    plant: &Plant,
    task: &MaintenanceTask,
    applications: &[Application],
    today: NaiveDate,
    window: (NaiveDate, NaiveDate),
) -> bool {
    let Some(plant_id) = plant.id else {
        return false;
    };
    let allowed_types = task.task_type.matching_application_types();
    if allowed_types.is_empty() {
        return false;
    }
    let earliest = window.0 - chrono::Duration::days(EARLY_COMPLETION_DAYS);
    applications.iter().any(|app| {
        app.plant_id == Some(plant_id)
            && allowed_types.contains(&app.application_type)
            && app.application_date >= earliest
            && app.application_date <= window.1
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

    for plant in plants {
        for (idx, task) in plant.maintenance_plan.tasks.iter().enumerate() {
            let Some((start, end)) = window_on(task, today) else {
                continue;
            };
            // The id carries the year the window opened in, so it is stable across Jan 1.
            let year = start.year();
            let lead_in = start - chrono::Duration::days(WINDOW_LEAD_DAYS);
            if today < lead_in || today > end {
                continue;
            }
            if task_completed(plant, task, applications, today, (start, end)) {
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

            if task.task_type == TaskType::Fertilizing {
                rec = rec.with_need(
                    ProductNeed::new("plant fertilizer", ProductCategory::Fertilizer)
                        .or_category(ProductCategory::Supplement),
                );
            }

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
            // For the current year show the window as it stands today (possibly one that
            // opened last December); other years show the window that opens in them.
            let window = if year == today.year() {
                window_on(task, today)
            } else {
                resolve_window(task, year)
            };
            let Some((start, end)) = window else {
                continue;
            };

            let completed = task_completed(plant, task, applications, today, (start, end));
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
mod tests;
