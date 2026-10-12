//! A falling crossing the forecast recovers from is not a crossing.

use super::*;

fn day(d: u32, temp: f64) -> SoilTempPrediction {
    SoilTempPrediction {
        date: NaiveDate::from_ymd_opt(2026, 10, d).unwrap(),
        predicted_soil_temp_f: temp,
        confidence: PredictionConfidence::High,
        air_temp_used_f: temp - 5.0,
        source_description: "test".to_string(),
    }
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
}

fn named(crossings: &[ThresholdPrediction], name: &str) -> bool {
    crossings.iter().any(|c| c.threshold_name == name)
}

// @spec WX-OUTLOOK-CROSS-006
#[test]
fn a_dip_that_recovers_within_the_forecast_is_not_announced() {
    // 46 → 44 → 47: the soil ends above 45°F, so dormancy is not coming.
    let dip = predict_threshold_crossings(46.0, &[day(2, 44.0), day(3, 47.0)], today());
    assert!(!named(&dip, "Winterizer / Dormancy"), "{dip:?}");
    // 66 → 64 → 66: the either-direction 65°F threshold is not crossed either way.
    let peak = predict_threshold_crossings(66.0, &[day(2, 64.0), day(3, 66.0)], today());
    assert!(!named(&peak, "Active Growth Peak"), "{peak:?}");
}

// @spec WX-OUTLOOK-CROSS-006
#[test]
fn a_fall_that_holds_to_the_end_of_the_forecast_is_announced() {
    let holds = predict_threshold_crossings(46.0, &[day(2, 44.0), day(3, 43.0)], today());
    let dormancy = holds
        .iter()
        .find(|c| c.threshold_name == "Winterizer / Dormancy")
        .expect("held below 45°F");
    assert_eq!(dormancy.direction, CrossingDirection::Falling);
    assert_eq!(dormancy.estimated_crossing_date, Some(day(2, 0.0).date));
}
