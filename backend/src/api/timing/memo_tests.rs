use super::memo::*;
use chrono::{DateTime, Duration, NaiveDate, Utc};

fn at(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

#[test]
fn a_record_is_fresh_for_an_hour() {
    let read = at("2026-10-11T12:00:00Z");
    assert!(is_fresh(read, read + Duration::minutes(59)));
    assert!(!is_fresh(read, read + Duration::minutes(60)));
}

// @spec SYNC-MEMO-002
#[test]
fn after_a_failed_reread_the_last_good_record_stands_in_for_a_day() {
    let read = at("2026-10-11T12:00:00Z");
    assert!(may_stand_in(read, read + Duration::hours(2)));
    assert!(may_stand_in(
        read,
        read + Duration::hours(23) + Duration::minutes(59)
    ));
    assert!(
        !may_stand_in(read, read + Duration::hours(24)),
        "a day old: return the error"
    );
}

// @spec TIME-ENG-046
#[test]
fn the_last_good_note_says_so_and_when() {
    let read = NaiveDate::from_ymd_opt(2026, 10, 11)
        .unwrap()
        .and_hms_opt(14, 5, 0)
        .unwrap();
    let note = last_good_note(read);
    assert!(note.contains("last good read"), "{note}");
    assert!(
        note.contains("Oct 11") && note.contains("2:05 PM"),
        "{note}"
    );
}
