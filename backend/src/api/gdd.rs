use crate::error::{Source, TurfOpsError};
use crate::logic::gdd;
use crate::models::GddSummary;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GddQuery {
    pub year: Option<i32>,
}

// @spec RULE-GDD-006
/// GET /api/v1/gdd?year=2026
/// Returns GDD accumulation data for the requested year (defaults to current year).
/// Reads the gold daily layer's precomputed `gdd50` from the data lake and accumulates
/// the year-to-date running total on the fly (no DB cache).
pub async fn get_gdd(
    State(state): State<AppState>,
    Query(params): Query<GddQuery>,
) -> Result<Json<GddSummary>, TurfOpsError> {
    let today = state.clock.today();
    let year = params.year.unwrap_or_else(|| today.year());
    let jan1 = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;

    // For the current year, stop at today; otherwise the full calendar year.
    let end_date = if year == today.year() {
        today
    } else {
        NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or(today)
    };

    // @spec RULE-GDD-007
    // Without the lake there is no GDD record: say so rather than report a zero total.
    let client = state
        .sync_service
        .read()
        .await
        .weather_client()
        .cloned()
        .ok_or_else(|| TurfOpsError::not_configured(Source::WeatherLake))?;
    let records = if jan1 <= end_date {
        gdd::accumulate_daily_gdd(&client.fetch_daily_gdd(jan1, end_date).await?)
    } else {
        Vec::new()
    };

    let summary = gdd::build_gdd_summary(year, &records);
    Ok(Json(summary))
}

#[cfg(test)]
mod handler_tests;
