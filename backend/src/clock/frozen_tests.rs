//! One reading of the clock, and a clock stopped at a chosen instant.

use super::*;

fn at(utc: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(utc)
        .unwrap()
        .with_timezone(&Utc)
}

// @spec DEPLOY-TZ-007
#[test]
fn a_frozen_clock_reads_its_instant_and_that_instants_local_date() {
    let stopped = Clock::Frozen {
        at: at("2026-01-01T03:30:00Z"),
        zone: chrono_tz::America::New_York,
    };
    assert_eq!(stopped.now(), at("2026-01-01T03:30:00Z"));
    // 03:30 UTC on New Year's Day is still New Year's Eve in New York.
    assert_eq!(
        stopped.today(),
        NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()
    );
    assert_eq!(
        stopped.moment(),
        Moment {
            now: at("2026-01-01T03:30:00Z"),
            today: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        }
    );
    assert_eq!(stopped.now(), stopped.now(), "a frozen clock does not move");
    assert!(!stopped.is_system());
}

// @spec DEPLOY-TZ-007
#[test]
fn a_running_clock_reads_the_real_time_and_one_moment_agrees_with_itself() {
    let clock = Clock::Zone(chrono_tz::America::New_York);
    let before = Utc::now();
    let moment = clock.moment();
    assert!(moment.now >= before && moment.now <= Utc::now());
    assert_eq!(moment.today, clock.local_date(moment.now));
}

// @spec DEPLOY-TZ-007
#[test]
fn configuration_never_produces_a_frozen_clock() {
    for tz in [None, Some(""), Some("UTC"), Some("America/New_York")] {
        assert!(!matches!(Clock::from_tz(tz).unwrap(), Clock::Frozen { .. }));
    }
}

// @spec DEPLOY-TZ-007
#[test]
fn no_production_code_reads_the_process_clock_for_a_date() {
    let readers: Vec<String> = crate::test_support::production_sources()
        .into_iter()
        .filter(|f| !f.ends_with("src/clock.rs"))
        .filter(|f| {
            let code = crate::test_support::production_code(f);
            code.contains("Local::now") || code.contains("Local::today")
        })
        .map(|f| f.display().to_string())
        .collect();
    assert!(
        readers.is_empty(),
        "read the app clock instead: {readers:?}"
    );
}
