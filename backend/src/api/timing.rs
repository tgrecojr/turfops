use crate::datasources::weather::ClimateDay;
use crate::datasources::WeatherLakeClient;
use crate::db::queries;
use crate::error::TurfOpsError;
use crate::logic::timing::{self, context, Assessment};
use crate::models::seasonal_plan::PlannedActivity;
use crate::models::timing::TimingResponse;
use crate::models::{LawnProfile, Recommendation};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{Datelike, Duration, Local, NaiveDate, Utc};
use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

/// The climate record spans 15 years of hourly soil data, and the lake only gains a
/// day at a time, so one read serves an hour of requests.
const CLIMATE_CACHE_TTL: StdDuration = StdDuration::from_secs(3600);
/// Application history window: reaches the previous July from any date, which is as
/// far back as a window's done/conflict spans look.
const HISTORY_LOOKBACK_DAYS: i64 = 550;
/// Fewer complete years than this and the typical dates are flagged as rough.
const MIN_SOLID_HISTORY_YEARS: usize = 5;

/// GET /api/v1/timing-windows
/// Seeding and pre-emergent windows from the station's 5 cm soil record, freeze
/// climatology, the soil forecast and the application log.
pub async fn get_timing_windows(
    State(state): State<AppState>,
) -> Result<Json<TimingResponse>, TurfOpsError> {
    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
    Ok(Json(compute(&state, &profile).await?))
}

async fn climate_days(
    state: &AppState,
    client: &WeatherLakeClient,
    start: NaiveDate,
) -> Result<Arc<Vec<ClimateDay>>, TurfOpsError> {
    if let Some((read_at, days)) = state.climate_cache.read().await.as_ref() {
        if read_at.elapsed() < CLIMATE_CACHE_TTL {
            return Ok(days.clone());
        }
    }
    let days = Arc::new(client.fetch_climate_days(start).await?);
    *state.climate_cache.write().await = Some((Instant::now(), days.clone()));
    Ok(days)
}

async fn compute(state: &AppState, profile: &LawnProfile) -> Result<TimingResponse, TurfOpsError> {
    let (client, forecast) = {
        let mut service = state.sync_service.write().await;
        let summary = service.get_or_refresh().await?;
        (service.weather_client().cloned(), summary.forecast)
    };
    let client = client.ok_or_else(|| {
        TurfOpsError::DataSourceUnavailable("weather data lake not configured".into())
    })?;

    let today = Local::now().date_naive();
    let start = NaiveDate::from_ymd_opt(today.year() - timing::HISTORY_YEARS, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData("Invalid history start".into()))?;
    let days = climate_days(state, &client, start).await?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let history = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        today - Duration::days(HISTORY_LOOKBACK_DAYS),
        today + Duration::days(1),
    )
    .await?;

    let daily_forecast = forecast
        .as_ref()
        .map(|f| f.daily_summary.as_slice())
        .unwrap_or_default();
    let assessment = timing::assess(&timing::Inputs {
        today,
        days: &days,
        forecast: daily_forecast,
        grass: profile.grass_type,
        history: &history,
    });

    let data_notes = data_notes(&assessment, profile, forecast.is_some());
    Ok(TimingResponse {
        generated_at: Utc::now(),
        today,
        station: format!("NOAA USCRN station {}", client.station_wbanno()),
        context: context::build_context(&days, &assessment.history_years, daily_forecast),
        series: context::build_series(&assessment.smoothed, today, &assessment.history_years),
        soil: assessment.soil,
        freeze: assessment.freeze,
        windows: assessment.windows,
        history_years: assessment.history_years,
        data_notes,
    })
}

/// Pre-emergent and seeding recommendations for the feed/dashboard. Like disease risk,
/// these are additive: a lake outage degrades to none rather than failing the feed.
pub(crate) async fn recommendations(
    state: &AppState,
    profile: &LawnProfile,
) -> Vec<Recommendation> {
    match compute(state, profile).await {
        Ok(response) => timing::recommendations::to_recommendations(
            &response.windows,
            &response.soil,
            profile,
            response.today,
        ),
        Err(e) => {
            tracing::warn!("Timing windows unavailable for recommendations: {}", e);
            Vec::new()
        }
    }
}

