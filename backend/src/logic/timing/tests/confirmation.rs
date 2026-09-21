//! How much station evidence a crossing needs before a window acts on it.

use super::*;

/// The station record with its last `n` days of soil forced to `temp_f`.
fn with_final_days(mut days: Vec<ClimateDay>, n: usize, temp_f: f64) -> Vec<ClimateDay> {
    let len = days.len();
    for day in &mut days[len - n..] {
        day.soil_temp_5_f = Some(temp_f);
    }
    days
}

#[test]
fn a_brief_touch_of_the_closing_threshold_reads_closing_not_closed() {
    // Apr 12: the curve sits near 52°F. One freak 75°F day lifts the 5-day mean past
    // 55°F — reached, but nowhere near held.
    let today = date(2026, 4, 13);
    let days = with_final_days(station(date(2026, 4, 12), 0.0), 1, 75.0);
    let result = run(today, &days, &[]);

    let pre_em = window(&result, WindowId::SpringPreEmergent);
    assert_eq!(pre_em.closes.source, DateSource::Tentative);
    assert_eq!(pre_em.state, WindowState::Closing);
    assert!(
        pre_em.headline.contains("held 1 of 5"),
        "{}",
        pre_em.headline
    );
    // No "crabgrass has germinated, you missed it" while it could still be a blip.
    assert!(pre_em.guidance.iter().all(|g| !g.contains("likely begun")));

    // A cold front follows: the run never held, so the window is simply still Ideal —
    // it did not close and reopen.
    let mut cooled = station(date(2026, 4, 15), 0.0);
    let len = cooled.len();
    cooled[len - 4].soil_temp_5_f = Some(75.0);
    for day in &mut cooled[len - 3..] {
        day.soil_temp_5_f = Some(45.0);
    }
    let after = run(date(2026, 4, 16), &cooled, &[]);
    assert_eq!(
        window(&after, WindowId::SpringPreEmergent).state,
        WindowState::Ideal
    );
}

#[test]
fn a_run_in_progress_when_the_station_went_quiet_is_not_trusted() {
    // Data stops Mar 10 during a two-day warm spell; six days later nobody knows whether
    // it held. The calendar (typical open ≈ Mar 24) decides, as the data note promises.
    let days = with_final_days(station(date(2026, 3, 10), 0.0), 3, 60.0);
    let result = run(date(2026, 3, 16), &days, &[]);
    assert!(!result.data_fresh);

    let pre_em = window(&result, WindowId::SpringPreEmergent);
    assert_eq!(pre_em.opens.source, DateSource::Typical);
    assert!(!pre_em.opens.passed);
    assert_ne!(pre_em.state, WindowState::Open);
}
