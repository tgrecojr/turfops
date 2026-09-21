use super::climatology::{date_stat, gdd_reached};
use super::series::{detect, Detected, SeasonDay, SoilPoint};
use super::windows::{Boundary, WindowSpec};
use crate::datasources::weather::ClimateDay;
use crate::models::timing::{BoundaryView, DateSource, DateStat, WindowState};
use chrono::{Duration, NaiveDate};

/// A window that opens within this many days is flagged as opening soon.
const OPENING_SOON_DAYS: i64 = 7;
/// GDD accumulates through summer, but a spring GDD trigger is moot after this.
const GDD_SCAN_END: SeasonDay = SeasonDay::new(0, 7, 31);

/// Everything the windows are evaluated against.
pub struct Season<'a> {
    pub today: NaiveDate,
    pub days: &'a [ClimateDay],
    /// Smoothed 5 cm soil series, all years, ending with any forecast days.
    pub smoothed: &'a [SoilPoint],
    pub history_years: &'a [i32],
    /// Historical first fall freezes as `(year, date)`.
    pub fall_freezes: &'a [(i32, NaiveDate)],
    /// Station soil data reaches close enough to today to judge what has happened.
    /// When false, boundaries fall back to the calendar.
    pub data_fresh: bool,
}

/// A boundary's standing in the evaluated season.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub date: Option<NaiveDate>,
    pub source: DateSource,
    pub passed: bool,
    pub days_held: Option<u32>,
}

fn shift(stat: DateStat, days: i64) -> DateStat {
    let by = Duration::days(days);
    DateStat {
        median: stat.median + by,
        p10: stat.p10 + by,
        p90: stat.p90 + by,
        earliest: stat.earliest + by,
        latest: stat.latest + by,
        ..stat
    }
}

/// When the boundary fell in a past year. Freeze-anchored boundaries use the typical
/// freeze rather than that year's actual one, which nobody could have known in advance.
fn historical_date(boundary: &Boundary, season: &Season, year: i32) -> Option<NaiveDate> {
    match boundary {
        // A run that begins right after a sensor outage dates the outage, not the crossing.
        Boundary::Soil(crossing) => detect(season.smoothed, year, crossing)
            .filter(|d| d.confirmed() && !d.after_gap)
            .map(|d| d.date),
        Boundary::BeforeFirstFreeze(days) => {
            date_stat(season.fall_freezes, year).map(|s| s.median - Duration::days(*days))
        }
        Boundary::Gdd(target) => gdd_reached(season.days, year, *target),
        Boundary::Earliest(options) => options
            .iter()
            .filter_map(|o| historical_date(o, season, year))
            .min(),
    }
}

/// Typical dates for the boundary, expressed in `season_year`.
pub fn typical(boundary: &Boundary, season: &Season, season_year: i32) -> Option<DateStat> {
    if let Boundary::BeforeFirstFreeze(days) = boundary {
        // Carry the freeze's own year-to-year spread rather than a constant.
        return date_stat(season.fall_freezes, season_year).map(|s| shift(s, -days));
    }
    let events: Vec<(i32, NaiveDate)> = season
        .history_years
        .iter()
        .filter_map(|&y| historical_date(boundary, season, y).map(|d| (y, d)))
        .collect();
    date_stat(&events, season_year)
}

/// A trigger not seen in the data: judge it against its typical date. While station
/// data is current and the scan range is still open, a typical date in the past means
/// "overdue" (no date), not "passed".
fn from_typical(
    median: Option<NaiveDate>,
    scan_to: SeasonDay,
    season: &Season,
    year: i32,
) -> Resolved {
    let scan_over = scan_to
        .in_season(year)
        .is_some_and(|end| season.today > end);
    let (date, passed) = match median {
        Some(m) if m > season.today => (Some(m), false),
        Some(m) if scan_over || !season.data_fresh => (Some(m), true),
        _ => (None, false),
    };
    Resolved {
        date,
        source: DateSource::Typical,
        passed,
        days_held: None,
    }
}

fn from_detected(detected: Detected, today: NaiveDate) -> Resolved {
    if detected.forecast_only() {
        // The outlook starts the day after the last station day, which trails today by a
        // day or two — a forecast crossing in that gap means "about now", not a past date.
        return Resolved {
            date: Some(detected.date.max(today)),
            source: DateSource::Forecast,
            passed: false,
            days_held: None,
        };
    }
    let confirmed = detected.confirmed();
    Resolved {
        date: Some(detected.date),
        source: if confirmed {
            DateSource::Observed
        } else {
            DateSource::Tentative
        },
        passed: true,
        days_held: (!confirmed).then_some(detected.days_held),
    }
}

