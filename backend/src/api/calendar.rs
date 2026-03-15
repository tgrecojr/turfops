use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::Application;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CalendarResponse {
    pub year: i32,
    pub month: u32,
    pub days: BTreeMap<String, Vec<Application>>,
}

pub async fn get_calendar(
    State(state): State<AppState>,
    Query(params): Query<CalendarQuery>,
) -> Result<Json<CalendarResponse>, TurfOpsError> {
    let today = Local::now().date_naive();
    let year = params.year.unwrap_or(today.year());
    let month = params.month.unwrap_or(today.month());

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let apps = queries::get_applications_for_profile(&state.pool, profile.id.unwrap()).await?;

    // Filter to the requested month and group by date
    let month_start = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
        TurfOpsError::InvalidData(format!("Invalid year/month: {}/{}", year, month))
    })?;
    let month_end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap_or(month_start);

    let mut days: BTreeMap<String, Vec<Application>> = BTreeMap::new();

    for app in apps {
        if app.application_date >= month_start && app.application_date < month_end {
            let date_key = app.application_date.format("%Y-%m-%d").to_string();
            days.entry(date_key).or_default().push(app);
        }
    }

    Ok(Json(CalendarResponse { year, month, days }))
}
