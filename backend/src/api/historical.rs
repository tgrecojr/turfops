use crate::error::TurfOpsError;
use crate::logic::gdd;
use crate::models::{HistoricalData, TimeSeriesPoint};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HistoricalQuery {
    pub range: Option<String>,
}

/// GET /api/v1/historical?range=7d
/// Returns time-series environmental data for the requested range, read directly from
/// the data lake (silver hourly observations + gold daily GDD).
/// Supported ranges: 7d, 30d, 90d
pub async fn get_historical(
    State(state): State<AppState>,
    Query(params): Query<HistoricalQuery>,
) -> Result<Json<HistoricalData>, TurfOpsError> {
    let range_str = params.range.as_deref().unwrap_or("7d");
    let now = Utc::now();

    // Silver is hourly, so downsample to keep point counts reasonable on longer ranges.
    let (start, downsample_interval) = match range_str {
        "7d" => (now - Duration::days(7), 1),    // ~168 hourly points
        "30d" => (now - Duration::days(30), 6),  // ~every 6h
        "90d" => (now - Duration::days(90), 24), // ~daily
        _ => {
            return Err(TurfOpsError::InvalidData(
                "Invalid range. Use 7d, 30d, or 90d".into(),
            ));
        }
    };

    let service = state.sync_service.read().await;
    let client = service.weather_client().ok_or_else(|| {
        TurfOpsError::DataSourceUnavailable("Weather data lake not configured".into())
    })?;

    // fetch_range returns newest-first; reverse to ascending for charting.
    let mut readings = client.fetch_range(start, now).await?;
    readings.reverse();

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

    // GDD accumulation from the gold daily layer. Accumulate from Jan 1 so the values
    // are true year-to-date (resetting on year boundaries), then keep only points
    // within the requested range.
    let start_date = start.date_naive();
    let accum_start = NaiveDate::from_ymd_opt(start.year(), 1, 1).unwrap_or(start_date);
    let gdd_records = gdd::accumulate_daily_gdd(
        &client
            .fetch_daily_gdd(accum_start, now.date_naive())
            .await?,
    );

    let gdd_accumulation: Vec<TimeSeriesPoint> = gdd_records
        .iter()
        .filter(|d| d.date >= start_date)
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
