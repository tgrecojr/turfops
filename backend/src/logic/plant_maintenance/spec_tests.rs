//! Task windows, completion and the seasonal-plan activities.

use super::tests::{make_plant, make_task, pruning_app};
use super::*;
use crate::models::plant::TaskFrequency;
use crate::models::{ApplicationType, EnvironmentalSummary, Severity};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn recs(plant: &Plant, apps: &[Application], today: NaiveDate) -> Vec<Recommendation> {
    generate_plant_maintenance_recommendations(
        std::slice::from_ref(plant),
        apps,
        &EnvironmentalSummary::default(),
        today,
    )
}

fn pruning(start: &str, end: &str) -> MaintenanceTask {
    make_task(TaskType::Pruning, start, end, Severity::Advisory)
}

// @spec PLANT-WIN-001
#[test]
fn a_month_day_that_does_not_exist_becomes_the_months_last_day() {
    assert_eq!(parse_month_day("03-15", 2026), Some(d(2026, 3, 15)));
    assert_eq!(parse_month_day("02-30", 2028), Some(d(2028, 2, 29)));
    assert_eq!(parse_month_day("02-29", 2027), Some(d(2027, 2, 28)));
    assert_eq!(parse_month_day("02-29", 2028), Some(d(2028, 2, 29)));
    assert_eq!(parse_month_day("04-31", 2026), Some(d(2026, 4, 30)));
}

// @spec PLANT-WIN-002
#[test]
fn a_malformed_window_skips_only_that_task() {
    let plant = make_plant(
        7,
        vec![
            pruning("3/15", "04-15"),
            pruning("13-01", "04-15"),
            pruning("03-01", ""),
            make_task(TaskType::Mulching, "03-01", "03-31", Severity::Info),
        ],
    );
    let today = d(2026, 3, 10);
    let feed = recs(&plant, &[], today);
    assert_eq!(feed.len(), 1);
    assert!(feed[0].title.contains("Mulching"));
    let plan = build_plant_activities(std::slice::from_ref(&plant), &[], 2026, today);
    assert_eq!(plan.len(), 1);
    assert!(plan[0].name.ends_with("Mulching"));
}

// @spec PLANT-WIN-003
#[test]
fn an_end_before_the_start_ends_in_the_next_year() {
    assert_eq!(
        resolve_window(&pruning("11-15", "02-15"), 2026),
        Some((d(2026, 11, 15), d(2027, 2, 15)))
    );
    assert_eq!(
        resolve_window(&pruning("03-01", "03-31"), 2026),
        Some((d(2026, 3, 1), d(2026, 3, 31)))
    );
}

// @spec PLANT-WIN-004
#[test]
fn last_years_window_is_in_force_until_it_ends() {
    let dormant = pruning("12-01", "02-28");
    assert_eq!(
        window_on(&dormant, d(2027, 1, 10)),
        Some((d(2026, 12, 1), d(2027, 2, 28)))
    );
    assert_eq!(
        window_on(&dormant, d(2027, 2, 28)),
        Some((d(2026, 12, 1), d(2027, 2, 28)))
    );
    assert_eq!(
        window_on(&dormant, d(2027, 3, 1)),
        Some((d(2027, 12, 1), d(2028, 2, 28))),
        "after it ends, this year's window"
    );
    assert_eq!(
        window_on(&pruning("03-01", "03-31"), d(2027, 1, 10)),
        Some((d(2027, 3, 1), d(2027, 3, 31)))
    );
}

// @spec PLANT-WIN-005
#[test]
fn completion_counts_a_matching_entry_for_that_plant_from_30_days_before_through_the_end() {
    let task = pruning("04-01", "04-30");
    let plant = make_plant(7, vec![task.clone()]);
    let window = (d(2026, 4, 1), d(2026, 4, 30));
    let done = |app: Application, today: NaiveDate| {
        task_completed(&plant, &task, std::slice::from_ref(&app), today, window)
    };
    let today = d(2026, 5, 10);
    assert!(done(pruning_app(7, d(2026, 3, 2)), today), "30 days early");
    assert!(!done(pruning_app(7, d(2026, 3, 1)), today), "31 days early");
    assert!(done(pruning_app(7, d(2026, 4, 30)), today), "last day");
    assert!(
        !done(pruning_app(7, d(2026, 5, 1)), today),
        "after the window"
    );
    assert!(
        !done(pruning_app(8, d(2026, 4, 10)), today),
        "another plant"
    );
    let mulch = Application {
        application_type: ApplicationType::Mulching,
        ..pruning_app(7, d(2026, 4, 10))
    };
    assert!(!done(mulch, today), "a type that does not complete pruning");
    assert!(
        !done(pruning_app(7, d(2026, 4, 20)), d(2026, 4, 15)),
        "dated after today"
    );
}

