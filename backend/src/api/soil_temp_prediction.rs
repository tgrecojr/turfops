use crate::error::{Source, TurfOpsError};
use crate::logic::soil_temp_prediction;
use crate::models::soil_temp_prediction::SoilTempForecast;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{Duration, Utc};

// @spec WX-OUTLOOK-API-001, WX-OUTLOOK-API-002, WX-OUTLOOK-MODEL-004, WX-OUTLOOK-MODEL-005
/// GET /api/v1/soil-temp-forecast
/// Fetches 30-day paired NOAA data, fits model, applies to OWM forecast.
pub async fn get_soil_temp_forecast(
    State(state): State<AppState>,
) -> Result<Json<SoilTempForecast>, TurfOpsError> {
    let service = state.sync_service.read().await;

    // We need: weather lake client for paired data, current soil temp, and forecast
    let weather = service
        .weather_client()
        .ok_or_else(|| TurfOpsError::not_configured(Source::WeatherLake))?;

    // Fetch 30 days of daily paired (air_temp, soil_temp) averages from the gold layer
    let now = Utc::now();
    let thirty_days_ago = now - Duration::days(30);
    let daily_pairs = weather
        .fetch_daily_paired_averages(thirty_days_ago, now)
        .await?;

    if daily_pairs.len() < soil_temp_prediction::MIN_TRAINING_PAIRS {
        // @spec WX-OUTLOOK-API-003
        return Err(TurfOpsError::not_enough_data(Source::SoilOutlook));
    }

    // Get current environmental summary for soil temp + forecast
    drop(service);
    let mut service = state.sync_service.write().await;
    let summary = service.get_or_refresh().await?;

    let current_soil_temp = summary
        .current
        .as_ref()
        .and_then(|c| c.soil_temp_10_f)
        .ok_or_else(|| TurfOpsError::not_enough_data(Source::SoilOutlook))?;

    // Extract forecast daily avg air temps from OWM
    let forecast_daily_air: Vec<(chrono::NaiveDate, f64)> = summary
        .forecast
        .as_ref()
        .map(|f| {
            // Partial first/last days would enter the regression as a full-day mean.
            f.full_days()
                .map(|d| {
                    let avg_temp = (d.high_temp_f + d.low_temp_f) / 2.0;
                    (d.date, avg_temp)
                })
                .collect()
        })
        .unwrap_or_default();

    if forecast_daily_air.is_empty() {
        return Err(TurfOpsError::unavailable(
            Source::Forecast,
            "no full forecast days to drive the soil outlook",
        ));
    }

    let forecast = soil_temp_prediction::build_forecast(
        &daily_pairs,
        &forecast_daily_air,
        current_soil_temp,
        state.clock.today(),
    )
    .ok_or_else(|| TurfOpsError::not_enough_data(Source::SoilOutlook))?;

    Ok(Json(forecast))
}

#[cfg(test)]
mod tests;
