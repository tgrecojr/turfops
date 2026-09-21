use chrono::{Duration, NaiveDate};

/// Trailing window for the soil-temperature mean. Smooths out single-day spikes.
pub const SMOOTHING_DAYS: i64 = 5;
/// Days of the trailing window that must have data for the mean to count.
const MIN_SMOOTHING_POINTS: usize = 4;
/// Consecutive days the smoothed value must hold past a threshold. Filters warm spells
/// in late winter and cold snaps in early fall that are not the seasonal transition.
pub const SUSTAIN_DAYS: u32 = 5;

/// One day of the 5 cm soil series (°F). Forecast days are regression estimates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoilPoint {
    pub date: NaiveDate,
    pub temp_f: f64,
    pub is_forecast: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Rising,
    Falling,
}

/// A month/day in the season year (`year_offset` 0) or the following one (1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeasonDay {
    pub year_offset: i32,
    pub month: u32,
    pub day: u32,
}

impl SeasonDay {
    pub const fn new(year_offset: i32, month: u32, day: u32) -> Self {
        Self {
            year_offset,
            month,
            day,
        }
    }

    pub fn in_season(&self, season_year: i32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(season_year + self.year_offset, self.month, self.day)
    }
}

/// "The smoothed 5 cm soil temperature reaches `threshold_f` and stays there", searched
/// between `scan_from` and `scan_to`. The condition is state-based rather than a
/// below→above transition, so a season that starts already past the threshold opens
/// on `scan_from`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    pub threshold_f: f64,
    pub direction: Direction,
    pub scan_from: SeasonDay,
    pub scan_to: SeasonDay,
}

impl Crossing {
    fn is_met(&self, temp_f: f64) -> bool {
        match self.direction {
            Direction::Rising => temp_f >= self.threshold_f,
            Direction::Falling => temp_f <= self.threshold_f,
        }
    }
}

/// A run of days past the threshold, dated by its first day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    pub date: NaiveDate,
    /// Observed (non-forecast) days in the run.
    pub days_held: u32,
    /// The run starts on the first day of data after a gap, so the real crossing
    /// happened somewhere inside the gap. Fine for "has it happened yet?", but not a
    /// date to put into the historical record.
    pub after_gap: bool,
}

impl Detected {
    /// Held for the full sustain period in station data.
    pub fn confirmed(&self) -> bool {
        self.days_held >= SUSTAIN_DAYS
    }

    /// Only the forecast puts the soil past the threshold.
    pub fn forecast_only(&self) -> bool {
        self.days_held == 0
    }
}

/// Trailing `SMOOTHING_DAYS` mean by calendar date. Days without enough data in their
/// window are dropped, so gaps in the input become gaps in the output.
pub fn smooth(daily: &[SoilPoint]) -> Vec<SoilPoint> {
    let mut out = Vec::with_capacity(daily.len());
    for (i, point) in daily.iter().enumerate() {
        let window_start = point.date - Duration::days(SMOOTHING_DAYS - 1);
        let window: Vec<f64> = daily[..=i]
            .iter()
            .rev()
            .take_while(|p| p.date >= window_start)
            .map(|p| p.temp_f)
            .collect();
        if window.len() >= MIN_SMOOTHING_POINTS {
            out.push(SoilPoint {
                temp_f: window.iter().sum::<f64>() / window.len() as f64,
                ..*point
            });
        }
    }
    out
}

