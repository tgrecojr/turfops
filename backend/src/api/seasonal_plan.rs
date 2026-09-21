use crate::db::{plant_queries, queries};
use crate::error::TurfOpsError;
use crate::logic::plant_maintenance::build_plant_activities;
use crate::logic::seasonal_plan::{build_seasonal_plan, find_threshold_crossings};
use crate::models::seasonal_plan::SeasonalPlan;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, Local, NaiveDate, TimeZone, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SeasonalPlanQuery {
    pub year: Option<i32>,
}

/// GET /api/v1/seasonal-plan?year=2026
///
/// Returns a seasonal plan for the requested year based on historical
/// soil temperature threshold crossings from NOAA data.
/// Lazily fills the threshold crossing cache from the weather data lake on first call.
pub async fn get_seasonal_plan(
    State(state): State<AppState>,
    Query(params): Query<SeasonalPlanQuery>,
) -> Result<Json<SeasonalPlan>, TurfOpsError> {
    let year = params.year.unwrap_or_else(|| Local::now().year());

    // Backfill threshold crossings from the data lake. Past years are computed once; the
    // year in progress (and last year, until it has been recomputed after it ended) is
    // recomputed on each load — a local parquet read — so its later crossings appear.
    let current_year = Local::now().year();
    let earliest_desired = current_year - 10; // Up to 10 years of history
    {
        let sync = state.sync_service.read().await;
        if let Some(client) = sync.weather_client() {
            let station = client.station_wbanno();
            queries::delete_crossings_for_other_stations(&state.pool, station).await?;
            let settled = queries::get_settled_crossing_years(&state.pool, station).await?;
            for fill_year in (earliest_desired..=current_year).filter(|y| !settled.contains(y)) {
                let start = Utc
                    .with_ymd_and_hms(fill_year, 1, 1, 0, 0, 0)
                    .single()
                    .unwrap_or_default();
                let end = Utc
                    .with_ymd_and_hms(fill_year, 12, 31, 23, 59, 59)
                    .single()
                    .unwrap_or_default();

                match client.fetch_daily_soil_temp_averages(start, end).await {
                    Ok(daily_temps) if daily_temps.len() >= 30 => {
                        let crossings = find_threshold_crossings(fill_year, &daily_temps);
                        if let Err(e) = queries::replace_threshold_crossings(
                            &state.pool,
                            fill_year,
                            station,
                            &crossings,
                        )
                        .await
                        {
                            tracing::warn!(
                                year = fill_year,
                                "Failed to cache threshold crossings: {}",
                                e
                            );
                        }
                    }
                    Ok(daily_temps) => {
                        tracing::debug!(
                            year = fill_year,
                            points = daily_temps.len(),
                            "Insufficient data for threshold analysis"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            year = fill_year,
                            "Failed to fetch soil temps for threshold analysis: {}",
                            e
                        );
                    }
                }
            }
        }
    }

    // Load all crossings (including newly cached ones)
    let all_crossings = queries::get_threshold_crossings(&state.pool).await?;

    let data_years = all_crossings
        .iter()
        .map(|c| c.year)
        .collect::<std::collections::HashSet<_>>()
        .len() as i32;

    // Get application history for the requested year
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let start_date = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;
    let end_date = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;

    let applications = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        start_date,
        end_date,
    )
    .await?;

    let mut plan = build_seasonal_plan(
        year,
        &all_crossings,
        &applications,
        data_years,
        profile.grass_type,
    );

    // Pre-emergent, seeding and aeration come from the timing windows (5 cm soil +
    // freeze dates) so the plan agrees with the Timing page. The plan's own 10 cm version
    // of every activity the windows answer for is dropped — also one the log has ruled out
    // (seed vs. pre-emergent), which must not come back. With the lake unavailable the
    // originals stay.
    if let Some(timing) = super::timing::plan_activities(&state, &profile, year).await {
        let owned = crate::logic::timing::plan::owned_ids(profile.grass_type);
        plan.activities.retain(|a| !owned.contains(&a.id.as_str()));
        plan.activities.extend(timing);
    }

    // Overlay plant-maintenance activities from the landscape feature.
    let plants = plant_queries::list_plants_for_profile(&state.pool, profile_id).await?;
    let today = Local::now().date_naive();
    // A plant window can open late in the previous year (dormant pruning, Dec → Feb), and
    // the job may have been done back then — look that far for its completion.
    let plant_log_start = NaiveDate::from_ymd_opt(year - 1, 10, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;
    let plant_applications = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        plant_log_start,
        end_date,
    )
    .await?;
    plan.activities.extend(build_plant_activities(
        &plants,
        &plant_applications,
        year,
        today,
    ));
    plan.activities
        .sort_by_key(|a| a.date_window.predicted_start);

    Ok(Json(plan))
}
