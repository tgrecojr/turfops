use crate::models::{CrabgrassModel, CrabgrassStatus, DailyGdd, EnvironmentalReading, GddSummary};
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

/// GDD base temperature (°F) for cool-season turf calculations.
pub const GDD_BASE_TEMP_F: f64 = 50.0;

/// Crabgrass germination GDD threshold (base 50°F).
pub const CRABGRASS_GDD_THRESHOLD: f64 = 200.0;

/// GDD value at which to issue a warning that germination is approaching.
pub const CRABGRASS_GDD_WARNING: f64 = 150.0;

/// Calculate GDD base 50 for a single day from daily high/low temperatures.
pub fn daily_gdd_base50(high_f: f64, low_f: f64) -> f64 {
    ((high_f + low_f) / 2.0 - GDD_BASE_TEMP_F).max(0.0)
}

/// Compute daily high/low temperatures from hourly readings, grouped by date.
/// Returns entries sorted by date ascending.
pub fn compute_daily_highs_lows(readings: &[EnvironmentalReading]) -> Vec<(NaiveDate, f64, f64)> {
    let mut by_date: BTreeMap<NaiveDate, Vec<f64>> = BTreeMap::new();

    for reading in readings {
        if let Some(temp_f) = reading.ambient_temp_f {
            let date = reading.timestamp.date_naive();
            by_date.entry(date).or_default().push(temp_f);
        }
    }

    by_date
        .into_iter()
        .filter(|(_, temps)| temps.len() >= 12) // Require at least 12 hours of data
        .map(|(date, temps)| {
            let high = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let low = temps.iter().cloned().fold(f64::INFINITY, f64::min);
            (date, high, low)
        })
        .collect()
}

/// Build GDD accumulation from daily high/low data.
/// Resets accumulation at the start of each year.
pub fn accumulate_gdd(daily_data: &[(NaiveDate, f64, f64)]) -> Vec<DailyGdd> {
    let mut result = Vec::with_capacity(daily_data.len());
    let mut cumulative = 0.0;
    let mut current_year: Option<i32> = None;

    for &(date, high, low) in daily_data {
        // Reset accumulation on year boundary
        if current_year != Some(date.year()) {
            cumulative = 0.0;
            current_year = Some(date.year());
        }

        let gdd = daily_gdd_base50(high, low);
        cumulative += gdd;

        result.push(DailyGdd {
            date,
            high_temp_f: high,
            low_temp_f: low,
            gdd_base50: gdd,
            cumulative_gdd_base50: cumulative,
        });
    }

    result
}

/// Determine crabgrass model status based on current YTD GDD accumulation.
pub fn crabgrass_model(current_gdd: f64) -> CrabgrassModel {
    let status = if current_gdd >= CRABGRASS_GDD_THRESHOLD {
        CrabgrassStatus::PostGermination
    } else if current_gdd >= CRABGRASS_GDD_WARNING {
        CrabgrassStatus::GerminationLikely
    } else if current_gdd >= CRABGRASS_GDD_WARNING * 0.5 {
        CrabgrassStatus::ApproachingGermination
    } else {
        CrabgrassStatus::PreGermination
    };

    CrabgrassModel {
        germination_threshold: CRABGRASS_GDD_THRESHOLD,
        current_gdd,
        status,
        estimated_germination_date: None, // Could be computed from forecast data
    }
}

/// Build a complete GDD summary from daily GDD records for a given year.
pub fn build_gdd_summary(year: i32, daily_records: &[DailyGdd]) -> GddSummary {
    let year_records: Vec<DailyGdd> = daily_records
        .iter()
        .filter(|d| d.date.year() == year)
        .cloned()
        .collect();

    let current_gdd_total = year_records
        .last()
        .map(|d| d.cumulative_gdd_base50)
        .unwrap_or(0.0);

    let last_computed_date = year_records.last().map(|d| d.date);

    GddSummary {
        year,
        current_gdd_total,
        crabgrass_model: crabgrass_model(current_gdd_total),
        daily_history: year_records,
        last_computed_date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdd_base50_above_base() {
        // Average = (70 + 50) / 2 = 60, GDD = 60 - 50 = 10
        assert!((daily_gdd_base50(70.0, 50.0) - 10.0).abs() < 0.001);
    }

    #[test]
    fn gdd_base50_below_base() {
        // Average = (40 + 30) / 2 = 35, GDD = max(0, 35 - 50) = 0
        assert!((daily_gdd_base50(40.0, 30.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn gdd_base50_at_base() {
        // Average = (60 + 40) / 2 = 50, GDD = 0
        assert!((daily_gdd_base50(60.0, 40.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn gdd_accumulation() {
        let data = vec![
            (NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(), 70.0, 50.0), // GDD = 10
            (NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(), 75.0, 55.0), // GDD = 15
            (NaiveDate::from_ymd_opt(2026, 3, 3).unwrap(), 60.0, 40.0), // GDD = 0
        ];

        let result = accumulate_gdd(&data);
        assert_eq!(result.len(), 3);
        assert!((result[0].cumulative_gdd_base50 - 10.0).abs() < 0.001);
        assert!((result[1].cumulative_gdd_base50 - 25.0).abs() < 0.001);
        assert!((result[2].cumulative_gdd_base50 - 25.0).abs() < 0.001);
    }

    #[test]
    fn crabgrass_model_pre_germination() {
        let model = crabgrass_model(50.0);
        assert_eq!(model.status, CrabgrassStatus::PreGermination);
    }

    #[test]
    fn crabgrass_model_approaching() {
        let model = crabgrass_model(100.0);
        assert_eq!(model.status, CrabgrassStatus::ApproachingGermination);
    }

    #[test]
    fn crabgrass_model_likely() {
        let model = crabgrass_model(175.0);
        assert_eq!(model.status, CrabgrassStatus::GerminationLikely);
    }

    #[test]
    fn crabgrass_model_post() {
        let model = crabgrass_model(220.0);
        assert_eq!(model.status, CrabgrassStatus::PostGermination);
    }
}
