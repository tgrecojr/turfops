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
    for d in 8..=21 {
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

#[test]
fn warm_season_lawns_get_no_cool_season_program() {
    // Every threshold the plan looks for, on a plausible date, for three past years.
    let crossings: Vec<ThresholdCrossing> = [
        ("soil_50f_rising", 4, 1),
        ("soil_55f_rising", 4, 20),
        ("soil_60f_rising", 5, 10),
        ("soil_65f_rising", 5, 25),
        ("soil_75f_rising", 6, 25),
        ("soil_65f_falling", 9, 25),
        ("soil_55f_falling", 10, 25),
        ("soil_50f_falling", 11, 5),
        ("soil_45f_falling", 11, 20),
    ]
    .iter()
    .flat_map(|(name, month, day)| {
        (2023..=2025).map(move |year| ThresholdCrossing {
            year,
            threshold_name: name.to_string(),
            crossing_date: NaiveDate::from_ymd_opt(year, *month, *day).unwrap(),
            avg_soil_temp_f: 0.0,
        })
    })
    .collect();
    let settled = [2023, 2024, 2025];
    let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
    let cool = build_seasonal_plan(
        2026,
        &crossings,
        &settled,
        &[],
        GrassType::TallFescue,
        today,
    );
    let warm = build_seasonal_plan(2026, &crossings, &settled, &[], GrassType::Bermuda, today);
    let has = |plan: &SeasonalPlan, id: &str| plan.activities.iter().any(|a| a.id == id);
    for id in COOL_SEASON_ONLY {
        assert!(has(&cool, id), "cool-season plan lost {id}");
        assert!(!has(&warm, id), "warm-season plan kept {id}");
    }
    // Weed and grub control apply to any turf.
    assert!(has(&warm, "pre_emergent"));
    assert!(has(&warm, "grub_preventative"));
}