// @spec PLANT-WIN-007
#[test]
fn the_task_logic_judges_the_day_its_caller_supplies() {
    let plant = make_plant(7, vec![pruning("03-01", "03-31")]);
    let long_ago = d(2001, 3, 15);
    let feed = recs(&plant, &[], long_ago);
    assert_eq!(feed.len(), 1);
    assert_eq!(feed[0].id, "plant_7_pruning_2001-03-01");
    let plan = build_plant_activities(std::slice::from_ref(&plant), &[], 2001, long_ago);
    assert!(matches!(plan[0].status, ActivityStatus::Active));
}

// @spec PLANT-PLAN-001
#[test]
fn one_activity_per_task_in_the_window_in_force_or_the_years_own() {
    let plant = make_plant(
        7,
        vec![pruning("12-01", "02-28"), pruning("06-01", "06-30")],
    );
    let today = d(2027, 1, 10);
    let current = build_plant_activities(std::slice::from_ref(&plant), &[], 2027, today);
    assert_eq!(current.len(), 2);
    assert_eq!(current[0].date_window.predicted_start, d(2026, 12, 1));
    assert_eq!(current[1].date_window.predicted_start, d(2027, 6, 1));
    let next = build_plant_activities(std::slice::from_ref(&plant), &[], 2028, today);
    assert_eq!(next[0].date_window.predicted_start, d(2028, 12, 1));
    assert_eq!(next[0].date_window.predicted_end, d(2029, 2, 28));
}

// @spec PLANT-PLAN-002
#[test]
fn an_activitys_status_follows_completion_then_the_window_with_no_lead_in() {
    let plant = make_plant(7, vec![pruning("04-01", "04-30")]);
    let status = |apps: &[Application], today: NaiveDate| {
        build_plant_activities(std::slice::from_ref(&plant), apps, 2026, today)[0].status
    };
    assert_eq!(status(&[], d(2026, 3, 31)), ActivityStatus::Upcoming);
    assert_eq!(status(&[], d(2026, 4, 1)), ActivityStatus::Active);
    assert_eq!(status(&[], d(2026, 4, 30)), ActivityStatus::Active);
    assert_eq!(status(&[], d(2026, 5, 1)), ActivityStatus::Missed);
    let done = [pruning_app(7, d(2026, 4, 10))];
    assert_eq!(status(&done, d(2026, 4, 12)), ActivityStatus::Completed);
    assert_eq!(status(&done, d(2026, 6, 1)), ActivityStatus::Completed);
}

// @spec PLANT-PLAN-004
#[test]
fn only_feeding_and_mulching_activities_carry_a_generic_suggestion() {
    let plant = make_plant(
        7,
        vec![
            make_task(TaskType::Fertilizing, "04-01", "04-30", Severity::Info),
            make_task(TaskType::Mulching, "04-01", "04-30", Severity::Info),
            pruning("04-01", "04-30"),
        ],
    );
    let plan = build_plant_activities(&[plant], &[], 2026, d(2026, 4, 2));
    assert_eq!(
        plan[0].details.product_suggestions,
        vec!["Balanced slow-release granular (e.g., 10-10-10)"]
    );
    assert_eq!(
        plan[1].details.product_suggestions,
        vec!["Shredded hardwood or pine bark mulch, 2-3 in"]
    );
    assert!(plan[2].details.product_suggestions.is_empty());
    assert!(plan.iter().all(|a| a.details.rate.is_none()));
}

// @spec PLANT-DATA-005
#[test]
fn one_stored_mm_dd_window_applies_every_year() {
    let task = pruning("03-15", "04-15");
    let stored = serde_json::to_value(&task).unwrap();
    assert_eq!(stored["window_start_month_day"], "03-15");
    assert_eq!(stored["window_end_month_day"], "04-15");
    for year in [2026, 2027, 2030] {
        assert_eq!(
            resolve_window(&task, year),
            Some((d(year, 3, 15), d(year, 4, 15)))
        );
    }
}

// @spec PLANT-DATA-007
#[test]
fn a_tasks_frequency_changes_nothing_but_its_label() {
    let today = d(2026, 4, 10);
    let apps = [pruning_app(7, d(2026, 4, 5))];
    let judged = |frequency: TaskFrequency| {
        let mut task = pruning("04-01", "04-30");
        task.frequency = frequency;
        let plant = make_plant(7, vec![task]);
        let open = recs(&plant, &[], today);
        let after_one = recs(&plant, &apps, today);
        let plan = build_plant_activities(std::slice::from_ref(&plant), &apps, 2026, today);
        (
            open.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
            after_one.len(),
            plan[0].status,
            plan[0].date_window.predicted_start,
            plan[0].date_window.predicted_end,
        )
    };
    let once = judged(TaskFrequency::Once);
    assert_eq!(once.1, 0, "one entry completes the task");
    for f in [
        TaskFrequency::Twice,
        TaskFrequency::Monthly,
        TaskFrequency::AsNeeded,
    ] {
        assert_eq!(judged(f), once, "{f:?}");
    }
}
