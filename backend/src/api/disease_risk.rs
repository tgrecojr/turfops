use crate::db::{product_queries, queries};
use crate::error::TurfOpsError;
use crate::logic::disease::{self, weather_days};
use crate::models::{
    DailyWeather, DiseaseRiskResponse, LawnProfile, ProductCategory, Recommendation,
    ShelfFungicide, ShelfProduct, StockStatus,
};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{Duration, NaiveDate, Utc};

/// GET /api/v1/disease-risk
/// Per-disease risk from published/heuristic weather models, each with a management
/// plan driven by the lawn's application history.
pub async fn get_disease_risk(
    State(state): State<AppState>,
) -> Result<Json<DiseaseRiskResponse>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
    Ok(Json(compute(&state, &profile).await?))
}

/// Disease recommendations for the feed/dashboard. Disease risk is additive there, so
/// a lake or forecast outage degrades to "no disease alerts" rather than failing.
pub(crate) async fn recommendations(
    state: &AppState,
    profile: &LawnProfile,
) -> Vec<Recommendation> {
    match compute(state, profile).await {
        Ok(response) => disease::recommendations::to_recommendations(&response.diseases),
        Err(e) => {
            tracing::warn!("Disease risk unavailable for recommendations: {}", e);
            Vec::new()
        }
    }
}

/// Observed days come from the data lake's hourly silver layer, today's remainder and
/// the outlook from the OpenWeatherMap forecast, and lawn history (overseeding,
/// nitrogen, fungicides) from the applications log.
async fn compute(
    state: &AppState,
    profile: &LawnProfile,
) -> Result<DiseaseRiskResponse, TurfOpsError> {
    let (client, forecast) = {
        let mut service = state.sync_service.write().await;
        let summary = service.get_or_refresh().await?;
        (service.weather_client().cloned(), summary.forecast)
    };
    let client = client.ok_or_else(|| {
        TurfOpsError::DataSourceUnavailable("weather data lake not configured".into())
    })?;

    // The lake's local date can differ from the server's, so anchor the fetch a day early.
    let fetch_start = Utc::now().date_naive() - Duration::days(disease::LOOKBACK_DAYS + 1);
    let lake = client.fetch_daily_disease_inputs(fetch_start).await?;
    let today = (Utc::now().naive_utc() + Duration::minutes(lake.utc_offset_minutes as i64)).date();

    let forecast_days = forecast
        .as_ref()
        .map(|f| weather_days::forecast_days(&f.hourly, lake.utc_offset_minutes))
        .unwrap_or_default();
    let series = weather_days::build_series(lake.days, forecast_days);

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let history = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        today - Duration::days(365),
        today + Duration::days(1),
    )
    .await?;
    let mut ctx = disease::context_from_history(&history, today);
    ctx.shelf = product_queries::list_products_for_profile(
        &state.pool,
        profile_id,
        Some(ProductCategory::Fungicide),
        false,
    )
    .await?
    .iter()
    .filter(|p| p.stock_status != StockStatus::Out)
    .filter_map(|p| {
        Some(ShelfFungicide {
            product: ShelfProduct::of(p)?,
            classes: p.facts.frac_classes.clone(),
        })
    })
    .collect();

    let mut data_notes = data_notes(&series, today, forecast.is_some());
    // Every tracked disease is a cool-season turf disease.
    let diseases = if profile.grass_type.is_cool_season() {
        disease::assess_all(&series, today, &ctx)
    } else {
        data_notes.push("Disease models cover cool-season turf only.".into());
        Vec::new()
    };

    Ok(DiseaseRiskResponse {
        generated_at: Utc::now(),
        today,
        station: format!("NOAA USCRN station {}", client.station_wbanno()),
        diseases,
        data_notes,
    })
}

/// Plain-language caveats about the inputs behind this response.
fn data_notes(series: &[DailyWeather], today: NaiveDate, has_forecast: bool) -> Vec<String> {
    let mut notes = Vec::new();
    let latest_observed = series
        .iter()
        .filter(|d| !d.is_forecast && d.hours_covered >= disease::MIN_USABLE_HOURS)
        .map(|d| d.date)
        .max();
    match latest_observed {
        None => notes.push("No recent station observations are available.".into()),
        Some(date) if date < today - Duration::days(1) => notes.push(format!(
            "Station observations end {date}; more recent days rely on the forecast."
        )),
        Some(_) => {}
    }

    let today_usable = series
        .iter()
        .any(|d| d.date == today && d.hours_covered >= disease::MIN_USABLE_HOURS);
    if !today_usable {
        notes.push(
            "Station data lags by up to a day and too little of today remains in the forecast, \
             so current risk reflects the most recent complete day."
                .into(),
        );
    }
    if !has_forecast {
        notes.push("Forecast unavailable — no outlook days are shown.".into());
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};

    #[test]
    fn no_notes_when_today_is_covered() {
        let today = date(9, 20);
        let mut series = run_of(today, 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        series[2].is_forecast = true;
        assert!(data_notes(&series, today, true).is_empty());
    }

    #[test]
    fn notes_flag_stale_lake_thin_today_and_missing_forecast() {
        let today = date(9, 20);
        let series = run_of(date(9, 17), 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        let notes = data_notes(&series, today, false);
        assert_eq!(notes.len(), 3);
        assert!(notes[0].contains("2026-09-17"));
    }
}
