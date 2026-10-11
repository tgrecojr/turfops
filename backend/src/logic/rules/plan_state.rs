//! Where each seasonal-plan activity a rule also covers stands this season, judged by
//! the rule's own log and calendar checks plus the assembled feed, so the plan's status
//! for that activity is the feed's answer rather than a second opinion.

use super::thresholds::{SPRING_HERBICIDE_GDD_HIGH, SPRING_N_READY_HIGH_F};
use super::{broadleaf_herbicide, fall_fertilization, grub_control, spring_nitrogen};
use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};
use chrono::{Datelike, NaiveDate};

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod window_tests;

/// One covered activity's current-season state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonState {
    /// The window hasn't opened, or the rule is waiting inside it.
    NotYet,
    /// The feed advises the work now.
    Due,
    /// The feed's feeding for it is past the annual nitrogen target.
    NotNeeded,
    /// The rule's window is over without the work.
    Passed,
    /// The rule's own log check finds the work.
    Done,
    /// An input the rule needs is missing while its window could be open.
    Unknown,
}

/// What the states are judged on: the same date, conditions, log and feed the
/// recommendations came from.
pub struct SeasonInputs<'a> {
    pub today: NaiveDate,
    pub env: &'a EnvironmentalSummary,
    pub profile: &'a LawnProfile,
    pub history: &'a [Application],
    /// The assembled feed (after the nitrogen guard and timing companions), answered
    /// recommendations included.
    pub feed: &'a [Recommendation],
}

/// Each covered plan activity id with its state. Activities whose rule does not apply
/// to the lawn (the cool-season program on a warm-season lawn) are left out.
// @spec RULE-ENG-010
pub fn season_states(inputs: &SeasonInputs) -> Vec<(&'static str, SeasonState)> {
    let mut states = vec![("grub_preventative", grub_preventative(inputs))];
    if inputs.profile.grass_type.is_cool_season() {
        states.push(("spring_nitrogen", spring_nitrogen(inputs)));
        for phase in FALL_PHASES {
            states.push((phase.activity, fall_phase(inputs, &phase)));
        }
        states.push(("spring_herbicide", spring_herbicide(inputs)));
        states.push(("fall_herbicide", fall_herbicide(inputs)));
    }
    states
}

/// What the feed says about the work: Due, Not needed, or nothing.
fn from_feed(feed: &[Recommendation], ids: &[&str]) -> Option<SeasonState> {
    feed.iter().find(|r| ids.contains(&r.id.as_str())).map(|r| {
        if r.feeding.is_some_and(|f| f.target_reached) {
            SeasonState::NotNeeded
        } else {
            SeasonState::Due
        }
    })
}

/// The shared precedence: Done → the feed → Passed → Unknown → Not yet.
fn judge(done: bool, feed: Option<SeasonState>, passed: bool, unknown: bool) -> SeasonState {
    if done {
        SeasonState::Done
    } else if let Some(state) = feed {
        state
    } else if passed {
        SeasonState::Passed
    } else if unknown {
        SeasonState::Unknown
    } else {
        SeasonState::NotYet
    }
}

fn date(today: NaiveDate, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(today.year(), month, day).unwrap_or(today)
}

// @spec RULE-ENG-011
fn spring_nitrogen(i: &SeasonInputs) -> SeasonState {
    let soil = i.env.soil_temp_7day_avg_f;
    let in_window = (date(i.today, 2, 1)..=date(i.today, 4, 30)).contains(&i.today);
    judge(
        !spring_nitrogen::spring_feedings(i.history, i.today).is_empty(),
        from_feed(i.feed, &["spring_n_ready"]),
        i.today.month() >= 5 || soil.is_some_and(|s| s > SPRING_N_READY_HIGH_F),
        in_window && soil.is_none(),
    )
}

struct FallPhase {
    activity: &'static str,
    rec: &'static str,
    number: usize,
    month: u32,
}

const FALL_PHASES: [FallPhase; 3] = [
    FallPhase {
        activity: "early_fall_fert",
        rec: "fall_fert_early",
        number: 1,
        month: 9,
    },
    FallPhase {
        activity: "mid_fall_fert",
        rec: "fall_fert_mid",
        number: 2,
        month: 10,
    },
    FallPhase {
        activity: "winterizer",
        rec: "fall_fert_winterizer",
        number: 3,
        month: 11,
    },
];

// @spec RULE-ENG-012
fn fall_phase(i: &SeasonInputs, phase: &FallPhase) -> SeasonState {
    judge(
        fall_fertilization::fall_feeding_count(i.history, i.today) >= phase.number,
        from_feed(i.feed, &[phase.rec]),
        i.today.month() > phase.month,
        i.today.month() == phase.month && i.env.soil_temp_7day_avg_f.is_none(),
    )
}

// @spec RULE-ENG-013
fn spring_herbicide(i: &SeasonInputs) -> SeasonState {
    let month = i.today.month();
    let gdd_spent = i
        .env
        .gdd_base50_ytd
        .is_none_or(|g| g > SPRING_HERBICIDE_GDD_HIGH);
    judge(
        broadleaf_herbicide::applied_spring(i.history, i.today.year()),
        from_feed(i.feed, &["broadleaf_spring"]),
        month >= 5 || (month == 4 && gdd_spent),
        (3..=4).contains(&month) && i.env.soil_temp_7day_avg_f.is_none(),
    )
}

// @spec RULE-ENG-014
fn fall_herbicide(i: &SeasonInputs) -> SeasonState {
    let window = date(i.today, 9, 20)..=date(i.today, 10, 31);
    judge(
        broadleaf_herbicide::applied_fall(i.history, i.today.year()),
        from_feed(i.feed, &["broadleaf_fall"]),
        i.today > *window.end(),
        window.contains(&i.today) && i.env.soil_temp_7day_avg_f.is_none(),
    )
}

// @spec RULE-ENG-015
fn grub_preventative(i: &SeasonInputs) -> SeasonState {
    let year = i.today.year();
    let open = grub_control::window_open(i.today, i.env.gdd_base50_ytd);
    let live = i.env.current.as_ref().and_then(|c| c.soil_temp_10_f);
    judge(
        grub_control::applied_this_season(i.history, i.today),
        from_feed(
            i.feed,
            &[
                &format!("grub_control_{year}"),
                &format!("grub_control_late_{year}"),
            ],
        ),
        i.today > date(i.today, 7, 4) && !open,
        open && (i.env.soil_temp_7day_avg_f.is_none() || live.is_none()),
    )
}
