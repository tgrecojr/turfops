use super::super::tests::{applied, date, station, within};
use super::super::SeasonData;
use super::*;

fn plan(history: &[Application]) -> Vec<PlannedActivity> {
    let today = date(2026, 9, 20);
    let days = station(date(2026, 9, 19), 0.0);
    let data = SeasonData::build(&days, &[], today.year());
    activities(
        &data.season(&days, today),
        GrassType::TallFescue,
        history,
        2026,
    )
}

fn find<'a>(plan: &'a [PlannedActivity], id: &str) -> Option<&'a PlannedActivity> {
    plan.iter().find(|a| a.id == id)
}

#[test]
fn plan_uses_the_timing_windows_for_seeding_aeration_and_pre_emergent() {
    let plan = plan(&[]);
    let seeding = find(&plan, "fall_overseeding").unwrap();
    assert_eq!(seeding.date_window.predicted_start, date(2026, 8, 15));
    assert!(within(
        Some(seeding.date_window.predicted_end),
        date(2026, 10, 6),
        2
    ));
    assert!(matches!(seeding.status, ActivityStatus::Active));

    let aeration = find(&plan, "core_aeration").unwrap();
    assert_eq!(
        aeration.date_window.predicted_start,
        seeding.date_window.predicted_start
    );

    let spring = find(&plan, "pre_emergent").unwrap();
    assert!(within(
        Some(spring.date_window.predicted_start),
        date(2026, 3, 24),
        2
    ));
    assert!(matches!(spring.status, ActivityStatus::Missed));
    assert!(find(&plan, "fall_pre_emergent").is_some());
}

#[test]
fn overseeding_drops_fall_pre_emergent_from_the_plan() {
    let plan = plan(&[applied(ApplicationType::Overseed, date(2026, 9, 12))]);
    assert!(find(&plan, "fall_pre_emergent").is_none());
    let seeding = find(&plan, "fall_overseeding").unwrap();
    assert!(matches!(seeding.status, ActivityStatus::Completed));
}

#[test]
fn fall_pre_emergent_does_not_complete_the_spring_application() {
    let plan = plan(&[applied(ApplicationType::PreEmergent, date(2026, 9, 5))]);
    let spring = find(&plan, "pre_emergent").unwrap();
    assert!(matches!(spring.status, ActivityStatus::Missed));
    let fall = find(&plan, "fall_pre_emergent").unwrap();
    assert!(matches!(fall.status, ActivityStatus::Completed));
    // Seeding is ruled out, and aeration goes with it.
    assert!(find(&plan, "fall_overseeding").is_none());
    assert!(find(&plan, "core_aeration").is_none());
}

#[test]
fn warm_season_lawns_only_get_the_pre_emergent_activities() {
    let today = date(2026, 9, 20);
    let days = station(date(2026, 9, 19), 0.0);
    let data = SeasonData::build(&days, &[], today.year());
    let plan = activities(&data.season(&days, today), GrassType::Bermuda, &[], 2026);
    let ids: Vec<&str> = plan.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, vec!["pre_emergent", "fall_pre_emergent"]);
}

fn plan_on(
    today: NaiveDate,
    days: &[crate::datasources::weather::ClimateDay],
) -> Vec<PlannedActivity> {
    let data = SeasonData::build(days, &[], today.year());
    activities(
        &data.season(days, today),
        GrassType::TallFescue,
        &[],
        today.year(),
    )
}

#[test]
fn this_seasons_status_follows_the_window_not_the_typical_dates() {
    // A spring running 6°F cold: on Apr 28 (typical close ≈ Apr 21) the soil has not
    // reached 55°F, so the Timing page says apply now — and so must the plan.
    let today = date(2026, 4, 28);
    let cold = plan_on(today, &station(date(2026, 4, 27), -6.0));
    assert!(matches!(
        find(&cold, "pre_emergent").unwrap().status,
        ActivityStatus::Active
    ));

    // A spring running 6°F warm closes it well before the typical date.
    let today = date(2026, 4, 14);
    let warm = plan_on(today, &station(date(2026, 4, 13), 6.0));
    assert!(matches!(
        find(&warm, "pre_emergent").unwrap().status,
        ActivityStatus::Missed
    ));
}

#[test]
fn the_windows_answer_for_their_activities_even_when_one_is_blocked() {
    let owned = owned_ids(GrassType::TallFescue);
    for id in [
        "pre_emergent",
        "fall_pre_emergent",
        "fall_overseeding",
        "core_aeration",
    ] {
        assert!(owned.contains(&id), "{id}");
    }
    // A logged fall pre-emergent drops seeding + aeration from the timing list; the
    // handler removes the plan's own copies by `owned_ids`, so they cannot come back.
    let blocked = plan(&[applied(ApplicationType::PreEmergent, date(2026, 9, 5))]);
    assert!(find(&blocked, "fall_overseeding").is_none());
    assert!(owned.contains(&"fall_overseeding"));
}
