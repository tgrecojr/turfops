use crate::datasources::weather::ClimateDay;
use crate::models::seasonal_plan::WindowConfidence;
use crate::models::timing::DateStat;
use chrono::{Datelike, Duration, NaiveDate};

/// A day counts as a freeze when the minimum air temperature is at or below this (°F).
pub const FREEZE_F: f64 = 32.0;
/// Share of days with data required between a freeze and the edge of its season (after a
/// last spring freeze, before a first fall freeze) for that date to be trusted.
const MIN_FREEZE_COVERAGE: f64 = 0.9;
/// Days of 5 cm soil data a calendar year needs to count toward the typical dates.
const MIN_SOIL_DAYS_PER_YEAR: usize = 300;
/// Share of elapsed days that need a GDD value for the year's running total to be trusted.
const MIN_GDD_COVERAGE: f64 = 0.8;

fn year_days(days: &[ClimateDay], year: i32) -> impl Iterator<Item = &ClimateDay> {
    days.iter().filter(move |d| d.date.year() == year)
}

fn is_freeze(day: &ClimateDay) -> bool {
    day.air_min_f.is_some_and(|min| min <= FREEZE_F)
}

/// Last freeze of the first half of `year`, whatever that year's data coverage.
pub fn last_spring_freeze(days: &[ClimateDay], year: i32) -> Option<NaiveDate> {
    year_days(days, year)
        .filter(|d| d.date.month() <= 6 && is_freeze(d))
        .map(|d| d.date)
        .last()
}

/// First freeze on or after Aug 1 of `year`, whatever that year's data coverage.
pub fn first_fall_freeze(days: &[ClimateDay], year: i32) -> Option<NaiveDate> {
    year_days(days, year)
        .find(|d| d.date.month() >= 8 && is_freeze(d))
        .map(|d| d.date)
}

/// Share of days in `[from, to]` with an air minimum on record.
fn air_coverage(days: &[ClimateDay], from: NaiveDate, to: NaiveDate) -> f64 {
    let span = (to - from).num_days() + 1;
    if span <= 0 {
        return 1.0;
    }
    let present = days
        .iter()
        .filter(|d| d.date >= from && d.date <= to && d.air_min_f.is_some())
        .count();
    present as f64 / span as f64
}

/// A first fall freeze is only as good as the record *before* it: a hole there could be
/// hiding an earlier one. What happens afterwards is irrelevant — a December outage must
/// not throw away an October freeze that was plainly observed.
fn fall_freeze_is_sound(days: &[ClimateDay], freeze: NaiveDate) -> bool {
    NaiveDate::from_ymd_opt(freeze.year(), 9, 15)
        .is_some_and(|from| air_coverage(days, from, freeze) >= MIN_FREEZE_COVERAGE)
}

/// Likewise a last spring freeze needs the record *after* it, through May.
fn spring_freeze_is_sound(days: &[ClimateDay], freeze: NaiveDate) -> bool {
    NaiveDate::from_ymd_opt(freeze.year(), 5, 31)
        .is_some_and(|to| air_coverage(days, freeze, to) >= MIN_FREEZE_COVERAGE)
}

pub fn spring_freeze_history(days: &[ClimateDay], years: &[i32]) -> Vec<(i32, NaiveDate)> {
    years
        .iter()
        .filter_map(|&y| last_spring_freeze(days, y).map(|d| (y, d)))
        .filter(|(_, freeze)| spring_freeze_is_sound(days, *freeze))
        .collect()
}

/// `(year, date)` of each historical first fall freeze with enough Oct–Dec data.
pub fn fall_freeze_history(days: &[ClimateDay], years: &[i32]) -> Vec<(i32, NaiveDate)> {
    years
        .iter()
        .filter_map(|&y| first_fall_freeze(days, y).map(|d| (y, d)))
        .filter(|(_, freeze)| fall_freeze_is_sound(days, *freeze))
        .collect()
}

/// First day the year's running GDD (base 50 °F) total reaches `target`.
pub fn gdd_reached(days: &[ClimateDay], year: i32, target: f64) -> Option<NaiveDate> {
    let mut total = 0.0;
    let mut counted = 0usize;
    for day in year_days(days, year) {
        let Some(gdd) = day.gdd50 else { continue };
        total += gdd;
        counted += 1;
        if total >= target {
            let covered = counted as f64 / day.date.ordinal() as f64;
            return (covered >= MIN_GDD_COVERAGE).then_some(day.date);
        }
    }
    None
}

/// Complete calendar years before `current_year` with enough soil data, ascending.
pub fn history_years(days: &[ClimateDay], current_year: i32) -> Vec<i32> {
    let Some(first) = days.first().map(|d| d.date.year()) else {
        return Vec::new();
    };
    (first..current_year)
        .filter(|&y| {
            year_days(days, y)
                .filter(|d| d.soil_temp_5_f.is_some())
                .count()
                >= MIN_SOIL_DAYS_PER_YEAR
        })
        .collect()
}

fn confidence(sample_count: usize) -> WindowConfidence {
    match sample_count {
        n if n >= 8 => WindowConfidence::High,
        n if n >= 4 => WindowConfidence::Medium,
        _ => WindowConfidence::Low,
    }
}