pub fn resolve(boundary: &Boundary, season: &Season, season_year: i32) -> Resolved {
    let median = || typical(boundary, season, season_year).map(|s| s.median);
    match boundary {
        // With stale station data only a confirmed crossing is trusted. A run still in
        // progress when the data stopped, or one seen only in a forecast that no longer
        // connects to the station series, falls back to the calendar like no run at all.
        Boundary::Soil(crossing) => match detect(season.smoothed, season_year, crossing) {
            Some(detected) if season.data_fresh || detected.confirmed() => {
                from_detected(detected, season.today)
            }
            _ => from_typical(median(), crossing.scan_to, season, season_year),
        },
        Boundary::BeforeFirstFreeze(_) => {
            let date = median();
            Resolved {
                date,
                source: DateSource::Typical,
                passed: date.is_some_and(|d| d <= season.today),
                days_held: None,
            }
        }
        Boundary::Gdd(target) => match gdd_reached(season.days, season_year, *target) {
            Some(date) => Resolved {
                date: Some(date),
                source: DateSource::Observed,
                passed: true,
                days_held: None,
            },
            None => from_typical(median(), GDD_SCAN_END, season, season_year),
        },
        Boundary::Earliest(options) => {
            let resolved: Vec<Resolved> = options
                .iter()
                .map(|o| resolve(o, season, season_year))
                .collect();
            let first = |passed: bool| {
                resolved
                    .iter()
                    .filter(|r| r.passed == passed && r.date.is_some())
                    .min_by_key(|r| r.date)
                    .cloned()
            };
            first(true).or_else(|| first(false)).unwrap_or(Resolved {
                date: None,
                source: DateSource::Typical,
                passed: false,
                days_held: None,
            })
        }
    }
}

pub fn view(boundary: &Boundary, season: &Season, season_year: i32) -> BoundaryView {
    let resolved = resolve(boundary, season, season_year);
    BoundaryView {
        label: boundary.label(),
        typical: typical(boundary, season, season_year),
        date: resolved.date,
        source: resolved.source,
        passed: resolved.passed,
        days_held: resolved.days_held,
    }
}

/// The four edges of a window, resolved for one season.
pub struct WindowViews {
    pub opens: BoundaryView,
    pub ideal_from: Option<BoundaryView>,
    pub ideal_until: Option<BoundaryView>,
    pub closes: BoundaryView,
}

impl WindowViews {
    pub fn build(spec: &WindowSpec, season: &Season, season_year: i32) -> Self {
        let optional = |b: &Option<Boundary>| b.as_ref().map(|b| view(b, season, season_year));
        Self {
            opens: view(&spec.opens, season, season_year),
            ideal_from: optional(&spec.ideal_from),
            ideal_until: optional(&spec.ideal_until),
            closes: view(&spec.closes, season, season_year),
        }
    }

    /// The closing threshold has been reached but has not yet held long enough to be the
    /// seasonal shift. One warm (or cold) day must not declare the window over — and then
    /// reopen it after the next front — so this reads as Closing, not Closed.
    pub fn closing_tentatively(&self) -> bool {
        self.closes.passed && self.closes.source == DateSource::Tentative
    }

    /// State from weather and climatology alone, before the application log is considered.
    pub fn weather_state(&self, today: NaiveDate) -> WindowState {
        if self.closes.passed && !self.closing_tentatively() {
            return WindowState::Closed;
        }
        if !self.opens.passed {
            let soon = self.opens.source == DateSource::Forecast
                || self
                    .opens
                    .date
                    .is_some_and(|d| (d - today).num_days() <= OPENING_SOON_DAYS);
            return if soon {
                WindowState::OpeningSoon
            } else {
                WindowState::NotYet
            };
        }
        if self.closing_tentatively() {
            WindowState::Closing
        } else if self.ideal_from.as_ref().is_some_and(|b| !b.passed) {
            WindowState::Open
        } else if self.ideal_until.as_ref().is_some_and(|b| b.passed) {
            WindowState::Closing
        } else if self.ideal_from.is_some() || self.ideal_until.is_some() {
            WindowState::Ideal
        } else {
            WindowState::Open
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forecast_crossing_in_the_lake_lag_is_dated_today_not_in_the_past() {
        // Last station day Sep 18, today Sep 20: the outlook's first day is Sep 19.
        let today = NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let seen = Detected {
            date: NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            days_held: 0,
            after_gap: false,
        };
        let resolved = from_detected(seen, today);
        assert_eq!(resolved.source, DateSource::Forecast);
        assert_eq!(resolved.date, Some(today));
        assert!(!resolved.passed);
    }
}
