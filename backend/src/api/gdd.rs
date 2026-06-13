use crate::error::TurfOpsError;
use crate::logic::gdd;
use crate::models::GddSummary;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, Local, NaiveDate};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GddQuery {
    pub year: Option<i32>,
}

/// GET /api/v1/gdd?year=2026
/// Returns GDD accumulation data for the requested year (defaults to current year).
/// Reads the gold daily layer's precomputed `gdd50` from the data lake and accumulates
/// the year-to-date running total on the fly (no DB cache).
pub async fn get_gdd(
    State(state): State<AppState>,
    Query(params): Query<GddQuery>,
) -> Result<Json<GddSummary>, TurfOpsError> {
    let year = params.year.unwrap_or_else(|| Local::now().year());
    let jan1 = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;
    let today = Local::now().date_naive();

    // For the current year, stop at today; otherwise the full calendar year.
    let end_date = if year == today.year() {
        today
    } else {
        NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or(today)
    };

    let records = if jan1 <= end_date {
        let service = state.sync_service.read().await;
        match service.weather_client() {
            Some(client) => {
                let rows = client.fetch_daily_gdd(jan1, end_date).await?;
                gdd::accumulate_daily_gdd(&rows)
            }
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let summary = gdd::build_gdd_summary(year, &records);
    Ok(Json(summary))
}
