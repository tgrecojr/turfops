//! The climate-record slot's rules: when a memoized record is reused as fresh, and when
//! it may stand in for a failed re-read.

use chrono::{DateTime, Duration, Utc};

/// The climate record spans 15 years of hourly soil data and the lake only gains a day at
/// a time, so one read serves an hour of requests (TIME-ENG-038).
pub(crate) const FRESH_FOR: Duration = Duration::hours(1);
/// After a failed re-read, the last good record is served for this long.
pub(crate) const LAST_GOOD_FOR: Duration = Duration::hours(24);

// @spec TIME-ENG-038
/// Reuse the slot's record without reading the lake.
pub(crate) fn is_fresh(read_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - read_at < FRESH_FOR
}

/// After a failed re-read, whether the slot's record may still be served.
// @spec SYNC-MEMO-002
pub(crate) fn may_stand_in(read_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - read_at < LAST_GOOD_FOR
}

/// The Timing page's data note when the climate record is the last good read.
// @spec TIME-ENG-046
pub(crate) fn last_good_note(read_at_local: chrono::NaiveDateTime) -> String {
    format!(
        "The weather lake could not be read just now; station history is from the last good \
         read ({}).",
        read_at_local.format("%b %-d, %-I:%M %p")
    )
}
