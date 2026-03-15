use crate::models::seasonal_plan::*;
use crate::models::{Application, ApplicationType};
use chrono::{Datelike, NaiveDate};

/// Soil temperature thresholds (°F, 7-day rolling average) that trigger activities.
/// Each threshold has a name used as cache key and the temp value.
pub const THRESHOLDS: &[(&str, f64)] = &[
    ("soil_50f_rising", 50.0),  // Pre-emergent, aeration, overseeding low
    ("soil_55f_rising", 55.0),  // Spring N, broadleaf herbicide, overseeding peak
    ("soil_60f_rising", 60.0),  // Grub control start
    ("soil_65f_rising", 65.0),  // Grub/aeration/overseeding upper limit
    ("soil_75f_rising", 75.0),  // Grub upper limit
    ("soil_65f_falling", 65.0), // Fall activities begin
    ("soil_55f_falling", 55.0), // Fall herbicide lower
    ("soil_50f_falling", 50.0), // Overseeding/aeration lower on fall side
    ("soil_45f_falling", 45.0), // Winterizer window
];

/// Compute 7-day rolling average soil temps and detect threshold crossings.
pub fn find_threshold_crossings(
    year: i32,
    daily_temps: &[DailySoilTempAvg],
) -> Vec<ThresholdCrossing> {
    if daily_temps.len() < 7 {
        return Vec::new();
    }

    let mut crossings = Vec::new();
    let mut found: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Compute 7-day rolling average
    let rolling: Vec<(NaiveDate, f64)> = daily_temps
        .windows(7)
        .map(|w| {
            let avg = w.iter().map(|d| d.avg_temp_f).sum::<f64>() / 7.0;
            (w[6].date, avg)
        })
        .collect();

    if rolling.len() < 2 {
        return crossings;
    }

    // Find rising crossings (Jan-Jun): temp goes from below to at/above threshold
    for window in rolling.windows(2) {
        let (prev_date, prev_avg) = window[0];
        let (curr_date, curr_avg) = window[1];

        // Only look at rising crossings in first half of year
        if curr_date.month() <= 6 {
            for &(name, threshold) in THRESHOLDS {
                if !name.contains("rising") {
                    continue;
                }
                let key = format!("{}_{}", year, name);
                if !found.contains(&key) && prev_avg < threshold && curr_avg >= threshold {
                    crossings.push(ThresholdCrossing {
                        year,
                        threshold_name: name.to_string(),
                        crossing_date: curr_date,
                        avg_soil_temp_f: curr_avg,
                    });
                    found.insert(key);
                }
            }
        }

        // Falling crossings in second half (Jul-Dec): temp goes from above to at/below
        if curr_date.month() >= 7 || prev_date.month() >= 7 {
            for &(name, threshold) in THRESHOLDS {
                if !name.contains("falling") {
                    continue;
                }
                let key = format!("{}_{}", year, name);
                if !found.contains(&key) && prev_avg > threshold && curr_avg <= threshold {
                    crossings.push(ThresholdCrossing {
                        year,
                        threshold_name: name.to_string(),
                        crossing_date: curr_date,
                        avg_soil_temp_f: curr_avg,
                    });
                    found.insert(key);
                }
            }
        }
    }

    crossings
}

/// Aggregate threshold crossings across multiple years to produce median/earliest/latest.
pub fn aggregate_crossings(
    crossings: &[ThresholdCrossing],
    threshold_name: &str,
    target_year: i32,
) -> Option<AggregatedWindow> {
    let mut dates: Vec<NaiveDate> = crossings
        .iter()
        .filter(|c| c.threshold_name == threshold_name)
        .map(|c| {
            // Normalize to target year for day-of-year comparison
            NaiveDate::from_yo_opt(target_year, c.crossing_date.ordinal())
                .unwrap_or(c.crossing_date)
        })
        .collect();

    if dates.is_empty() {
        return None;
    }

    dates.sort();

    let earliest = *dates.first().unwrap();
    let latest = *dates.last().unwrap();
    let median = dates[dates.len() / 2];

    Some(AggregatedWindow {
        median,
        earliest,
        latest,
        sample_count: dates.len(),
    })
}

#[derive(Debug, Clone)]
pub struct AggregatedWindow {
    pub median: NaiveDate,
    pub earliest: NaiveDate,
    pub latest: NaiveDate,
    pub sample_count: usize,
}

