use crate::datasources::weather::DailyGddRow;
use crate::models::{CrabgrassModel, CrabgrassStatus, DailyGdd, GddSummary};
use chrono::Datelike;

/// Crabgrass germination GDD threshold (base 50°F).
pub const CRABGRASS_GDD_THRESHOLD: f64 = 200.0;

/// GDD value at which to issue a warning that germination is approaching.
pub const CRABGRASS_GDD_WARNING: f64 = 150.0;

/// Accumulate cumulative GDD from the gold layer's daily rows
/// `(date, high_f, low_f, gdd50)`. The lake precomputes `gdd50` with the identical
/// base-50°F formula, so we trust it directly and only run the running sum here.
/// Resets accumulation at the start of each year. Rows must be sorted by date ascending.
pub fn accumulate_daily_gdd(daily_data: &[DailyGddRow]) -> Vec<DailyGdd> {
    let mut result = Vec::with_capacity(daily_data.len());
    let mut cumulative = 0.0;
    let mut current_year: Option<i32> = None;

    for &(date, high, low, gdd) in daily_data {
        // Reset accumulation on year boundary
        if current_year != Some(date.year()) {
            cumulative = 0.0;
            current_year = Some(date.year());
        }

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

    use chrono::NaiveDate;

    #[test]
    fn gdd_accumulation_running_sum() {
        // Rows carry the gold layer's precomputed gdd50 directly.
        let data = vec![
            (
                NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                70.0,
                50.0,
                10.0,
            ),
            (
                NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
                75.0,
                55.0,
                15.0,
            ),
            (
                NaiveDate::from_ymd_opt(2026, 3, 3).unwrap(),
                60.0,
                40.0,
                0.0,
            ),
        ];

        let result = accumulate_daily_gdd(&data);
        assert_eq!(result.len(), 3);
        assert!((result[0].cumulative_gdd_base50 - 10.0).abs() < 0.001);
        assert!((result[1].cumulative_gdd_base50 - 25.0).abs() < 0.001);
        assert!((result[2].cumulative_gdd_base50 - 25.0).abs() < 0.001);
    }

    #[test]
    fn gdd_accumulation_resets_each_year() {
        let data = vec![
            (
                NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
                70.0,
                50.0,
                10.0,
            ),
            (
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                75.0,
                55.0,
                15.0,
            ),
        ];

        let result = accumulate_daily_gdd(&data);
        assert!((result[0].cumulative_gdd_base50 - 10.0).abs() < 0.001);
        // Year boundary resets the running total.
        assert!((result[1].cumulative_gdd_base50 - 15.0).abs() < 0.001);
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
