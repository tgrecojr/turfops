use crate::db::queries;
use crate::error::TurfOpsError;
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
/// Lazily fills the threshold crossing cache from SoilData on first call.
pub async fn get_seasonal_plan(
    State(state): State<AppState>,
    Query(params): Query<SeasonalPlanQuery>,
) -> Result<Json<SeasonalPlan>, TurfOpsError> {
    let year = params.year.unwrap_or_else(|| Local::now().year());

    // Load cached crossings
    let cached_years = queries::get_threshold_crossings_years(&state.pool).await?;

    // Determine which years need backfilling from SoilData
    let current_year = Local::now().year();
    let earliest_desired = current_year - 10; // Up to 10 years of history
    let years_to_fill: Vec<i32> = (earliest_desired..=current_year)
        .filter(|y| !cached_years.contains(y))
        .collect();

    // Backfill from SoilData if needed
    if !years_to_fill.is_empty() {
        let sync = state.sync_service.read().await;
        if let Some(client) = sync.soildata_client() {
            for fill_year in &years_to_fill {
                let start = Utc
                    .with_ymd_and_hms(*fill_year, 1, 1, 0, 0, 0)
                    .single()
                    .unwrap_or_default();
                let end = Utc
                    .with_ymd_and_hms(*fill_year, 12, 31, 23, 59, 59)
                    .single()
                    .unwrap_or_default();

                match client.fetch_daily_soil_temp_averages(start, end).await {
                    Ok(daily_temps) => {
                        if daily_temps.len() >= 30 {
                            let crossings = find_threshold_crossings(*fill_year, &daily_temps);
                            for crossing in &crossings {
                                if let Err(e) =
                                    queries::upsert_threshold_crossing(&state.pool, crossing).await
                                {
                                    tracing::warn!(
                                        year = fill_year,
                                        "Failed to cache threshold crossing: {}",
                                        e
                                    );
                                }
                            }
                            tracing::info!(
                                year = fill_year,
                                crossings = crossings.len(),
                                daily_points = daily_temps.len(),
                                "Cached threshold crossings for year"
                            );
                        } else {
                            tracing::debug!(
                                year = fill_year,
                                points = daily_temps.len(),
                                "Insufficient data for threshold analysis"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(year = fill_year, "Failed to fetch soil data: {}", e);
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

    let plan = build_seasonal_plan(year, &all_crossings, &applications, data_years);

    Ok(Json(plan))
}
