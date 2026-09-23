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
    /// Last day of station soil data. The lake trails today by a day or two, and a
    /// crossing cannot be "late" on days the station has not reported yet.
    pub last_observed: Option<NaiveDate>,
}

/// A boundary's standing in the evaluated season.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub date: Option<NaiveDate>,
    pub source: DateSource,
    pub passed: bool,
    pub days_held: Option<u32>,
}

/// When the boundary fell in a past year. A freeze-anchored boundary is dated from that
/// year's own freeze so the typical spread reflects how much the freeze really moves;
/// only the current season (`resolve`) pins it to the typical freeze, since by the time
/// a freeze is in the forecast it is too late to seed.
fn historical_date(boundary: &Boundary, season: &Season, year: i32) -> Option<NaiveDate> {
    match boundary {
        // A run that begins right after a sensor outage dates the outage, not the crossing.
        Boundary::Soil(crossing) => detect(season.smoothed, year, crossing)
            .filter(|d| d.confirmed() && !d.after_gap)
            .map(|d| d.date),
        Boundary::BeforeFirstFreeze(days) => season
            .fall_freezes
            .iter()
            .find(|(y, _)| *y == year)
            .map(|(_, freeze)| *freeze - Duration::days(*days)),
        Boundary::Gdd(target) => gdd_reached(season.days, year, *target),
        // The earliest of several triggers is unknown while any one of them is: a year
        // with no freeze on record must not report its soil crossing as "the close".
        Boundary::Earliest(options) => options
            .iter()
            .map(|o| historical_date(o, season, year))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .min(),
    }
}

/// Typical dates for the boundary, expressed in `season_year`.
pub fn typical(boundary: &Boundary, season: &Season, season_year: i32) -> Option<DateStat> {
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
    // Overdue means the station has data *past* the typical date and still shows no
    // crossing. Until fresh data gets there, the typical date remains the estimate. (With
    // stale data the calendar decides instead: a typical date in the past has passed.)
    let seen_through = season.last_observed.unwrap_or(season.today);
    let (date, passed) = match median {
        Some(m) if m > season.today || (season.data_fresh && m > seen_through) => (Some(m), false),
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

    #[test]
    fn freeze_anchored_boundaries_carry_the_freezes_own_spread() {
        use crate::logic::timing::tests::{date, station};
        use crate::logic::timing::windows::specs_for;
        use crate::logic::timing::SeasonData;
        use crate::models::lawn_profile::GrassType;
        use crate::models::timing::WindowId;

        let today = date(2026, 9, 20);
        let days = station(date(2026, 9, 19), 0.0);
        let data = SeasonData::build(&days, &[], 2026);
        // The synthetic station freezes on the same day every year; give it a real
        // spread instead, and leave 2020 and 2024 without a freeze on record (they are
        // also the leap years, whose day-of-year offsets would land a day later in 2026).
        let freezes = vec![
            (2021, date(2021, 11, 10)),
            (2022, date(2022, 10, 30)),
            (2023, date(2023, 11, 5)),
            (2025, date(2025, 10, 20)),
        ];
        let season = Season {
            fall_freezes: &freezes,
            ..data.season(&days, today)
        };
        let specs = specs_for(GrassType::TallFescue);
        let seeding = specs
            .iter()
            .find(|s| s.id == WindowId::FallSeeding)
            .unwrap();

        // Ideal-until is a bare freeze boundary: 45 d before each year's freeze.
        let ideal = typical(seeding.ideal_until.as_ref().unwrap(), &season, 2026).unwrap();
        assert_eq!(ideal.sample_count, 4);
        assert_eq!(ideal.earliest, date(2026, 9, 5));
        assert_eq!(ideal.latest, date(2026, 9, 26));

        // Closes is the earlier of 30 d before the freeze and soil cooling to 55°F
        // (≈ Oct 20 on this curve, so the freeze buffer wins every year). It must carry
        // the same spread as the freeze, not collapse to one date — and the years with
        // no freeze on record must not contribute their soil crossing.
        let closes = typical(&seeding.closes, &season, 2026).unwrap();
        assert_eq!(closes.sample_count, 4);
        assert_eq!(closes.earliest, date(2026, 9, 20));
        assert_eq!(closes.latest, date(2026, 10, 11));
        assert!(closes.p10 < closes.p90);

        // The current season still pins to the typical freeze.
        let resolved = resolve(&seeding.closes, &season, 2026);
        assert_eq!(resolved.source, DateSource::Typical);
        assert_eq!(resolved.date, Some(closes.median));
    }
}