/// Summarize one event across years. Each event is `(season_year, date)`; it is reduced
/// to days since Jan 1 of its season year, so events that spill into the following
/// calendar year (dormant seeding) keep their order. Results are dates in
/// `target_season_year`.
pub fn date_stat(events: &[(i32, NaiveDate)], target_season_year: i32) -> Option<DateStat> {
    let mut offsets: Vec<i64> = events
        .iter()
        .filter_map(|&(season_year, date)| {
            let anchor = NaiveDate::from_ymd_opt(season_year, 1, 1)?;
            Some((date - anchor).num_days())
        })
        .collect();
    if offsets.is_empty() {
        return None;
    }
    offsets.sort_unstable();

    let n = offsets.len();
    let anchor = NaiveDate::from_ymd_opt(target_season_year, 1, 1)?;
    let to_date = |offset: i64| anchor + Duration::days(offset);
    // Nearest-rank percentile.
    let rank = |p: f64| offsets[((p * n as f64).ceil() as usize).clamp(1, n) - 1];
    let median = if n % 2 == 1 {
        offsets[n / 2]
    } else {
        (offsets[n / 2 - 1] + offsets[n / 2]) / 2
    };

    Some(DateStat {
        median: to_date(median),
        p10: to_date(rank(0.1)),
        p90: to_date(rank(0.9)),
        earliest: to_date(offsets[0]),
        latest: to_date(offsets[n - 1]),
        sample_count: n,
        confidence: confidence(n),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    /// A year of daily air minimums from `min_for(date)`, with GDD 10/day from May 1.
    fn year_of(year: i32, min_for: impl Fn(NaiveDate) -> f64) -> Vec<ClimateDay> {
        date(year, 1, 1)
            .iter_days()
            .take_while(|d| d.year() == year)
            .map(|d| ClimateDay {
                date: d,
                soil_temp_5_f: Some(55.0),
                air_min_f: Some(min_for(d)),
                air_avg_f: None,
                gdd50: Some(if d.month() >= 5 { 10.0 } else { 0.0 }),
            })
            .collect()
    }

    /// Freezing through Apr 5 and again from Nov 8.
    fn typical_min(d: NaiveDate) -> f64 {
        let thaw = date(d.year(), 4, 5);
        let frost = date(d.year(), 11, 8);
        if d <= thaw || d >= frost {
            28.0
        } else {
            45.0
        }
    }

    #[test]
    fn finds_last_spring_and_first_fall_freeze() {
        let days = year_of(2024, typical_min);
        assert_eq!(last_spring_freeze(&days, 2024), Some(date(2024, 4, 5)));
        assert_eq!(first_fall_freeze(&days, 2024), Some(date(2024, 11, 8)));
        assert_eq!(first_fall_freeze(&days, 2023), None);
    }

    #[test]
    fn freeze_history_skips_years_missing_the_season() {
        let mut days = year_of(2023, typical_min);
        // 2024 has no data after September: a fall freeze cannot be established.
        days.extend(
            year_of(2024, typical_min)
                .into_iter()
                .filter(|d| d.date.month() <= 9),
        );
        assert_eq!(spring_freeze_history(&days, &[2023, 2024]).len(), 2);
        assert_eq!(
            fall_freeze_history(&days, &[2023, 2024]),
            vec![(2023, date(2023, 11, 8))]
        );
    }

    #[test]
    fn an_outage_after_the_first_freeze_does_not_discard_it() {
        // The real station, 2018: first freeze Oct 22, then most of December missing.
        let days: Vec<ClimateDay> = year_of(2023, typical_min)
            .into_iter()
            .filter(|d| d.date.month() != 12)
            .collect();
        assert_eq!(
            fall_freeze_history(&days, &[2023]),
            vec![(2023, date(2023, 11, 8))]
        );

        // A hole *before* the freeze could hide an earlier one: not trusted.
        let holed: Vec<ClimateDay> = year_of(2023, typical_min)
            .into_iter()
            .filter(|d| d.date.month() != 10)
            .collect();
        assert!(fall_freeze_history(&holed, &[2023]).is_empty());
    }

    #[test]
    fn gdd_target_date_requires_coverage() {
        let days = year_of(2024, typical_min);
        // 10/day from May 1 → 200 on May 20.
        assert_eq!(gdd_reached(&days, 2024, 200.0), Some(date(2024, 5, 20)));

        let sparse: Vec<ClimateDay> = days.into_iter().filter(|d| d.date.month() >= 5).collect();
        assert_eq!(gdd_reached(&sparse, 2024, 200.0), None);
    }

    #[test]
    fn history_excludes_current_and_sparse_years() {
        let mut days = year_of(2023, typical_min);
        days.extend(year_of(2024, typical_min).into_iter().take(100));
        days.extend(year_of(2025, typical_min));
        days.extend(year_of(2026, typical_min));
        assert_eq!(history_years(&days, 2026), vec![2023, 2025]);
    }

    #[test]
    fn date_stat_reports_median_and_spread_in_the_target_year() {
        let events = [
            (2021, date(2021, 9, 10)),
            (2022, date(2022, 9, 20)),
            (2023, date(2023, 9, 14)),
            (2025, date(2025, 9, 30)),
        ];
        let stat = date_stat(&events, 2026).unwrap();
        assert_eq!(stat.median, date(2026, 9, 17)); // midpoint of Sep 14 and Sep 20
        assert_eq!(stat.earliest, date(2026, 9, 10));
        assert_eq!(stat.latest, date(2026, 9, 30));
        assert_eq!(stat.p10, date(2026, 9, 10));
        assert_eq!(stat.p90, date(2026, 9, 30));
        assert_eq!(stat.sample_count, 4);
        assert_eq!(stat.confidence, WindowConfidence::Medium);
        assert_eq!(date_stat(&[], 2026), None);
    }

    #[test]
    fn date_stat_keeps_order_across_the_new_year() {
        // Dormant-seeding style events: two in December, one the following January.
        let events = [
            (2022, date(2022, 12, 20)),
            (2023, date(2024, 1, 5)),
            (2025, date(2025, 12, 28)),
        ];
        let stat = date_stat(&events, 2026).unwrap();
        assert_eq!(stat.median, date(2026, 12, 28));
        assert_eq!(stat.latest, date(2027, 1, 5));
    }
}
