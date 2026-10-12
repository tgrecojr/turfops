use crate::db::{product_queries, queries};
use crate::error::{Source, TurfOpsError};
use crate::logic::disease::{self, weather_days};
use crate::models::{
    DailyWeather, Disease, DiseaseContext, DiseaseRiskResponse, LawnProfile, ProductCategory,
    Recommendation, ShelfFungicide, ShelfProduct, StockStatus,
};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{Duration, NaiveDate, Utc};

// @spec DIS-MODEL-API-001, DIS-MODEL-API-002
/// GET /api/v1/disease-risk
/// Per-disease risk from published/heuristic weather models, each with a management
/// plan driven by the lawn's application history.
pub async fn get_disease_risk(
    State(state): State<AppState>,
) -> Result<Json<DiseaseRiskResponse>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
    let (response, _) = compute(&state, &profile).await?;
    Ok(Json(response))
}

// @spec FEED-PIPE-015, DIS-MGMT-REC-005
/// Disease recommendations for the feed/dashboard. Disease risk is additive there, so
/// a lake or forecast outage degrades to "no disease alerts" rather than failing.
pub(crate) async fn recommendations(
    state: &AppState,
    profile: &LawnProfile,
) -> Vec<Recommendation> {
    match compute(state, profile).await {
        Ok((response, ctx)) => {
            disease::recommendations::to_recommendations(&response.diseases, &ctx)
        }
        Err(e) => {
            tracing::warn!("Disease risk unavailable for recommendations: {}", e);
            Vec::new()
        }
    }
}

/// Observed days come from the data lake's hourly silver layer, today's remainder and
/// the outlook from the OpenWeatherMap forecast, and lawn history (overseeding,
/// nitrogen, fungicides) from the applications log. Returns the context alongside the
/// response so the recommendation bridge can see the same history.
async fn compute(
    state: &AppState,
    profile: &LawnProfile,
) -> Result<(DiseaseRiskResponse, DiseaseContext), TurfOpsError> {
    let (client, forecast) = {
        let mut service = state.sync_service.write().await;
        let summary = service.get_or_refresh().await?;
        (service.weather_client().cloned(), summary.forecast)
    };
    let client = client.ok_or_else(|| TurfOpsError::not_configured(Source::WeatherLake))?;

    // @spec DIS-MODEL-DAY-001
    // Lake days are keyed by the same Eastern calendar `TZ` gives, so no extra margin.
    let today = state.clock.today();
    let fetch_start = today - Duration::days(disease::LOOKBACK_DAYS);
    let lake = client.fetch_daily_disease_inputs(fetch_start).await?;

    let forecast_days = forecast
        .as_ref()
        .map(|f| weather_days::forecast_days(&f.hourly, &state.clock, lake.latest_observed))
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
    // @spec DIS-MGMT-PICK-005
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

    // Every tracked disease is a cool-season turf disease.
    let cool_season = profile.grass_type.is_cool_season();
    let diseases = if cool_season {
        disease::assess_all(&series, today, &ctx)
    } else {
        Vec::new()
    };
    let omitted: Vec<Disease> = if cool_season {
        Disease::ALL
            .into_iter()
            .filter(|d| !diseases.iter().any(|r| r.disease == *d))
            .collect()
    } else {
        Vec::new()
    };
    let mut data_notes = data_notes(&series, today, forecast.is_some(), &omitted);
    if !cool_season {
        data_notes.push("Disease models cover cool-season turf only.".into());
    }

    let response = DiseaseRiskResponse {
        generated_at: Utc::now(),
        today,
        station: format!("NOAA USCRN station {}", client.station_wbanno()),
        diseases,
        data_notes,
    };
    Ok((response, ctx))
}

// @spec DIS-MODEL-API-003
/// Plain-language caveats about the inputs behind this response.
fn data_notes(
    series: &[DailyWeather],
    today: NaiveDate,
    has_forecast: bool,
    omitted: &[Disease],
) -> Vec<String> {
    // @spec DIS-MODEL-API-004, DIS-MODEL-API-005
    // "Usable" is the headline's own rule (hours, dawn and afternoon), so a note explains
    // every fallback the headline makes.
    let mut notes = Vec::new();
    let latest_observed = series
        .iter()
        .filter(|d| !d.is_forecast && disease::is_usable(d))
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
        .any(|d| d.date == today && disease::is_usable(d));
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
    for d in omitted {
        notes.push(format!(
            "{} is left out: no recent day has enough station data to score it.",
            sentence_case(d.name())
        ));
    }
    notes
}

/// "Gray Leaf Spot" → "Gray leaf spot".
fn sentence_case(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(all(test, feature = "db-tests"))]
mod db_tests;
#[cfg(all(test, feature = "db-tests"))]
mod shelf_db_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::disease::test_support::{date, day, run_of};

    #[test]
    fn no_notes_when_today_is_covered() {
        let today = date(9, 20);
        let mut series = run_of(today, 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        series[2].is_forecast = true;
        assert!(data_notes(&series, today, true, &[]).is_empty());
    }

    // @spec DIS-MODEL-API-003
    #[test]
    fn notes_flag_stale_lake_thin_today_and_missing_forecast() {
        let today = date(9, 20);
        let series = run_of(date(9, 17), 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        let notes = data_notes(&series, today, false, &[]);
        assert_eq!(notes.len(), 3);
        assert!(notes[0].contains("2026-09-17"));
    }

    // @spec DIS-MODEL-API-004
    #[test]
    fn today_without_dawn_hours_is_not_usable_for_the_notes_either() {
        let today = date(9, 20);
        let mut series = run_of(today, 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        // Twelve hours of today, but none of them dawn: the headline falls back.
        series[2].hours_covered = 12.0;
        series[2].covers_dawn = false;
        let notes = data_notes(&series, today, true, &[]);
        assert!(
            notes.iter().any(|n| n.contains("most recent complete day")),
            "{notes:?}"
        );
    }

    // @spec DIS-MODEL-API-005
    #[test]
    fn a_disease_left_out_is_named() {
        let today = date(9, 20);
        let series = run_of(today, 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        let notes = data_notes(&series, today, true, &[Disease::GrayLeafSpot]);
        assert!(
            notes.iter().any(|n| n.contains("Gray leaf spot")),
            "{notes:?}"
        );
    }

    // @spec DIS-MODEL-API-003
    #[test]
    fn no_station_days_at_all_is_its_own_note() {
        let today = date(9, 20);
        let mut forecast_only = run_of(date(9, 22), 3, |d| day(d, 20.0, 15.0, 25.0, 80.0));
        for d in &mut forecast_only {
            d.is_forecast = true;
        }
        let notes = data_notes(&forecast_only, today, true, &[]);
        assert_eq!(notes, vec!["No recent station observations are available."]);
    }
}
