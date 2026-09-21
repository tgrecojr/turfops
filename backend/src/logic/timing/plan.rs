//! Seasonal-plan activities derived from the timing windows, so the plan and calendar
//! show the same pre-emergent and seeding dates as the Timing page. The handler swaps
//! each one in for the plan's own 10 cm activity with the same id.

use super::evaluate::{typical, Season, WindowViews};
use super::log;
use super::windows::{season_year, specs_for, WindowSpec};
use crate::models::seasonal_plan::{ActivityDetails, ActivityStatus, DateWindow, PlannedActivity};
use crate::models::timing::{WindowId, WindowState};
use crate::models::{Application, ApplicationType, GrassType};
use chrono::{Datelike, NaiveDate};

struct ActivityCopy {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    trigger: &'static str,
    products: &'static [&'static str],
    rate: Option<&'static str>,
    notes: &'static str,
}

fn copy_for(id: WindowId) -> Option<ActivityCopy> {
    match id {
        WindowId::SpringPreEmergent => Some(ActivityCopy {
            id: "pre_emergent",
            name: "Spring Pre-Emergent",
            category: "Weed Prevention",
            description: "Get the barrier down before crabgrass germinates. Worthwhile from \
                45°F soil, ideal from 50°F, too late once soil holds 55°F or GDD reaches 200.",
            trigger: "45–55°F (5-day mean at 5 cm), or 200 GDD",
            products: &[
                "Prodiamine (Barricade)",
                "Dithiopyr (Dimension)",
                "Pendimethalin",
            ],
            rate: Some("Label rate"),
            notes: "Water in within 24 hours. Skip wherever you plan to seed this spring.",
        }),
        WindowId::FallPreEmergent => Some(ActivityCopy {
            id: "fall_pre_emergent",
            name: "Fall Pre-Emergent",
            category: "Weed Prevention",
            description: "Stops Poa annua and other winter annuals, which germinate as soil \
                cools through 70°F. Earlier in the window is better.",
            trigger: "70–55°F falling (5-day mean at 5 cm); ideal above 65°F",
            products: &["Prodiamine (Barricade)", "Dithiopyr (Dimension)"],
            rate: Some("Label rate"),
            notes: "Only in a fall without seeding — it stops grass seed too.",
        }),
        WindowId::FallSeeding => Some(ActivityCopy {
            id: "fall_overseeding",
            name: "Fall Seeding & Overseeding",
            category: "Lawn Repair",
            description: "Seed once soil is below 75°F, early enough that seedlings establish \
                before the first freeze. The window ends 30 days before the typical first \
                freeze, or when soil cools to 55°F.",
            trigger: "Below 75°F from Aug 15 until 30 days before the typical first freeze",
            products: &["TTTF blend (3+ cultivars)", "KBG/TTTF mix"],
            rate: Some("4 lbs/1000 sqft (overseeding)"),
            notes: "Aerate first. Keep the seedbed moist until germination. \
                No pre-emergent this fall.",
        }),
        _ => None,
    }
}

fn status(done: bool, start: NaiveDate, end: NaiveDate, today: NaiveDate) -> ActivityStatus {
    if done {
        ActivityStatus::Completed
    } else if today > end {
        ActivityStatus::Missed
    } else if today >= start {
        ActivityStatus::Active
    } else {
        ActivityStatus::Upcoming
    }
}

/// Status for the season being lived right now: what the Timing page says, so the two
/// never disagree (a cold spring keeps pre-emergent Active past its typical close; a warm
/// one ends it early). Other years only have the typical dates to go on.
fn current_status(
    spec: &WindowSpec,
    season: &Season,
    year: i32,
    done: bool,
) -> Option<ActivityStatus> {
    if done || season_year(spec.id, season.today) != year {
        return None;
    }
    let state = WindowViews::build(spec, season, year).weather_state(season.today);
    Some(match state {
        WindowState::NotYet | WindowState::OpeningSoon => ActivityStatus::Upcoming,
        WindowState::Open | WindowState::Ideal | WindowState::Closing => ActivityStatus::Active,
        WindowState::Closed => ActivityStatus::Missed,
        WindowState::Done | WindowState::Blocked => return None,
    })
}

