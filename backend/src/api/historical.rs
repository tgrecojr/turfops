use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{HistoricalData, TimeSeriesPoint};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HistoricalQuery {
    pub range: Option<String>,
}

/// GET /api/v1/historical?range=7d
/// Returns time-series environmental data for the requested range.
/// Supported ranges: 7d, 30d, 90d
pub async fn get_historical(
    State(state): State<AppState>,
    Query(params): Query<HistoricalQuery>,
) -> Result<Json<HistoricalData>, TurfOpsError> {
    let range_str = params.range.as_deref().unwrap_or("7d");
    let now = Utc::now();

    let (start, downsample_interval) = match range_str {
        "7d" => (now - Duration::days(7), 1),    // Every reading
        "30d" => (now - Duration::days(30), 6),  // ~every 30min (6 * 5min)
        "90d" => (now - Duration::days(90), 24), // ~every 2hr (24 * 5min)
        _ => {
            return Err(TurfOpsError::InvalidData(
                "Invalid range. Use 7d, 30d, or 90d".into(),
            ));
        }
    };

    // Query the environmental_cache table (local DB, up to 90 days retention)
    let readings = queries::get_environmental_cache_range(&state.pool, start, now).await?;

    // Build time-series, downsampling for longer ranges
    let mut soil_temp_10_f = Vec::new();
    let mut ambient_temp_f = Vec::new();
    let mut humidity_percent = Vec::new();
    let mut soil_moisture_10 = Vec::new();
    let mut precipitation_mm = Vec::new();

    for (i, reading) in readings.iter().enumerate() {
        if i % downsample_interval != 0 {
            continue;
        }

        if let Some(v) = reading.soil_temp_10_f {
            soil_temp_10_f.push(TimeSeriesPoint {
                timestamp: reading.timestamp,
                value: v,
            });
        }
        if let Some(v) = reading.ambient_temp_f {
            ambient_temp_f.push(TimeSeriesPoint {
                timestamp: reading.timestamp,
                value: v,
            });
        }
        if let Some(v) = reading.humidity_percent {
            humidity_percent.push(TimeSeriesPoint {
                timestamp: reading.timestamp,
                value: v,
            });
        }
        if let Some(v) = reading.soil_moisture_10 {
            soil_moisture_10.push(TimeSeriesPoint {
                timestamp: reading.timestamp,
                value: v,
            });
        }
        if let Some(v) = reading.precipitation_mm {
            if v >= 0.0 {
                precipitation_mm.push(TimeSeriesPoint {
                    timestamp: reading.timestamp,
                    value: v,
                });
            }
        }
    }

    // Add GDD accumulation from gdd_daily cache
    let gdd_start_date = start.date_naive();
    let gdd_end_date = now.date_naive();
    let gdd_records =
        queries::get_gdd_daily_range(&state.pool, gdd_start_date, gdd_end_date).await?;

    let gdd_accumulation: Vec<TimeSeriesPoint> = gdd_records
        .iter()
        .map(|d| TimeSeriesPoint {
            timestamp: d.date.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            value: d.cumulative_gdd_base50,
        })
        .collect();

    Ok(Json(HistoricalData {
        range: range_str.to_string(),
        soil_temp_10_f,
        ambient_temp_f,
        humidity_percent,
        soil_moisture_10,
        precipitation_mm,
        gdd_accumulation,
    }))
}
