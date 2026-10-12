mod memo;
#[cfg(test)]
mod memo_tests;

use crate::datasources::weather::ClimateRecord;
use crate::datasources::WeatherLakeClient;
use crate::db::queries;
use crate::error::{Source, TurfOpsError};
use crate::logic::timing::{self, context, Assessment};
use crate::models::seasonal_plan::PlannedActivity;
use crate::models::timing::TimingResponse;
use crate::models::{Application, DailyForecast, LawnProfile, Recommendation};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use std::sync::Arc;

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

/// First day of the climate record: Jan 1, `HISTORY_YEARS` before the lawn's current year.
pub(crate) fn climate_start(today: NaiveDate) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(today.year() - timing::HISTORY_YEARS, 1, 1)
}

/// The climate record and, when the lake could not be re-read, when the record served
/// in its place was read.
pub(crate) struct ClimateRead {
    pub record: Arc<ClimateRecord>,
    pub last_good_at: Option<chrono::DateTime<Utc>>,
}

/// The memoized climate record (see `climate_read`).
pub(crate) async fn climate_record(
    state: &AppState,
    client: &WeatherLakeClient,
) -> Result<Arc<ClimateRecord>, TurfOpsError> {
    Ok(climate_read(state, client).await?.record)
}

/// The memoized climate record, re-read from the lake when missing or an hour old. When
/// the re-read fails, the slot is left as it is and its record stands in while under a
/// day old; only with no such record does the error reach the caller.
// @spec SYNC-MEMO-001, SYNC-MEMO-002
pub(crate) async fn climate_read(
    state: &AppState,
    client: &WeatherLakeClient,
) -> Result<ClimateRead, TurfOpsError> {
    let slot = state.climate_cache.read().await.clone();
    if let Some((read_at, record)) = &slot {
        if memo::is_fresh(*read_at, Utc::now()) {
            return Ok(ClimateRead {
                record: record.clone(),
                last_good_at: None,
            });
        }
    }
    let start = climate_start(state.clock.today())
        .ok_or_else(|| TurfOpsError::InvalidData("Invalid history start".into()))?;
    match client.fetch_climate_record(start).await {
        Ok(record) => {
            let record = Arc::new(record);
            *state.climate_cache.write().await = Some((Utc::now(), record.clone()));
            Ok(ClimateRead {
                record,
                last_good_at: None,
            })
        }
        Err(e) => match slot.filter(|(read_at, _)| memo::may_stand_in(*read_at, Utc::now())) {
            Some((read_at, record)) => {
                tracing::warn!(%read_at, "Climate record re-read failed; serving the last good read: {e}");
                Ok(ClimateRead {
                    record,
                    last_good_at: Some(read_at),
                })
            }
            None => Err(e),
        },
    }
}

/// The lake client and the forecast's full days: the inputs the Timing page and the
/// seasonal plan's timing activities both evaluate the windows with.
async fn lake_and_forecast(
    state: &AppState,
) -> Result<(WeatherLakeClient, Option<Vec<DailyForecast>>), TurfOpsError> {
    let (client, forecast) = {
        let mut service = state.sync_service.write().await;
        let summary = service.get_or_refresh().await?;
        (service.weather_client().cloned(), summary.forecast)
    };
    let client = client.ok_or_else(|| TurfOpsError::not_configured(Source::WeatherLake))?;
    // Only full days: a partial day's (high + low) / 2 is an evening or a night, not a mean.
    let full_days = forecast.map(|f| f.full_days().cloned().collect());
    Ok((client, full_days))
}

async fn compute(state: &AppState, profile: &LawnProfile) -> Result<TimingResponse, TurfOpsError> {
    let (client, forecast) = lake_and_forecast(state).await?;

    // @spec TIME-ENG-039
    let climate = climate_read(state, &client).await?;
    let record = &climate.record;
    let today = state.clock.today();
    let days = &record.days;

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

    let full_days: Vec<DailyForecast> = forecast.clone().unwrap_or_default();
    let daily_forecast = full_days.as_slice();
    let assessment = timing::assess(&timing::Inputs {
        today,
        days,
        forecast: daily_forecast,
        grass: profile.grass_type,
        history: &history,
    });

    let mut data_notes = data_notes(&assessment, profile, forecast.is_some());
    // @spec TIME-ENG-046
    if let Some(read_at) = climate.last_good_at {
        data_notes.insert(0, memo::last_good_note(state.clock.local(read_at)));
    }
    Ok(TimingResponse {
        generated_at: Utc::now(),
        today,
        station: format!("NOAA USCRN station {}", client.station_wbanno()),
        context: context::build_context(days, &assessment.history_years, daily_forecast),
        series: context::build_series(&assessment.smoothed, today, &assessment.history_years),
        soil: assessment.soil,
        freeze: assessment.freeze,
        windows: assessment.windows,
        history_years: assessment.history_years,
        data_notes,
    })
}

// @spec FEED-PIPE-016
/// What the timing windows contribute to the feed/dashboard: one recommendation per
/// actionable window, core aeration (same window as seeding), and the fall herbicide
/// reconciled with the seeding decision. Like disease risk this is additive: a lake
/// outage leaves the feed as the rules produced it rather than failing it.
pub(crate) async fn apply_to_feed(
    state: &AppState,
    profile: &LawnProfile,
    history: &[Application],
    recommendations: &mut Vec<Recommendation>,
) {
    let response = match compute(state, profile).await {
        Ok(response) => response,
        Err(e) => {
            tracing::warn!("Timing windows unavailable for recommendations: {}", e);
            return;
        }
    };
    recommendations.extend(timing::recommendations::to_recommendations(
        &response.windows,
        &response.soil,
        profile,
        response.today,
    ));
    recommendations.extend(timing::companions::aeration(
        &response.windows,
        profile,
        history,
        response.today,
    ));
    timing::companions::reconcile_fall_herbicide(recommendations, &response.windows);
}

/// Seasonal-plan activities for `year` from the timing windows. `None` (so the plan keeps
/// its own versions) when the lake is unavailable or has no usable history.
pub(crate) async fn plan_activities(
    state: &AppState,
    profile: &LawnProfile,
    year: i32,
) -> Option<Vec<PlannedActivity>> {
    match plan_for(state, profile, year).await {
        Ok(activities) if !activities.is_empty() => Some(activities),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("Timing windows unavailable for the seasonal plan: {}", e);
            None
        }
    }
}

async fn plan_for(
    state: &AppState,
    profile: &LawnProfile,
    year: i32,
) -> Result<Vec<PlannedActivity>, TurfOpsError> {
    // @spec TIME-PLAN-007
    // The same inputs as the Timing page, forecast included, so the plan's status matches
    // the window's state there.
    let (client, forecast) = lake_and_forecast(state).await?;
    // @spec TIME-ENG-039
    let record = climate_record(state, &client).await?;
    let today = state.clock.today();
    let days = &record.days;

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

    let data =
        timing::SeasonData::build(days, forecast.as_deref().unwrap_or_default(), today.year());
    let season = data.season(days, today);
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

    // @spec TIME-ENG-039
    #[test]
    fn the_climate_record_starts_from_the_lawns_year() {
        let new_years_eve = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        assert_eq!(
            climate_start(new_years_eve),
            NaiveDate::from_ymd_opt(2025 - timing::HISTORY_YEARS, 1, 1)
        );
    }

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
