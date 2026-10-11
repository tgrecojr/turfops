//! The plan's spring activities from 10 cm rising crossings.

use super::{
    aggregate_crossings, compute_status, confidence_from_count, fed, in_year, logged, make_window,
    make_window_with_dates,
};
use crate::models::seasonal_plan::*;
use crate::models::{Application, ApplicationType};
use chrono::NaiveDate;

pub(super) fn spring_activities(
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

        // A fall pre-emergent belongs to the fall window, not this one.
        let completed = logged(applications, &[ApplicationType::PreEmergent], year, 1..=6);

        activities.push(PlannedActivity {
            id: "pre_emergent".into(),
            name: "Pre-Emergent Herbicide".into(),
            category: "Weed Prevention".into(),
            description: "Apply pre-emergent before crabgrass germination. \
                Soil temp at 50-60°F is the target window."
                .into(),
            date_window: make_window(&start, end_date, end.as_ref()),
            status: compute_status(completed, start.median, end_date, today),
            status_estimated: false,
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
        let end = aggregate_crossings(crossings, "soil_65f_rising", year);
        let end_date = end
            .as_ref()
            .map(|e| e.median)
            .unwrap_or_else(|| start.median + chrono::Duration::days(21));

        let completed = fed(applications, year, 1..=5);

        activities.push(PlannedActivity {
            id: "spring_nitrogen".into(),
            name: "Spring Nitrogen".into(),
            category: "Fertilization".into(),
            description: "Light nitrogen application to fuel spring green-up. \
                Use slow-release to avoid surge growth."
                .into(),
            date_window: make_window(&start, end_date, end.as_ref()),
            status: compute_status(completed, start.median, end_date, today),
            status_estimated: false,
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

        let completed = logged(applications, &[ApplicationType::PostEmergent], year, 1..=5);

        activities.push(PlannedActivity {
            id: "spring_herbicide".into(),
            name: "Spring Broadleaf Herbicide".into(),
            category: "Weed Control".into(),
            description: "Target spring broadleaf weeds (dandelion, clover) \
                during active growth before heat stress."
                .into(),
            date_window: DateWindow {
                predicted_start: in_year(start_date, year),
                predicted_end: in_year(end_date, year),
                earliest_historical: Some(start_agg.earliest - chrono::Duration::days(7)),
                latest_historical: end.as_ref().map(|e| e.latest + chrono::Duration::days(7)),
                confidence: confidence_from_count(start_agg.sample_count),
            },
            status: compute_status(completed, start_date, end_date, today),
            status_estimated: false,
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

        let completed = logged(
            applications,
            &[ApplicationType::GrubControl, ApplicationType::Insecticide],
            year,
            1..=12,
        );

        activities.push(PlannedActivity {
            id: "grub_preventative".into(),
            name: "Grub Preventative".into(),
            category: "Pest Control".into(),
            description: "Preventative grub control during beetle egg-laying season. \
                Soil temp 60-75°F window."
                .into(),
            date_window: make_window_with_dates(start_date, end_date, &start, end.as_ref()),
            status: compute_status(completed, start_date, end_date, today),
            status_estimated: false,
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

    activities
}
