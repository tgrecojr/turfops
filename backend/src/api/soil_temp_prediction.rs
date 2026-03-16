use crate::error::TurfOpsError;
use crate::logic::soil_temp_prediction;
use crate::models::soil_temp_prediction::SoilTempForecast;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};

/// GET /api/v1/soil-temp-forecast
/// Fetches 30-day paired NOAA data, fits model, applies to OWM forecast.
pub async fn get_soil_temp_forecast(
    State(state): State<AppState>,
) -> Result<Json<SoilTempForecast>, TurfOpsError> {
    let service = state.sync_service.read().await;

    // We need: SoilData client for paired data, current soil temp, and forecast
    let soildata = service
        .soildata_client()
        .ok_or_else(|| TurfOpsError::DataSourceUnavailable("SoilData not configured".into()))?;

    // Fetch 30 days of daily paired (air_temp, soil_temp) averages from NOAA
    let now = Utc::now();
    let thirty_days_ago = now - Duration::days(30);
    let daily_pairs = soildata
        .fetch_daily_paired_averages(thirty_days_ago, now)
        .await?;

    if daily_pairs.len() < 14 {
        return Err(TurfOpsError::InvalidData(
            "Insufficient paired data for model fitting (need at least 14 days)".into(),
        ));
    }

    // Get current environmental summary for soil temp + forecast
    drop(service);
    let mut service = state.sync_service.write().await;
    let summary = service.get_or_refresh().await?;

    let current_soil_temp = summary
        .current
        .as_ref()
        .and_then(|c| c.soil_temp_10_f)
        .ok_or_else(|| TurfOpsError::InvalidData("Current soil temperature unavailable".into()))?;

    // Extract recent daily air temps from paired data
    let recent_daily_air: Vec<(chrono::NaiveDate, f64)> =
        daily_pairs.iter().map(|(d, air, _)| (*d, *air)).collect();

    // Extract forecast daily avg air temps from OWM
    let forecast_daily_air: Vec<(chrono::NaiveDate, f64)> = summary
        .forecast
        .as_ref()
        .map(|f| {
            f.daily_summary
                .iter()
                .map(|d| {
                    let avg_temp = (d.high_temp_f + d.low_temp_f) / 2.0;
                    (d.date, avg_temp)
                })
                .collect()
        })
        .unwrap_or_default();

    if forecast_daily_air.is_empty() {
        return Err(TurfOpsError::DataSourceUnavailable(
            "Weather forecast not available".into(),
        ));
    }

    let forecast = soil_temp_prediction::build_forecast(
        &daily_pairs,
        &recent_daily_air,
        &forecast_daily_air,
        current_soil_temp,
    )
    .ok_or_else(|| {
        TurfOpsError::InvalidData("Could not build prediction model (poor data correlation)".into())
    })?;

    Ok(Json(forecast))
}