fn activity(
    spec: &WindowSpec,
    copy: &ActivityCopy,
    season: &Season,
    year: i32,
    done: bool,
) -> Option<PlannedActivity> {
    let opens = typical(&spec.opens, season, year)?;
    let closes = typical(&spec.closes, season, year)?;
    Some(PlannedActivity {
        id: copy.id.into(),
        name: copy.name.into(),
        category: copy.category.into(),
        description: copy.description.into(),
        date_window: DateWindow {
            predicted_start: opens.median,
            predicted_end: closes.median,
            earliest_historical: Some(opens.earliest),
            latest_historical: Some(closes.latest),
            confidence: opens.confidence,
        },
        status: current_status(spec, season, year, done)
            .unwrap_or_else(|| status(done, opens.median, closes.median, season.today)),
        details: ActivityDetails {
            soil_temp_trigger: Some(copy.trigger.into()),
            product_suggestions: copy.products.iter().map(|p| p.to_string()).collect(),
            rate: copy.rate.map(Into::into),
            notes: Some(copy.notes.into()),
        },
    })
}

/// Aeration shares the seeding window: cores are pulled right before seed goes down.
fn aeration(seeding: &PlannedActivity, applications: &[Application], year: i32) -> PlannedActivity {
    let done = applications.iter().any(|a| {
        a.application_type == ApplicationType::Aeration
            && a.application_date.year() == year
            && a.application_date.month() >= 7
    });
    let window = &seeding.date_window;
    PlannedActivity {
        id: "core_aeration".into(),
        name: "Core Aeration".into(),
        category: "Lawn Health".into(),
        description: "Relieve compaction and open the soil for seed. Do it at the start of \
            the fall seeding window, while the grass is growing strongly enough to recover."
            .into(),
        status: if done {
            ActivityStatus::Completed
        } else {
            seeding.status.clone()
        },
        date_window: window.clone(),
        details: ActivityDetails {
            soil_temp_trigger: Some("Same window as fall seeding".into()),
            product_suggestions: vec![],
            rate: None,
            notes: Some("Aerate before overseeding. 2-3 inch cores, 2-3 inch spacing.".into()),
        },
    }
}

/// Ids of the plan activities the timing windows answer for on this lawn. The handler
/// removes the plan's own 10 cm version of every one of them — including a window the log
/// has ruled out, which [`activities`] omits and nothing may resurrect.
pub fn owned_ids(grass: GrassType) -> Vec<&'static str> {
    let mut ids = Vec::new();
    for spec in specs_for(grass) {
        if let Some(copy) = copy_for(spec.id) {
            ids.push(copy.id);
            if spec.id == WindowId::FallSeeding {
                ids.push("core_aeration");
            }
        }
    }
    ids
}

/// Plan activities for `year`. A window ruled out by the application log (seed vs.
/// pre-emergent) is left off the plan. Empty when there is no station history.
pub fn activities(
    season: &Season,
    grass: GrassType,
    applications: &[Application],
    year: i32,
) -> Vec<PlannedActivity> {
    let mut out = Vec::new();
    for spec in specs_for(grass) {
        let Some(copy) = copy_for(spec.id) else {
            continue;
        };
        let logged = log::check(spec.id, year, applications);
        if logged.done_on.is_none() && logged.conflict.is_some() {
            continue;
        }
        let Some(planned) = activity(&spec, &copy, season, year, logged.done_on.is_some()) else {
            continue;
        };
        if spec.id == WindowId::FallSeeding {
            out.push(aeration(&planned, applications, year));
        }
        out.push(planned);
    }
    out
}

#[cfg(test)]
mod tests;