/// Build the full seasonal plan from aggregated threshold data and application history.
pub fn build_seasonal_plan(
    year: i32,
    crossings: &[ThresholdCrossing],
    applications: &[Application],
    data_years: i32,
) -> SeasonalPlan {
    let today = chrono::Local::now().date_naive();
    let activities = build_activities(year, crossings, applications, today);

    SeasonalPlan {
        year,
        activities,
        data_years_used: data_years,
        generated_at: chrono::Utc::now(),
    }
}

fn build_activities(
    year: i32,
    crossings: &[ThresholdCrossing],
    applications: &[Application],
    today: NaiveDate,
) -> Vec<PlannedActivity> {
    let mut activities = Vec::new();

    // 1. Pre-Emergent (soil rising through 50°F → 60°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_50f_rising", year) {
        let end = aggregate_crossings(crossings, "soil_60f_rising", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(30));

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::PreEmergent && a.application_date.year() == year
        });

        activities.push(PlannedActivity {
            id: "pre_emergent".into(),
            name: "Pre-Emergent Herbicide".into(),
            category: "Weed Prevention".into(),
            description: "Apply pre-emergent before crabgrass germination. \
                Soil temp at 50-60°F is the target window."
                .into(),
            date_window: make_window(&start, end_date, end.as_ref()),
            status: compute_status(completed, start.median, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("50-60°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "Prodiamine (Barricade)".into(),
                    "Dithiopyr (Dimension)".into(),
                    "Pendimethalin".into(),
                ],
                rate: Some("Label rate".into()),
                notes: Some("Water in within 24 hours. Do not aerate after application.".into()),
            },
        });
    }

    // 2. Spring Nitrogen (soil rising through 55°F → 65°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_55f_rising", year) {
        let end_date = aggregate_crossings(crossings, "soil_65f_rising", year)
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(21));

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::Fertilizer
                && a.application_date.year() == year
                && a.application_date.month() <= 5
        });

        activities.push(PlannedActivity {
            id: "spring_nitrogen".into(),
            name: "Spring Nitrogen".into(),
            category: "Fertilization".into(),
            description: "Light nitrogen application to fuel spring green-up. \
                Use slow-release to avoid surge growth."
                .into(),
            date_window: make_window(&start, end_date, None),
            status: compute_status(completed, start.median, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("55-65°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "Slow-release granular (e.g., Milorganite)".into(),
                    "Synthetic slow-release 30-0-4".into(),
                ],
                rate: Some("0.5 lb N/1000 sqft".into()),
                notes: Some("Avoid fast-release N in spring to limit top growth.".into()),
            },
        });
    }

    // 3. Spring Broadleaf Herbicide (soil 45-55°F rising)
    if let Some(start_agg) = aggregate_crossings(crossings, "soil_50f_rising", year) {
        let start_date = start_agg.median - chrono::Duration::days(7);
        let end = aggregate_crossings(crossings, "soil_55f_rising", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median + chrono::Duration::days(7))
            .unwrap_or_else(|| start_date + chrono::Duration::days(28));

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::PostEmergent
                && a.application_date.year() == year
                && a.application_date.month() <= 5
        });

        activities.push(PlannedActivity {
            id: "spring_herbicide".into(),
            name: "Spring Broadleaf Herbicide".into(),
            category: "Weed Control".into(),
            description: "Target spring broadleaf weeds (dandelion, clover) \
                during active growth before heat stress."
                .into(),
            date_window: DateWindow {
                predicted_start: clamp_year(start_date, year),
                predicted_end: clamp_year(end_date, year),
                earliest_historical: Some(start_agg.earliest - chrono::Duration::days(7)),
                latest_historical: end.as_ref().map(|e| e.latest + chrono::Duration::days(7)),
                confidence: confidence_from_count(start_agg.sample_count),
            },
            status: compute_status(completed, start_date, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("45-55°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "2,4-D + triclopyr + dicamba blend".into(),
                    "Trimec".into(),
                ],
                rate: Some("Label rate".into()),
                notes: Some("Apply when weeds are actively growing. Avoid if temps >85°F.".into()),
            },
        });
    }

    // 4. Grub Preventative (soil rising through 60-75°F, calendar May 15 - Jul 4)
    if let Some(start) = aggregate_crossings(crossings, "soil_60f_rising", year) {
        let end = aggregate_crossings(crossings, "soil_75f_rising", year);
        let end_date = end.as_ref().map(|e| e.median).unwrap_or_else(|| {
            NaiveDate::from_ymd_opt(year, 7, 4).unwrap_or(start.median + chrono::Duration::days(45))
        });
        // Clamp start to no earlier than May 15
        let start_date = start
            .median
            .max(NaiveDate::from_ymd_opt(year, 5, 15).unwrap_or(start.median));

        let completed = applications.iter().any(|a| {
            (a.application_type == ApplicationType::GrubControl
                || a.application_type == ApplicationType::Insecticide)
                && a.application_date.year() == year
        });

        activities.push(PlannedActivity {
            id: "grub_preventative".into(),
            name: "Grub Preventative".into(),
            category: "Pest Control".into(),
            description: "Preventative grub control during beetle egg-laying season. \
                Soil temp 60-75°F window."
                .into(),
            date_window: make_window_with_dates(start_date, end_date, &start, end.as_ref()),
            status: compute_status(completed, start_date, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("60-75°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "Chlorantraniliprole (GrubEx)".into(),
                    "Imidacloprid + bifenthrin".into(),
                ],
                rate: Some("Label rate".into()),
                notes: Some("Apply before grub eggs hatch. Water in well.".into()),
            },
        });
    }

    // 5. Core Aeration (fall — soil falling through 65°F → 50°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_65f_falling", year) {
        let end = aggregate_crossings(crossings, "soil_50f_falling", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(30));

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::Aeration && a.application_date.year() == year
        });

        activities.push(PlannedActivity {
            id: "core_aeration".into(),
            name: "Core Aeration".into(),
            category: "Lawn Health".into(),
            description: "Relieve soil compaction and improve root growth. \
                Best done in early fall when soil is 50-65°F and grass is actively growing."
                .into(),
            date_window: make_window(&start, end_date, end.as_ref()),
            status: compute_status(completed, start.median, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("50-65°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![],
                rate: None,
                notes: Some("Aerate before overseeding. 2-3 inch cores, 2-3 inch spacing.".into()),
            },
        });
    }

    // 6. Fall Overseeding (soil falling through 65°F → 50°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_65f_falling", year) {
        let end = aggregate_crossings(crossings, "soil_50f_falling", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(28));

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::Overseed && a.application_date.year() == year
        });

        activities.push(PlannedActivity {
            id: "fall_overseeding".into(),
            name: "Fall Overseeding".into(),
            category: "Lawn Repair".into(),
            description: "Overseed thin or bare areas. Fall is the best time \
                for cool-season grass seed germination."
                .into(),
            date_window: make_window(&start, end_date, end.as_ref()),
            status: compute_status(completed, start.median, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("50-65°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "TTTF blend (3+ cultivars)".into(),
                    "KBG/TTTF mix".into(),
                ],
                rate: Some("4 lbs/1000 sqft (overseeding)".into()),
                notes: Some(
                    "Aerate first. Keep soil moist for 2-3 weeks. \
                     No pre-emergent for 60 days after."
                        .into(),
                ),
            },
        });
    }

    // 7. Early Fall Fertilization (soil falling through 65°F, Sept)
    if let Some(start) = aggregate_crossings(crossings, "soil_65f_falling", year) {
        let start_date = start.median;
        let end_date = start_date + chrono::Duration::days(21);

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::Fertilizer
                && a.application_date.year() == year
                && a.application_date.month() == 9
        });

        activities.push(PlannedActivity {
            id: "early_fall_fert".into(),
            name: "Early Fall Fertilization".into(),
            category: "Fertilization".into(),
            description: "First fall feeding to support recovery from summer stress. \
                Heavier N rate to fuel root development."
                .into(),
            date_window: DateWindow {
                predicted_start: start_date,
                predicted_end: end_date,
                earliest_historical: Some(start.earliest),
                latest_historical: Some(start.latest + chrono::Duration::days(21)),
                confidence: confidence_from_count(start.sample_count),
            },
            status: compute_status(completed, start_date, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("45-65°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "Balanced fertilizer (e.g., 24-0-6)".into(),
                    "Milorganite".into(),
                ],
                rate: Some("1.0 lb N/1000 sqft".into()),
                notes: Some("21-day minimum before next fertilizer application.".into()),
            },
        });
    }

    // 8. Mid Fall Fertilization (21 days after early fall)
    if let Some(start) = aggregate_crossings(crossings, "soil_65f_falling", year) {
        let start_date = start.median + chrono::Duration::days(21);
        let end_date = start_date + chrono::Duration::days(21);

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::Fertilizer
                && a.application_date.year() == year
                && a.application_date.month() == 10
        });

        activities.push(PlannedActivity {
            id: "mid_fall_fert".into(),
            name: "Mid Fall Fertilization".into(),
            category: "Fertilization".into(),
            description: "Second fall feeding. Continue building root reserves \
                before dormancy."
                .into(),
            date_window: DateWindow {
                predicted_start: start_date,
                predicted_end: end_date,
                earliest_historical: Some(start.earliest + chrono::Duration::days(21)),
                latest_historical: Some(start.latest + chrono::Duration::days(42)),
                confidence: confidence_from_count(start.sample_count),
            },
            status: compute_status(completed, start_date, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("45-65°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "Balanced fertilizer (e.g., 24-0-6)".into(),
                    "Milorganite".into(),
                ],
                rate: Some("0.75 lb N/1000 sqft".into()),
                notes: Some("21-day minimum before winterizer.".into()),
            },
        });
    }

    // 9. Winterizer (soil falling through 45°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_45f_falling", year) {
        let end_date = start.median + chrono::Duration::days(14);

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::Fertilizer
                && a.application_date.year() == year
                && a.application_date.month() >= 11
        });

        activities.push(PlannedActivity {
            id: "winterizer".into(),
            name: "Winterizer Application".into(),
            category: "Fertilization".into(),
            description: "Final fertilizer of the season. Fast-release N applied after \
                top growth stops but roots are still active."
                .into(),
            date_window: DateWindow {
                predicted_start: start.median,
                predicted_end: end_date,
                earliest_historical: Some(start.earliest),
                latest_historical: Some(start.latest + chrono::Duration::days(14)),
                confidence: confidence_from_count(start.sample_count),
            },
            status: compute_status(completed, start.median, end_date, today),
            details: ActivityDetails {
                soil_temp_trigger: Some("40-45°F (7-day avg at 10cm)".into()),
                product_suggestions: vec![
                    "Fast-release urea (46-0-0)".into(),
                    "Winterizer blend (32-0-10)".into(),
                ],
                rate: Some("1.0 lb N/1000 sqft".into()),
                notes: Some("Apply after last mow of the season. Soil must be above 40°F.".into()),
            },
        });
    }

    // 10. Fall Broadleaf Herbicide (soil falling through 65°F → 55°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_65f_falling", year) {
        let end = aggregate_crossings(crossings, "soil_55f_falling", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(28));

        let completed = applications.iter().any(|a| {
            a.application_type == ApplicationType::PostEmergent
                && a.application_date.year() == year
                && a.application_date.month() >= 9
        });

        // Don't show if overseeded within 60 days
        let recently_overseeded = applications.iter().any(|a| {
            a.application_type == ApplicationType::Overseed
                && a.application_date.year() == year
                && (start.median - a.application_date).num_days() < 60
        });

        if !recently_overseeded {
            activities.push(PlannedActivity {
                id: "fall_herbicide".into(),
                name: "Fall Broadleaf Herbicide".into(),
                category: "Weed Control".into(),
                description: "Fall is the most effective time for broadleaf weed control. \
                    Weeds are translocating nutrients to roots."
                    .into(),
                date_window: make_window(&start, end_date, end.as_ref()),
                status: compute_status(completed, start.median, end_date, today),
                details: ActivityDetails {
                    soil_temp_trigger: Some("50-65°F (7-day avg at 10cm)".into()),
                    product_suggestions: vec![
                        "2,4-D + triclopyr + dicamba blend".into(),
                        "Trimec".into(),
                    ],
                    rate: Some("Label rate".into()),
                    notes: Some(
                        "Do not apply within 60 days of overseeding. \
                         Avoid if temps >85°F."
                            .into(),
                    ),
                },
            });
        }
    }

    // Sort by predicted start date
    activities.sort_by_key(|a| a.date_window.predicted_start);

    activities
}

