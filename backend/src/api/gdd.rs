use crate::db::queries;
use crate::error::TurfOpsError;
use crate::logic::gdd;
use crate::models::GddSummary;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GddQuery {
    pub year: Option<i32>,
}

/// GET /api/v1/gdd?year=2026
/// Returns GDD accumulation data for the requested year (defaults to current year).
/// Lazily fills the gdd_daily cache from NOAA SoilData on first request.
pub async fn get_gdd(
    State(state): State<AppState>,
    Query(params): Query<GddQuery>,
) -> Result<Json<GddSummary>, TurfOpsError> {
    let year = params.year.unwrap_or_else(|| Local::now().year());
    let jan1 = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;
    let today = Local::now().date_naive();

    // Check what's already cached
    let latest_cached = queries::get_latest_gdd_date(&state.pool).await?;

    // Determine if we need to fill gaps
    let needs_fill = match latest_cached {
        None => true,
        Some(cached_date) => {
            // Need fill if cached date is before yesterday (allow 1 day lag for incomplete data)
            cached_date < today - Duration::days(1) && cached_date.year() <= year
        }
    };

    if needs_fill {
        // Fetch hourly data from SoilData to compute daily high/low
        let service = state.sync_service.read().await;
        if let Some(client) = service.soildata_client() {
            let fill_start = match latest_cached {
                Some(d) if d.year() == year => d + Duration::days(1),
                _ => jan1,
            };
            let fill_end_date = today - Duration::days(1); // Yesterday (today may be incomplete)

            if fill_start <= fill_end_date {
                let start_dt = fill_start.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end_dt = fill_end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();

                match client.fetch_range(start_dt, end_dt).await {
                    Ok(readings) => {
                        let daily_highs_lows = gdd::compute_daily_highs_lows(&readings);
                        let daily_records = gdd::accumulate_gdd(&daily_highs_lows);

                        // If we're filling mid-year, adjust cumulative to include previously cached data
                        let base_cumulative = if let Some(cached_date) = latest_cached {
                            if cached_date.year() == year {
                                let cached = queries::get_gdd_daily_range(
                                    &state.pool,
                                    cached_date,
                                    cached_date,
                                )
                                .await?;
                                cached
                                    .first()
                                    .map(|d| d.cumulative_gdd_base50)
                                    .unwrap_or(0.0)
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };

                        for mut record in daily_records {
                            record.cumulative_gdd_base50 += base_cumulative;
                            if let Err(e) = queries::upsert_gdd_daily(&state.pool, &record).await {
                                tracing::warn!("Failed to cache GDD for {}: {}", record.date, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch SoilData for GDD fill: {}", e);
                    }
                }
            }
        }
    }

    // Read the full year from cache
    let end_date = if year == today.year() {
        today
    } else {
        NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or(today)
    };

    let cached_records = queries::get_gdd_daily_range(&state.pool, jan1, end_date).await?;
    let summary = gdd::build_gdd_summary(year, &cached_records);

    Ok(Json(summary))
}