/// Seasonal-plan activities for `year` from the timing windows. Empty (so the plan
/// keeps its own versions) when the lake is unavailable.
pub(crate) async fn plan_activities(
    state: &AppState,
    profile: &LawnProfile,
    year: i32,
) -> Vec<PlannedActivity> {
    match plan_for(state, profile, year).await {
        Ok(activities) => activities,
        Err(e) => {
            tracing::warn!("Timing windows unavailable for the seasonal plan: {}", e);
            Vec::new()
        }
    }
}

async fn plan_for(
    state: &AppState,
    profile: &LawnProfile,
    year: i32,
) -> Result<Vec<PlannedActivity>, TurfOpsError> {
    let client = state
        .sync_service
        .read()
        .await
        .weather_client()
        .cloned()
        .ok_or_else(|| {
            TurfOpsError::DataSourceUnavailable("weather data lake not configured".into())
        })?;
    let today = Local::now().date_naive();
    let start = NaiveDate::from_ymd_opt(today.year() - timing::HISTORY_YEARS, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData("Invalid history start".into()))?;
    let days = climate_days(state, &client, start).await?;

    // The log spans that decide done/blocked reach from the previous mid-November
    // (dormant seeding) into the following February.
    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
    let invalid_year = || TurfOpsError::InvalidData(format!("Invalid year: {year}"));
    let applications = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        NaiveDate::from_ymd_opt(year - 1, 11, 1).ok_or_else(invalid_year)?,
        NaiveDate::from_ymd_opt(year + 1, 3, 1).ok_or_else(invalid_year)?,
    )
    .await?;

    let data = timing::SeasonData::build(&days, &[], today.year());
    let season = data.season(&days, today);
    Ok(timing::plan::activities(
        &season,
        profile.grass_type,
        &applications,
        year,
    ))
}

/// Plain-language caveats about the inputs behind this response.
fn data_notes(assessment: &Assessment, profile: &LawnProfile, has_forecast: bool) -> Vec<String> {
    let mut notes = Vec::new();
    if !assessment.data_fresh {
        notes.push(match assessment.soil.as_of {
            Some(date) => format!(
                "Station soil data ends {date}. Until it catches up, windows follow their \
                 typical dates instead of this season's soil."
            ),
            None => "No station soil data is available; windows follow their typical dates.".into(),
        });
    }
    let years = assessment.history_years.len();
    if years < MIN_SOLID_HISTORY_YEARS {
        notes.push(format!(
            "Only {years} complete year(s) of station history — typical dates are rough."
        ));
    }
    if !has_forecast {
        notes.push("Forecast unavailable — no soil outlook or weather alerts.".into());
    }
    if !profile.grass_type.is_cool_season() {
        notes.push("Seeding windows cover cool-season turf only.".into());
    }
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timing::{FreezeDates, SoilNow};

    fn assessment(data_fresh: bool, years: usize) -> Assessment {
        Assessment {
            windows: Vec::new(),
            soil: SoilNow {
                avg_5day_f: Some(66.0),
                as_of: NaiveDate::from_ymd_opt(2026, 9, 10),
                depth_cm: 5,
            },
            freeze: FreezeDates {
                last_spring: None,
                first_fall: None,
                last_spring_this_year: None,
                first_fall_this_year: None,
            },
            history_years: (0..years as i32).map(|i| 2015 + i).collect(),
            smoothed: Vec::new(),
            data_fresh,
        }
    }

    #[test]
    fn no_notes_when_inputs_are_healthy() {
        let notes = data_notes(&assessment(true, 10), &LawnProfile::default(), true);
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn notes_flag_stale_soil_thin_history_and_missing_forecast() {
        let notes = data_notes(&assessment(false, 2), &LawnProfile::default(), false);
        assert_eq!(notes.len(), 3);
        assert!(notes[0].contains("2026-09-10"));
        assert!(notes[1].contains("2 complete year"));
    }
}