fn make_window(
    start: &AggregatedWindow,
    end_date: NaiveDate,
    end_agg: Option<&AggregatedWindow>,
) -> DateWindow {
    DateWindow {
        predicted_start: start.median,
        predicted_end: end_date,
        earliest_historical: Some(start.earliest),
        latest_historical: end_agg.map(|e| e.latest),
        confidence: confidence_from_count(start.sample_count),
    }
}

fn make_window_with_dates(
    start_date: NaiveDate,
    end_date: NaiveDate,
    start_agg: &AggregatedWindow,
    end_agg: Option<&AggregatedWindow>,
) -> DateWindow {
    DateWindow {
        predicted_start: start_date,
        predicted_end: end_date,
        earliest_historical: Some(start_agg.earliest),
        latest_historical: end_agg.map(|e| e.latest),
        confidence: confidence_from_count(start_agg.sample_count),
    }
}

fn confidence_from_count(count: usize) -> WindowConfidence {
    if count >= 5 {
        WindowConfidence::High
    } else if count >= 3 {
        WindowConfidence::Medium
    } else {
        WindowConfidence::Low
    }
}

fn compute_status(
    completed: bool,
    start: NaiveDate,
    end: NaiveDate,
    today: NaiveDate,
) -> ActivityStatus {
    if completed {
        ActivityStatus::Completed
    } else if today > end {
        ActivityStatus::Missed
    } else if today >= start {
        ActivityStatus::Active
    } else {
        ActivityStatus::Upcoming
    }
}

