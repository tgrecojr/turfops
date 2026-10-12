//! The 10 cm activities' details (rate, wording, trigger), window confidence, and the
//! trailing mean the crossings are read from.

use super::test_support::*;
use super::*;
use crate::models::seasonal_plan::DailySoilTempAvg;

fn rate(plan: &SeasonalPlan, id: &str) -> Option<String> {
    activity(plan, id).details.rate.clone()
}

// @spec PLAN-ACT-014
#[test]
fn feedings_and_seed_carry_an_amount_pesticides_say_label_rate_aeration_none() {
    let p = plan(&[]);
    let expected = [
        ("spring_nitrogen", Some("0.5 lb N/1000 sqft")),
        ("early_fall_fert", Some("1.0 lb N/1000 sqft")),
        ("mid_fall_fert", Some("0.75 lb N/1000 sqft")),
        ("winterizer", Some("1.0 lb N/1000 sqft")),
        ("fall_overseeding", Some("4 lbs/1000 sqft (overseeding)")),
        ("pre_emergent", Some("Label rate")),
        ("spring_herbicide", Some("Label rate")),
        ("grub_preventative", Some("Label rate")),
        ("fall_herbicide", Some("Label rate")),
        ("core_aeration", None),
    ];
    for (id, want) in expected {
        assert_eq!(rate(&p, id).as_deref(), want, "{id}");
    }
}

/// Whether `text` names an amount: a unit of weight, volume or area, or "per 1,000".
fn states_an_amount(text: &str) -> bool {
    const UNITS: [&str; 12] = [
        "lb", "lbs", "oz", "gal", "gallon", "gallons", "qt", "pt", "ml", "acre", "acres", "kg",
    ];
    let lower = text.to_lowercase();
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|token| UNITS.contains(&token))
        || ["/1000", "/ 1000", "per 1000", "per 1,000", "/1,000"]
            .iter()
            .any(|p| lower.contains(p))
}

// @spec PLAN-ACT-015
#[test]
fn no_description_product_or_note_states_a_rate() {
    assert!(
        states_an_amount("Apply 0.5 oz per 1,000 sq ft"),
        "the check works"
    );
    for a in &plan(&[]).activities {
        let notes = a.details.notes.iter().map(String::as_str);
        let texts = std::iter::once(a.description.as_str())
            .chain(a.details.product_suggestions.iter().map(String::as_str))
            .chain(notes);
        for text in texts {
            assert!(!states_an_amount(text), "{}: {text}", a.id);
        }
    }
}

// @spec PLAN-ACT-016
#[test]
fn every_trigger_names_its_10cm_range_as_a_7_day_average() {
    for a in &plan(&[]).activities {
        let trigger = a.details.soil_temp_trigger.as_deref().unwrap_or_default();
        let range = trigger
            .strip_suffix("°F (7-day avg at 10cm)")
            .unwrap_or_else(|| panic!("{}: {trigger}", a.id));
        let bounds: Vec<u32> = range.split('-').filter_map(|n| n.parse().ok()).collect();
        assert_eq!(bounds.len(), 2, "{}: {trigger}", a.id);
    }
}

fn crossings_in(years: impl Iterator<Item = i32>, name: &str) -> Vec<ThresholdCrossing> {
    years
        .map(|year| ThresholdCrossing {
            year,
            threshold_name: name.into(),
            crossing_date: d(year, 4, 10),
            avg_soil_temp_f: 0.0,
        })
        .collect()
}

// @spec PLAN-CROSS-007
#[test]
fn confidence_comes_from_the_opening_thresholds_sample_count() {
    use WindowConfidence::*;
    let rated: Vec<WindowConfidence> = (0..=6).map(confidence_from_count).collect();
    assert_eq!(rated, [Low, Low, Low, Medium, Medium, High, High]);

    // Five years of 55°F rising, one of 65°F: spring nitrogen is rated on the five.
    let mut crossings = crossings_in(2021..=2025, "soil_55f_rising");
    crossings.extend(crossings_in(2025..=2025, "soil_65f_rising"));
    crossings.extend(crossings_in(2024..=2025, "soil_50f_rising"));
    let settled: Vec<i32> = (2021..=2025).collect();
    let p = build_seasonal_plan(
        2026,
        &crossings,
        &settled,
        &[],
        GrassType::TallFescue,
        d(2026, 1, 15),
    );
    assert_eq!(activity(&p, "spring_nitrogen").date_window.confidence, High);
    assert_eq!(activity(&p, "pre_emergent").date_window.confidence, Low);
}

fn rows(days: &[(NaiveDate, f64)]) -> Vec<DailySoilTempAvg> {
    days.iter()
        .map(|&(date, avg_temp_f)| DailySoilTempAvg { date, avg_temp_f })
        .collect()
}

// @spec PLAN-CROSS-002
#[test]
fn the_trailing_mean_is_over_seven_rows_stamped_on_the_seventh() {
    // 40°F Mar 1–10, no rows Mar 11–12, then 60°F. Over rows, the mean first reaches 50°F
    // on Mar 16 (three 40s and four 60s); over calendar days it would on Mar 15.
    let mut days: Vec<(NaiveDate, f64)> = (1..=10).map(|day| (d(2025, 3, day), 40.0)).collect();
    days.extend((13..=31).map(|day| (d(2025, 3, day), 60.0)));
    let found = find_threshold_crossings(2025, &rows(&days));
    let c = found
        .iter()
        .find(|c| c.threshold_name == "soil_50f_rising")
        .expect("50°F rising");
    assert_eq!(c.crossing_date, d(2025, 3, 16));
    assert!((c.avg_soil_temp_f - 360.0 / 7.0).abs() < 1e-9);

    let six: Vec<(NaiveDate, f64)> = (1..=6).map(|day| (d(2025, 5, day), 70.0)).collect();
    assert!(find_threshold_crossings(2025, &rows(&six)).is_empty());
}
