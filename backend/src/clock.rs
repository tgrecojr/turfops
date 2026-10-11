//! The app's one clock. "Today", every date gate and every year boundary are the lawn's
//! local calendar day, taken from `TZ` (which deployment sets to the lawn's timezone).
//! `TZ` is resolved once at startup; everything that needs a local date takes a `Clock`
//! (or a `today` computed from one) rather than reading a clock of its own.

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;

/// Where local dates come from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Clock {
    /// `TZ` named an IANA zone.
    Zone(Tz),
    /// `TZ` is unset or empty: the host's default zone (UTC in the container image).
    #[default]
    System,
}

impl Clock {
    // @spec DEPLOY-TZ-005
    /// Resolve the value of `TZ`. Unset or blank means the host's default zone; a value
    /// that is not an IANA zone name is a configuration error naming `TZ`.
    pub fn from_tz(value: Option<&str>) -> Result<Self, String> {
        match value.map(str::trim) {
            None | Some("") => Ok(Clock::System),
            Some(name) => name.parse::<Tz>().map(Clock::Zone).map_err(|_| {
                format!(
                    "TZ={name:?} is not a timezone name; set TZ to the lawn's IANA zone, \
                     for example America/New_York"
                )
            }),
        }
    }

    /// Whether this clock follows the host's default zone because `TZ` is unset.
    pub fn is_system(&self) -> bool {
        matches!(self, Clock::System)
    }

    /// The local wall-clock time of an instant.
    pub fn local(&self, instant: DateTime<Utc>) -> NaiveDateTime {
        match self {
            Clock::Zone(tz) => instant.with_timezone(tz).naive_local(),
            Clock::System => instant.with_timezone(&Local).naive_local(),
        }
    }

    /// The local calendar date of an instant.
    pub fn local_date(&self, instant: DateTime<Utc>) -> NaiveDate {
        self.local(instant).date()
    }

    /// Today's local date.
    pub fn today(&self) -> NaiveDate {
        self.local_date(Utc::now())
    }

    /// One line for the startup log: the zone, its UTC offset at `instant` and the local
    /// date and time, so a wrong clock is visible at a glance.
    pub fn describe(&self, instant: DateTime<Utc>) -> String {
        let (zone, offset) = match self {
            Clock::Zone(tz) => (
                tz.name().to_string(),
                instant.with_timezone(tz).format("%:z"),
            ),
            Clock::System => (
                "system default".to_string(),
                instant.with_timezone(&Local).format("%:z"),
            ),
        };
        format!(
            "TZ={zone} (UTC{offset}), local time {}",
            self.local(instant).format("%Y-%m-%d %H:%M")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eastern() -> Clock {
        Clock::Zone(chrono_tz::America::New_York)
    }

    fn at(utc: &str) -> DateTime<Utc> {
        utc.parse().unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    // @spec DEPLOY-TZ-005
    #[test]
    fn a_tz_that_is_not_a_zone_is_a_configuration_error() {
        let err = Clock::from_tz(Some("America/New_Yrok")).unwrap_err();
        assert!(err.contains("TZ"), "error must name TZ: {err}");
        assert!(
            err.contains("America/New_Yrok"),
            "error must name the value: {err}"
        );
    }

    // @spec DEPLOY-TZ-004
    #[test]
    fn an_unset_or_blank_tz_follows_the_host_zone() {
        assert_eq!(Clock::from_tz(None).unwrap(), Clock::System);
        assert_eq!(Clock::from_tz(Some("  ")).unwrap(), Clock::System);
        assert!(Clock::from_tz(None).unwrap().is_system());
    }

    #[test]
    fn a_named_zone_resolves() {
        assert_eq!(Clock::from_tz(Some("America/New_York")).unwrap(), eastern());
        assert!(!eastern().is_system());
    }

    #[test]
    fn the_local_date_is_the_lawns_not_utcs() {
        // 10:30 pm Eastern on New Year's Eve is already Jan 1 in UTC.
        assert_eq!(
            eastern().local_date(at("2026-01-01T03:30:00Z")),
            date(2025, 12, 31)
        );
        // Same wall-clock evening in summer (EDT, UTC−4).
        assert_eq!(
            eastern().local_date(at("2026-07-15T03:30:00Z")),
            date(2026, 7, 14)
        );
    }

    #[test]
    fn local_time_follows_daylight_saving() {
        // DST ends 2026-11-01 at 06:00 UTC: 04:30 UTC on Nov 2 is 11:30 pm EST on Nov 1.
        let local = eastern().local(at("2026-11-02T04:30:00Z"));
        assert_eq!(local.date(), date(2026, 11, 1));
        assert_eq!(local.format("%H:%M").to_string(), "23:30");
    }

    // @spec DEPLOY-TZ-003
    #[test]
    fn the_startup_line_names_the_zone_offset_and_local_time() {
        let line = eastern().describe(at("2026-10-11T13:05:00Z"));
        assert!(line.contains("America/New_York"), "{line}");
        assert!(line.contains("-04:00"), "{line}");
        assert!(line.contains("2026-10-11 09:05"), "{line}");
    }
}
