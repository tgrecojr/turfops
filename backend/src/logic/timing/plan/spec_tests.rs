//! The plan activities the windows build: their dates, statuses and omissions.

use super::super::recommendations::to_recommendations;
use super::super::tests::{applied, date, station};
use super::super::{assess, Inputs, SeasonData};
use super::*;
use crate::logic::rules::thresholds::OVERSEED_RATE_LBS_PER_KSQFT;
use crate::models::timing::SoilNow;
use crate::models::LawnProfile;

fn plan_on(today: NaiveDate, history: &[Application]) -> Vec<PlannedActivity> {
    let days = station(today.pred_opt().unwrap(), 0.0);
    let data = SeasonData::build(&days, &[], today.year());
    activities(
        &data.season(&days, today),
        GrassType::TallFescue,
        history,
        today.year(),
    )
}

// @spec TIME-PLAN-001
#[test]
fn plan_activities_span_each_windows_typical_open_to_close() {
    let today = date(2026, 9, 20);
    let days = station(date(2026, 9, 19), 0.0);
    let data = SeasonData::build(&days, &[], 2026);
    let season = data.season(&days, today);
    let plan = activities(&season, GrassType::TallFescue, &[], 2026);

    let ids: Vec<&str> = plan.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "pre_emergent",
            "fall_pre_emergent",
            "core_aeration",
            "fall_overseeding"
        ]
    );
    let specs = specs_for(GrassType::TallFescue);
    for (activity_id, window) in [
        ("pre_emergent", WindowId::SpringPreEmergent),
        ("fall_pre_emergent", WindowId::FallPreEmergent),
        ("fall_overseeding", WindowId::FallSeeding),
        ("core_aeration", WindowId::FallSeeding),
    ] {
        let spec = specs.iter().find(|s| s.id == window).unwrap();
        let opens = typical(&spec.opens, &season, 2026).unwrap();
        let closes = typical(&spec.closes, &season, 2026).unwrap();
        let a = plan.iter().find(|a| a.id == activity_id).unwrap();
        let w = &a.date_window;
        assert_eq!(w.predicted_start, opens.median, "{activity_id}");
        assert_eq!(w.predicted_end, closes.median, "{activity_id}");
        assert_eq!(w.earliest_historical, Some(opens.earliest), "{activity_id}");
        assert_eq!(w.latest_historical, Some(closes.latest), "{activity_id}");
        assert_eq!(w.confidence, opens.confidence, "{activity_id}");
    }
}

fn expected_status(state: WindowState) -> Option<ActivityStatus> {
    match state {
        WindowState::NotYet | WindowState::OpeningSoon => Some(ActivityStatus::Upcoming),
        WindowState::Open | WindowState::Ideal | WindowState::Closing => {
            Some(ActivityStatus::Active)
        }
        WindowState::Closed => Some(ActivityStatus::Missed),
        WindowState::Done => Some(ActivityStatus::Completed),
        WindowState::Blocked => None,
    }
}

// @spec TIME-PLAN-002
#[test]
fn this_seasons_activity_status_is_the_windows_state() {
    let activity_for = |id| match id {
        WindowId::SpringPreEmergent => Some("pre_emergent"),
        WindowId::FallPreEmergent => Some("fall_pre_emergent"),
        WindowId::FallSeeding => Some("fall_overseeding"),
        _ => None,
    };
    let fall_pre_em = [applied(ApplicationType::PreEmergent, date(2026, 9, 5))];
    let mut seen = Vec::new();
    for (m, d, history) in [
        (3, 1, &[][..]),
        (3, 20, &[][..]),
        (3, 30, &[][..]),
        (4, 12, &[][..]),
        (5, 1, &[][..]),
        (8, 10, &[][..]),
        (9, 20, &[][..]),
        (9, 28, &[][..]),
        (9, 20, &fall_pre_em[..]),
    ] {
        let today = date(2026, m, d);
        let days = station(today.pred_opt().unwrap(), 0.0);
        let windows = assess(&Inputs {
            today,
            days: &days,
            forecast: &[],
            grass: GrassType::TallFescue,
            history,
        })
        .windows;
        let plan = plan_on(today, history);
        for w in windows.iter().filter(|w| w.season_year == 2026) {
            let Some(id) = activity_for(w.id) else {
                continue;
            };
            let status = plan.iter().find(|a| a.id == id).map(|a| a.status);
            assert_eq!(status, expected_status(w.state), "{id} on {today}");
            seen.push(w.state);
        }
    }
    for state in [
        WindowState::NotYet,
        WindowState::OpeningSoon,
        WindowState::Open,
        WindowState::Ideal,
        WindowState::Closing,
        WindowState::Closed,
        WindowState::Done,
    ] {
        assert!(seen.contains(&state), "covered {state:?}");
    }
}

// @spec TIME-PLAN-003
#[test]
fn a_window_without_typical_dates_or_blocked_by_the_log_is_left_off() {
    // No complete history year: no typical dates, so no activities at all.
    let days: Vec<_> = station(date(2026, 9, 19), 0.0)
        .into_iter()
        .filter(|d| d.date.year() == 2026)
        .collect();
    let data = SeasonData::build(&days, &[], 2026);
    let season = data.season(&days, date(2026, 9, 20));
    assert!(activities(&season, GrassType::TallFescue, &[], 2026).is_empty());

    // Seeding blocked by a fall pre-emergent takes aeration with it.
    let blocked = plan_on(
        date(2026, 9, 20),
        &[applied(ApplicationType::PreEmergent, date(2026, 9, 5))],
    );
    let ids: Vec<&str> = blocked.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, vec!["pre_emergent", "fall_pre_emergent"]);
}

// @spec TIME-PLAN-005
#[test]
fn the_plan_and_the_recommendation_state_the_same_seed_amount() {
    let amount = format!("{OVERSEED_RATE_LBS_PER_KSQFT:.0} lbs/1000 sqft");
    let today = date(2026, 9, 20);
    let plan = plan_on(today, &[]);
    let seeding = plan.iter().find(|a| a.id == "fall_overseeding").unwrap();
    let rate = seeding.details.rate.clone().unwrap();
    assert!(rate.contains(&amount), "{rate}");

    let days = station(date(2026, 9, 19), 0.0);
    let result = assess(&Inputs {
        today,
        days: &days,
        forecast: &[],
        grass: GrassType::TallFescue,
        history: &[],
    });
    let soil = SoilNow {
        avg_5day_f: None,
        as_of: None,
        depth_cm: 5,
    };
    let recs = to_recommendations(&result.windows, &soil, &LawnProfile::default(), today);
    let rec = recs
        .iter()
        .find(|r| r.id == "timing_fall_seeding_2026")
        .unwrap();
    let action = rec.suggested_action.clone().unwrap();
    assert!(action.contains(&amount), "{action}");
}
