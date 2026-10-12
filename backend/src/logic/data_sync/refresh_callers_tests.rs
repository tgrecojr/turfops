//! Which handlers may refresh current conditions, read from the source itself.

use crate::test_support::{production_code, production_sources};

// @spec SYNC-REFRESH-005
#[test]
fn only_the_condition_reading_handlers_trigger_a_refresh() {
    let mut callers: Vec<String> = production_sources()
        .into_iter()
        .filter(|f| !f.ends_with("logic/data_sync.rs"))
        .filter(|f| production_code(f).contains(".get_or_refresh()"))
        .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    callers.sort();
    assert_eq!(
        callers,
        [
            "dashboard.rs",
            "disease_risk.rs",
            "environmental.rs",
            "inventory.rs",
            "recommendations.rs",
            "seasonal_plan.rs",
            "soil_temp_prediction.rs",
            "timing.rs",
        ]
    );
}