/// First run in the scan range that either holds for `SUSTAIN_DAYS` observed days
/// (confirmed) or is still holding at the end of the series (tentative or forecast).
/// A data gap or a day back across the threshold resets the run.
pub fn detect(smoothed: &[SoilPoint], season_year: i32, crossing: &Crossing) -> Option<Detected> {
    let from = crossing.scan_from.in_season(season_year)?;
    let to = crossing.scan_to.in_season(season_year)?;
    let last_date = smoothed.last()?.date;

    let mut run: Option<(Detected, NaiveDate)> = None; // (run so far, date of its last day)
    for (i, point) in smoothed.iter().enumerate() {
        if point.date < from || point.date > to {
            continue;
        }
        if !crossing.is_met(point.temp_f) {
            run = None;
            continue;
        }
        let observed = u32::from(!point.is_forecast);
        let current = match run {
            Some((detected, prev)) if point.date == prev + Duration::days(1) => Detected {
                days_held: detected.days_held + observed,
                ..detected
            },
            _ => {
                let yesterday = point.date - Duration::days(1);
                let follows_data = i > 0 && smoothed[i - 1].date == yesterday;
                Detected {
                    date: point.date,
                    days_held: observed,
                    // A season already past the threshold opens on the scan start.
                    after_gap: point.date != from && !follows_data,
                }
            }
        };
        if current.confirmed() {
            return Some(current);
        }
        run = Some((current, point.date));
    }

    // An unconfirmed run only counts while it is still in progress.
    run.filter(|(_, end)| *end == last_date)
        .map(|(detected, _)| detected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    /// Consecutive observed days starting at `start`.
    fn observed(start: NaiveDate, temps: &[f64]) -> Vec<SoilPoint> {
        temps
            .iter()
            .enumerate()
            .map(|(i, &temp_f)| SoilPoint {
                date: start + Duration::days(i as i64),
                temp_f,
                is_forecast: false,
            })
            .collect()
    }

    const FALL_70: Crossing = Crossing {
        threshold_f: 70.0,
        direction: Direction::Falling,
        scan_from: SeasonDay::new(0, 8, 1),
        scan_to: SeasonDay::new(0, 12, 31),
    };

    #[test]
    fn smoothing_needs_four_of_five_days() {
        let mut daily = observed(date(9, 1), &[60.0, 62.0, 64.0, 66.0, 68.0]);
        daily.remove(1); // Sep 2 missing
        let smoothed = smooth(&daily);
        assert_eq!(smoothed.len(), 1);
        assert_eq!(smoothed[0].date, date(9, 5));
        assert!((smoothed[0].temp_f - 64.5).abs() < 1e-9);
    }

    #[test]
    fn cold_snap_shorter_than_sustain_is_ignored() {
        // Three cool days, a rebound, then the real transition.
        let series = observed(
            date(9, 1),
            &[
                72.0, 69.0, 69.0, 69.0, 73.0, 74.0, 69.5, 69.0, 68.0, 67.0, 66.0, 65.0,
            ],
        );
        let detected = detect(&series, 2026, &FALL_70).unwrap();
        assert_eq!(detected.date, date(9, 7));
        assert!(detected.confirmed());
    }

    #[test]
    fn run_in_progress_is_tentative_until_held() {
        let series = observed(date(9, 1), &[74.0, 73.0, 69.0, 68.0]);
        let detected = detect(&series, 2026, &FALL_70).unwrap();
        assert_eq!(detected.date, date(9, 3));
        assert_eq!(detected.days_held, 2);
        assert!(!detected.confirmed());
    }

    #[test]
    fn broken_run_is_not_reported() {
        let series = observed(date(9, 1), &[74.0, 69.0, 68.0, 72.0]);
        assert_eq!(detect(&series, 2026, &FALL_70), None);
    }

    #[test]
    fn forecast_days_do_not_confirm_a_crossing() {
        let mut series = observed(date(9, 1), &[74.0, 73.0, 69.0, 68.0, 67.0, 66.0, 65.0]);
        for point in series.iter_mut().skip(4) {
            point.is_forecast = true;
        }
        let detected = detect(&series, 2026, &FALL_70).unwrap();
        assert_eq!(detected.days_held, 2);
        assert!(!detected.confirmed());

        for point in series.iter_mut().skip(2) {
            point.is_forecast = true;
        }
        assert!(detect(&series, 2026, &FALL_70).unwrap().forecast_only());
    }

    #[test]
    fn season_already_past_threshold_opens_on_scan_start() {
        let series = observed(date(7, 28), &[68.0; 10]);
        assert_eq!(detect(&series, 2026, &FALL_70).unwrap().date, date(8, 1));
    }

    #[test]
    fn data_gap_resets_the_run() {
        let mut series = observed(date(9, 1), &[69.0; 9]);
        series.remove(3); // Sep 4 missing: run restarts on Sep 5
        assert_eq!(detect(&series, 2026, &FALL_70).unwrap().date, date(9, 5));
    }

    #[test]
    fn run_starting_right_after_an_outage_is_flagged() {
        // Sensor down from Aug 20; back on Oct 16 with the soil long since below 70°F.
        let mut series = observed(date(8, 10), &[76.0; 10]);
        series.extend(observed(date(10, 16), &[58.0; 8]));
        let detected = detect(&series, 2026, &FALL_70).unwrap();
        assert_eq!(detected.date, date(10, 16));
        assert!(detected.after_gap);

        // A crossing seen in continuous data, or a season open at scan start, is not.
        let continuous = observed(date(9, 1), &[72.0, 69.0, 68.0, 67.0, 66.0, 65.0]);
        assert!(!detect(&continuous, 2026, &FALL_70).unwrap().after_gap);
        let open_at_start = observed(date(8, 1), &[68.0; 6]);
        assert!(!detect(&open_at_start, 2026, &FALL_70).unwrap().after_gap);
    }

    #[test]
    fn rising_crossing_scans_the_following_year_when_offset() {
        let closes = Crossing {
            threshold_f: 45.0,
            direction: Direction::Rising,
            scan_from: SeasonDay::new(1, 2, 1),
            scan_to: SeasonDay::new(1, 7, 31),
        };
        let series = observed(date(3, 10), &[40.0, 44.0, 46.0, 47.0, 48.0, 49.0, 50.0]);
        assert_eq!(detect(&series, 2025, &closes).unwrap().date, date(3, 12));
        assert_eq!(detect(&series, 2026, &closes), None);
    }
}
