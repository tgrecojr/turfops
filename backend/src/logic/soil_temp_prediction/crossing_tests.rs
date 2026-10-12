//! The 10 cm outlook's thresholds and how crossings are found, approached and ordered.

use super::*;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()
}

/// Predictions on the days after `today`, one per value.
fn days(temps: &[f64]) -> Vec<SoilTempPrediction> {
    temps
        .iter()
        .enumerate()
        .map(|(i, t)| SoilTempPrediction {
            date: today() + chrono::Duration::days(i as i64 + 1),
            predicted_soil_temp_f: *t,
            confidence: PredictionConfidence::High,
            air_temp_used_f: *t,
            source_description: "test".into(),
        })
        .collect()
}

fn names(crossings: &[ThresholdPrediction]) -> Vec<&str> {
    crossings
        .iter()
        .map(|c| c.threshold_name.as_str())
        .collect()
}

// @spec WX-OUTLOOK-CROSS-001
#[test]
fn the_outlook_has_exactly_four_direction_aware_thresholds() {
    use CrossingDirection::*;
    let table: Vec<(&str, f64, Option<CrossingDirection>)> = THRESHOLDS
        .iter()
        .map(|t| (t.name, t.temp_f, t.direction))
        .collect();
    assert_eq!(
        table,
        vec![
            ("Winterizer / Dormancy", 45.0, Some(Falling)),
            ("Grub Control Window", 60.0, Some(Rising)),
            ("Active Growth Peak", 65.0, None),
            ("Heat Stress Risk", 75.0, Some(Rising)),
        ]
    );
    // Warming from 40 to 80°F passes 45, 50 and 55 too; only 60, 65 and 75 are announced.
    let warming =
        predict_threshold_crossings(40.0, &days(&[48.0, 53.0, 58.0, 62.0, 80.0]), today());
    assert_eq!(
        names(&warming),
        vec![
            "Grub Control Window",
            "Active Growth Peak",
            "Heat Stress Risk"
        ]
    );
    // Cooling from 80 to 40°F: only 65 (either way) and 45 (falling) are announced.
    let cooling = predict_threshold_crossings(80.0, &days(&[62.0, 52.0, 40.0]), today());
    assert_eq!(
        names(&cooling),
        vec!["Active Growth Peak", "Winterizer / Dormancy"]
    );
}

// @spec WX-OUTLOOK-CROSS-002
#[test]
fn the_first_step_across_is_the_crossing() {
    // 64 → 66 → 63 → 67: Active Growth Peak (65, either way) is first crossed on day 1.
    let c = predict_threshold_crossings(64.0, &days(&[66.0, 63.0, 67.0]), today());
    let peak = c.iter().find(|c| c.threshold_temp_f == 65.0).unwrap();
    assert_eq!(
        peak.estimated_crossing_date,
        Some(today() + chrono::Duration::days(1))
    );
    assert_eq!(peak.direction, CrossingDirection::Rising);
    assert_eq!(c.iter().filter(|c| c.threshold_temp_f == 65.0).count(), 1);
}

// @spec WX-OUTLOOK-CROSS-002
#[test]
fn a_first_step_the_wrong_way_ends_the_search_for_that_threshold() {
    // Grub Control is rising-only: dipping through 60 first means no announcement, even
    // though the soil warms back through it two days later.
    let c = predict_threshold_crossings(61.0, &days(&[59.0, 58.0, 62.0]), today());
    assert!(
        !names(&c).contains(&"Grub Control Window"),
        "{:?}",
        names(&c)
    );
}

// @spec WX-OUTLOOK-CROSS-003
#[test]
fn soil_within_5f_and_warming_gets_a_low_confidence_approach() {
    // 56 → 57 → 58 over two days: 60 is 4°F away and not reached.
    let c = predict_threshold_crossings(56.0, &days(&[57.0, 58.0]), today());
    let grub = c
        .iter()
        .find(|c| c.threshold_temp_f == 60.0)
        .expect("approaching 60");
    assert_eq!(grub.confidence, PredictionConfidence::Low);
    assert_eq!(grub.direction, CrossingDirection::Rising);
    // 4°F needed at 2°F per 2-day span: (4 / 2) × 2 = 4 days.
    assert_eq!(grub.days_until_crossing, 4);
    // Active Growth Peak (65) is 9°F away: not approaching.
    assert!(c.iter().all(|c| c.threshold_temp_f != 65.0));
}

// @spec WX-OUTLOOK-CROSS-003
#[test]
fn no_approach_for_a_falling_only_threshold_or_cooling_soil() {
    // 43 → 44: below Winterizer (45, falling only) and warming toward it.
    let c = predict_threshold_crossings(43.0, &days(&[43.5, 44.0]), today());
    assert!(c.is_empty(), "{:?}", names(&c));
    // Below 60 and within 5°F, but cooling.
    let cooling = predict_threshold_crossings(58.0, &days(&[57.5, 57.0]), today());
    assert!(cooling.is_empty(), "{:?}", names(&cooling));
}

// @spec WX-OUTLOOK-CROSS-004
#[test]
fn crossings_are_sorted_by_days_until_crossing() {
    // Grub Control on day 3, Active Growth Peak on day 4, Heat Stress on day 5.
    let c = predict_threshold_crossings(58.0, &days(&[58.5, 59.0, 61.0, 66.0, 76.0]), today());
    let order: Vec<i64> = c.iter().map(|c| c.days_until_crossing).collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);
    assert_eq!(
        names(&c),
        vec![
            "Grub Control Window",
            "Active Growth Peak",
            "Heat Stress Risk"
        ]
    );
}
