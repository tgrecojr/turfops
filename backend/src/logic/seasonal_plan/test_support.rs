//! Shared fixtures for the activity builder's tests: three settled years of crossings and
//! a 2026 plan judged on Jan 15.

use super::*;
pub(super) use crate::models::{ApplicationType, GrassType};

pub(super) fn d(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

pub(super) const SETTLED: [i32; 3] = [2023, 2024, 2025];
pub(super) const PLAN_YEAR: i32 = 2026;

/// Every threshold on a base date in 2023, two days later each following year, so the
/// median is the 2024 date and earliest / latest are 2023 / 2025.
pub(super) fn history(skip: &[&str]) -> Vec<ThresholdCrossing> {
    [
        ("soil_50f_rising", 4, 4),
        ("soil_55f_rising", 4, 21),
        ("soil_60f_rising", 5, 10),
        ("soil_65f_rising", 5, 25),
        ("soil_75f_rising", 6, 25),
        ("soil_65f_falling", 9, 23),
        ("soil_55f_falling", 10, 25),
        ("soil_50f_falling", 11, 5),
        ("soil_45f_falling", 11, 20),
    ]
    .iter()
    .filter(|(name, _, _)| !skip.contains(name))
    .flat_map(|&(name, month, day)| {
        SETTLED
            .iter()
            .enumerate()
            .map(move |(i, &year)| ThresholdCrossing {
                year,
                threshold_name: name.to_string(),
                crossing_date: d(year, month, day) + chrono::Duration::days(2 * i as i64),
                avg_soil_temp_f: 0.0,
            })
    })
    .collect()
}

pub(super) fn plan_with(
    crossings: &[ThresholdCrossing],
    applications: &[Application],
) -> SeasonalPlan {
    build_seasonal_plan(
        PLAN_YEAR,
        crossings,
        &SETTLED,
        applications,
        GrassType::TallFescue,
        d(PLAN_YEAR, 1, 15),
    )
}

pub(super) fn plan(applications: &[Application]) -> SeasonalPlan {
    plan_with(&history(&[]), applications)
}

pub(super) fn activity<'a>(plan: &'a SeasonalPlan, id: &str) -> &'a PlannedActivity {
    plan.activities
        .iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("no {id}"))
}

pub(super) fn completed(plan: &SeasonalPlan, id: &str) -> bool {
    matches!(activity(plan, id).status, ActivityStatus::Completed)
}

pub(super) fn window(plan: &SeasonalPlan, id: &str) -> (NaiveDate, NaiveDate) {
    let w = &activity(plan, id).date_window;
    (w.predicted_start, w.predicted_end)
}

pub(super) fn range(plan: &SeasonalPlan, id: &str) -> (Option<NaiveDate>, Option<NaiveDate>) {
    let w = &activity(plan, id).date_window;
    (w.earliest_historical, w.latest_historical)
}

pub(super) fn applied(kind: ApplicationType, on: NaiveDate) -> Application {
    Application {
        id: None,
        lawn_profile_id: 1,
        application_type: kind,
        product_name: None,
        application_date: on,
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

pub(super) fn on_a_plant(mut app: Application) -> Application {
    app.plant_id = Some(7);
    app
}

pub(super) fn analysis(mut app: Application, n: f64) -> Application {
    app.nitrogen_pct = Some(n);
    app
}
