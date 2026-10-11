//! The plan's fall activities from 10 cm falling crossings.

use super::{aggregate_crossings, compute_status, confidence_from_count, fed, logged, make_window};
use crate::models::seasonal_plan::*;
use crate::models::{Application, ApplicationType};
use chrono::NaiveDate;

pub(super) fn fall_activities(
    year: i32,
    crossings: &[ThresholdCrossing],
    applications: &[Application],
    today: NaiveDate,
) -> Vec<PlannedActivity> {
    let mut activities = Vec::new();

    // 5. Core Aeration (fall — soil falling through 65°F → 50°F)
    if let Some(start) = aggregate_crossings(crossings, "soil_65f_falling", year) {
        let end = aggregate_crossings(crossings, "soil_50f_falling", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(30));

        let completed = logged(applications, &[ApplicationType::Aeration], year, 1..=12);

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

        let completed = logged(applications, &[ApplicationType::Overseed], year, 1..=12);

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

        let completed = fed(applications, year, 9..=9);

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

        let completed = fed(applications, year, 10..=10);

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

        let completed = fed(applications, year, 11..=12);

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

        let completed = logged(applications, &[ApplicationType::PostEmergent], year, 9..=12);

        // Herbicide on young seedlings: step aside for seed sown in the 60 days before the
        // window opens. Seed sown once it is open leaves the window as planned.
        // @spec PLAN-ACT-018
        let seed_window = start.median - chrono::Duration::days(60)..start.median;
        let recently_overseeded = applications.iter().any(|a| {
            a.is_turf()
                && a.application_type == ApplicationType::Overseed
                && seed_window.contains(&a.application_date)
        });

        if completed || !recently_overseeded {
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

    activities
}
