use crate::logic::rules::plan_state::SeasonState;
use crate::models::seasonal_plan::*;
use crate::models::{Application, ApplicationType, GrassType};
use chrono::{Datelike, NaiveDate};
use std::collections::HashSet;
use std::ops::RangeInclusive;

#[cfg(test)]
mod completion_tests;
#[cfg(test)]
mod crossing_tests;
mod crossings;
mod fall;
mod spring;
#[cfg(test)]
mod status_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod window_tests;
pub use crossings::analyze_year;
use crossings::{aggregate_crossings, voting_years, AggregatedWindow};
#[cfg(test)]
use crossings::{find_threshold_crossings, year_settled};

/// Activities that only make sense on cool-season turf: fall seeding and aeration, and the
/// spring / fall / winterizer feeding program. A warm-season lawn is fed in summer and goes
/// dormant in fall; showing it this program put "Fall Overseeding — TTTF blend" on a Bermuda
/// lawn's calendar next to pages that correctly refuse cool-season advice. Weed and grub
/// control stay.
const COOL_SEASON_ONLY: [&str; 6] = [
    "spring_nitrogen",
    "core_aeration",
    "fall_overseeding",
    "early_fall_fert",
    "mid_fall_fert",
    "winterizer",
];

/// Build the full seasonal plan from aggregated threshold data and application history.
/// Only settled years (PLAN-CACHE-005) with cached crossings vote: the 10 most recent before
/// the plan year. Statuses are judged on `today`, the lawn's local date.
// @spec PLAN-CROSS-010
pub fn build_seasonal_plan(
    year: i32,
    crossings: &[ThresholdCrossing],
    settled_years: &[i32],
    applications: &[Application],
    grass: GrassType,
    today: NaiveDate,
) -> SeasonalPlan {
    let with_crossings: HashSet<i32> = crossings.iter().map(|c| c.year).collect();
    let eligible: Vec<i32> = settled_years
        .iter()
        .copied()
        .filter(|y| with_crossings.contains(y))
        .collect();
    let voters = voting_years(&eligible, year);
    let voting: Vec<ThresholdCrossing> = crossings
        .iter()
        .filter(|c| voters.contains(&c.year))
        .cloned()
        .collect();
    let mut activities = build_activities(year, &voting, applications, today);
    if !grass.is_cool_season() {
        activities.retain(|a| !COOL_SEASON_ONLY.contains(&a.id.as_str()));
    }

    SeasonalPlan {
        year,
        activities,
        data_years_used: voters.len() as i32,
        generated_at: chrono::Utc::now(),
        today,
    }
}

/// Give each rule-covered activity of the current season its status from the rules'
/// state (`None`: the feed could not be assembled, so every covered activity keeps its
/// typical-date status as an estimate).
// @spec PLAN-ACT-021, PLAN-ACT-022
pub fn apply_rule_states(
    activities: &mut [PlannedActivity],
    states: Option<&[(&str, SeasonState)]>,
) {
    for activity in activities.iter_mut() {
        let Some(states) = states else {
            activity.status_estimated |= RULE_COVERED.contains(&activity.id.as_str());
            continue;
        };
        let Some((_, state)) = states.iter().find(|(id, _)| *id == activity.id) else {
            continue;
        };
        match state {
            SeasonState::Done => activity.status = ActivityStatus::Completed,
            SeasonState::Due => activity.status = ActivityStatus::Active,
            SeasonState::NotNeeded => activity.status = ActivityStatus::Skipped,
            SeasonState::Passed => activity.status = ActivityStatus::Missed,
            SeasonState::NotYet => activity.status = ActivityStatus::Upcoming,
            SeasonState::Unknown => activity.status_estimated = true,
        }
    }
}

/// The plan activities an agronomic rule also covers (RULE-ENG-010).
const RULE_COVERED: [&str; 7] = [
    "spring_nitrogen",
    "spring_herbicide",
    "grub_preventative",
    "early_fall_fert",
    "mid_fall_fert",
    "winterizer",
    "fall_herbicide",
];

fn build_activities(
    year: i32,
    crossings: &[ThresholdCrossing],
    applications: &[Application],
    today: NaiveDate,
) -> Vec<PlannedActivity> {
    let mut activities = spring::spring_activities(year, crossings, applications, today);
    activities.extend(fall::fall_activities(year, crossings, applications, today));
    // Sort by predicted start date
    activities.sort_by_key(|a| a.date_window.predicted_start);

    activities
}

fn make_window(
    start: &AggregatedWindow,
    end_date: NaiveDate,
    end_agg: Option<&AggregatedWindow>,
) -> DateWindow {
    DateWindow {
        predicted_start: start.median,
        predicted_end: end_date,
        earliest_historical: Some(start.earliest),
        latest_historical: end_agg.map(|e| e.latest),
        confidence: confidence_from_count(start.sample_count),
    }
}

fn make_window_with_dates(
    start_date: NaiveDate,
    end_date: NaiveDate,
    start_agg: &AggregatedWindow,
    end_agg: Option<&AggregatedWindow>,
) -> DateWindow {
    DateWindow {
        predicted_start: start_date,
        predicted_end: end_date,
        earliest_historical: Some(start_agg.earliest),
        latest_historical: end_agg.map(|e| e.latest),
        confidence: confidence_from_count(start_agg.sample_count),
    }
}

fn confidence_from_count(count: usize) -> WindowConfidence {
    if count >= 5 {
        WindowConfidence::High
    } else if count >= 3 {
        WindowConfidence::Medium
    } else {
        WindowConfidence::Low
    }
}

fn compute_status(
    completed: bool,
    start: NaiveDate,
    end: NaiveDate,
    today: NaiveDate,
) -> ActivityStatus {
    if completed {
        ActivityStatus::Completed
    } else if today > end {
        ActivityStatus::Missed
    } else if today >= start {
        ActivityStatus::Active
    } else {
        ActivityStatus::Upcoming
    }
}

/// The same month and day in `year`; February 29 becomes February 28 in a common year.
// @spec PLAN-ACT-017
fn in_year(date: NaiveDate, year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, date.month(), date.day())
        .or_else(|| NaiveDate::from_ymd_opt(year, 2, 28))
        .unwrap_or(date)
}

/// Whether a lawn application of one of `kinds` is dated in `months` of the plan year.
/// Landscape-plant applications never complete a lawn activity.
// @spec PLAN-ACT-020
fn logged(
    applications: &[Application],
    kinds: &[ApplicationType],
    year: i32,
    months: RangeInclusive<u32>,
) -> bool {
    applications.iter().any(|a| {
        a.is_turf()
            && kinds.contains(&a.application_type)
            && a.application_date.year() == year
            && months.contains(&a.application_date.month())
    })
}

/// Whether a lawn feeding (`Application::is_feeding`) is dated in `months` of the plan year.
// @spec PLAN-ACT-004, PLAN-ACT-010
fn fed(applications: &[Application], year: i32, months: RangeInclusive<u32>) -> bool {
    applications.iter().any(|a| {
        a.is_feeding()
            && a.application_date.year() == year
            && months.contains(&a.application_date.month())
    })
}