fn clamp_year(date: NaiveDate, year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, date.month(), date.day().min(28)).unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_daily(dates_temps: &[(u32, u32, f64)]) -> Vec<DailySoilTempAvg> {
        dates_temps
            .iter()
            .map(|&(month, day, temp)| DailySoilTempAvg {
                date: NaiveDate::from_ymd_opt(2025, month, day).unwrap(),
                avg_temp_f: temp,
            })
            .collect()
    }

    #[test]
    fn finds_rising_crossing() {
        // 14 days of data: first 7 at 48°F, then crosses to 52°F
        let mut temps = Vec::new();
        for d in 1..=7 {
            temps.push((3, d, 48.0));
        }
        for d in 8..=14 {
            temps.push((3, d, 52.0));
        }
        let daily = make_daily(&temps);
        let crossings = find_threshold_crossings(2025, &daily);
        assert!(crossings
            .iter()
            .any(|c| c.threshold_name == "soil_50f_rising"));
    }

    #[test]
    fn no_crossing_when_below() {
        let mut temps = Vec::new();
        for d in 1..=14 {
            temps.push((3, d, 45.0));
        }
        let daily = make_daily(&temps);
        let crossings = find_threshold_crossings(2025, &daily);
        assert!(crossings.is_empty());
    }

    #[test]
    fn aggregate_gives_median() {
        let crossings = vec![
            ThresholdCrossing {
                year: 2022,
                threshold_name: "soil_50f_rising".into(),
                crossing_date: NaiveDate::from_ymd_opt(2022, 3, 10).unwrap(),
                avg_soil_temp_f: 50.5,
            },
            ThresholdCrossing {
                year: 2023,
                threshold_name: "soil_50f_rising".into(),
                crossing_date: NaiveDate::from_ymd_opt(2023, 3, 15).unwrap(),
                avg_soil_temp_f: 50.2,
            },
            ThresholdCrossing {
                year: 2024,
                threshold_name: "soil_50f_rising".into(),
                crossing_date: NaiveDate::from_ymd_opt(2024, 3, 20).unwrap(),
                avg_soil_temp_f: 51.0,
            },
        ];
        let agg = aggregate_crossings(&crossings, "soil_50f_rising", 2025).unwrap();
        assert_eq!(agg.median.month(), 3);
        assert_eq!(agg.median.day(), 15);
        assert_eq!(agg.sample_count, 3);
    }

    #[test]
    fn compute_status_active() {
        let today = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let start = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 15).unwrap();
        assert!(matches!(
            compute_status(false, start, end, today),
            ActivityStatus::Active
        ));
    }

    #[test]
    fn compute_status_completed() {
        let today = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let start = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 15).unwrap();
        assert!(matches!(
            compute_status(true, start, end, today),
            ActivityStatus::Completed
        ));
    }

    #[test]
    fn compute_status_missed() {
        let today = NaiveDate::from_ymd_opt(2025, 5, 1).unwrap();
        let start = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 4, 15).unwrap();
        assert!(matches!(
            compute_status(false, start, end, today),
            ActivityStatus::Missed
        ));
    }
}
