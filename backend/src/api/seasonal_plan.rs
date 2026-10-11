use crate::db::{plant_queries, queries};
use crate::error::TurfOpsError;
use crate::logic::plant_maintenance::{build_plant_activities, plant_log_range};
use crate::logic::rules::plan_state::{season_states, SeasonInputs, SeasonState};
use crate::logic::seasonal_plan::{analyze_year, apply_rule_states, build_seasonal_plan};
use crate::models::seasonal_plan::SeasonalPlan;
use crate::models::LawnProfile;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
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
    let today = state.clock.today();
    let year = params.year.unwrap_or_else(|| today.year());
    let settled_years = fill_crossing_cache(&state, today).await?;
    let all_crossings = queries::get_threshold_crossings(&state.pool).await?;

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
        &settled_years,
        &applications,
        profile.grass_type,
        today,
    );
    if year == today.year() {
        let states = rule_states(&state, &profile, today).await;
        apply_rule_states(&mut plan.activities, states.as_deref());
    }

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
    // A plant window can open late in the previous year (dormant pruning, Dec → Feb) or run
    // into the next; read the log from Oct 1 before through the end of the latest window.
    // @spec PLANT-PLAN-005
    let (plant_log_start, plant_log_end) = plant_log_range(&plants, year)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;
    let plant_applications = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        plant_log_start,
        plant_log_end,
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

/// Bring the 10 cm crossing cache up to date for the configured station and return its
/// settled years. Every unsettled year from ten years back through this one is re-read
/// from the lake, newest first, so the lake's newest date is known before an older year is
/// judged settled. Each computed year gets a marker, even with no crossings. A failed read
/// or write keeps that year's cached rows and moves on. Without a lake the cache is used
/// as it is: every settled marker it holds (only one station's survive a fill).
// @spec PLAN-CACHE-002, PLAN-CACHE-003, PLAN-CACHE-005, PLAN-CACHE-006, PLAN-CACHE-007, PLAN-CACHE-008
async fn fill_crossing_cache(state: &AppState, today: NaiveDate) -> Result<Vec<i32>, TurfOpsError> {
    let sync = state.sync_service.read().await;
    let Some(client) = sync.weather_client() else {
        return queries::get_cached_settled_years(&state.pool).await;
    };
    let station = client.station_wbanno();
    queries::delete_crossings_for_other_stations(&state.pool, station).await?;
    let settled = queries::get_settled_crossing_years(&state.pool, station).await?;
    let current_year = today.year();
    let mut lake_latest: Option<NaiveDate> = None;
    for fill_year in (current_year - 10..=current_year)
        .rev()
        .filter(|y| !settled.contains(y))
    {
        let (Some(start), Some(end)) = (
            Utc.with_ymd_and_hms(fill_year, 1, 1, 0, 0, 0).single(),
            Utc.with_ymd_and_hms(fill_year, 12, 31, 23, 59, 59).single(),
        ) else {
            continue;
        };
        let daily_temps = match client.fetch_daily_soil_temp_averages(start, end).await {
            Ok(daily_temps) => daily_temps,
            Err(e) => {
                tracing::warn!(
                    year = fill_year,
                    "Failed to read soil temps for the seasonal plan: {e}"
                );
                continue;
            }
        };
        lake_latest = lake_latest.max(daily_temps.last().map(|d| d.date));
        let analysis = analyze_year(fill_year, &daily_temps, today, lake_latest);
        if let Err(e) = queries::replace_threshold_crossings(
            &state.pool,
            fill_year,
            station,
            &analysis.crossings,
            analysis.settled,
        )
        .await
        {
            tracing::warn!(year = fill_year, "Failed to cache threshold crossings: {e}");
        }
    }
    queries::get_settled_crossing_years(&state.pool, station).await
}

/// The current season's rule states, judged on the same feed, conditions and log as the
/// recommendations. `None` when the feed cannot be assembled (the plan then marks the
/// covered activities as estimates).
// @spec PLAN-ACT-021, PLAN-ACT-022
async fn rule_states(
    state: &AppState,
    profile: &LawnProfile,
    today: NaiveDate,
) -> Option<Vec<(&'static str, SeasonState)>> {
    let summary = {
        let mut service = state.sync_service.write().await;
        service.get_or_refresh().await
    };
    let summary = summary
        .inspect_err(|e| tracing::warn!("Seasonal plan: no conditions for rule states: {e}"))
        .ok()?;
    let (feed, history) = super::recommendation_feed::all_with_history(state, profile, &summary)
        .await
        .inspect_err(|e| tracing::warn!("Seasonal plan: no feed for rule states: {e}"))
        .ok()?;
    Some(season_states(&SeasonInputs {
        today,
        env: &summary,
        profile,
        history: &history,
        feed: &feed,
    }))
}
