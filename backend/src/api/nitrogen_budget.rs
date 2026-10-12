use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{nitrogen_budget::annual_n_target, NitrogenApplication, NitrogenBudget};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NitrogenBudgetQuery {
    pub year: Option<i32>,
}

// @spec RULE-BUDGET-001, RULE-BUDGET-003, RULE-BUDGET-004
/// GET /api/v1/nitrogen-budget?year=2026
/// Returns nitrogen budget for the requested year based on application history.
pub async fn get_nitrogen_budget(
    State(state): State<AppState>,
    Query(params): Query<NitrogenBudgetQuery>,
) -> Result<Json<NitrogenBudget>, TurfOpsError> {
    let year = params.year.unwrap_or_else(|| state.clock.today().year());

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let (start_date, end_date) = budget_window(year, state.clock.today())
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;

    let apps = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        start_date,
        end_date,
    )
    .await?;

    let target = annual_n_target(profile.grass_type);

    // Calculate N applied to the lawn (plant applications and rows without analysis + rate are skipped)
    let mut n_applications = Vec::new();
    let mut total_n_applied = 0.0;

    for app in &apps {
        if let Some(n_lbs) = app.turf_nitrogen_lbs() {
            total_n_applied += n_lbs;
            n_applications.push(NitrogenApplication {
                date: app.application_date,
                product_name: app.product_name.clone(),
                nitrogen_pct: app.nitrogen_pct.unwrap_or_default(),
                rate_per_1000sqft: app.rate_per_1000sqft.unwrap_or_default(),
                n_lbs_per_1000sqft: n_lbs,
            });
        }
    }

    // Sort by date ascending
    n_applications.sort_by_key(|a| a.date);

    let remaining = (target.recommended_lbs_per_1000sqft - total_n_applied).max(0.0);
    let percent = if target.recommended_lbs_per_1000sqft > 0.0 {
        (total_n_applied / target.recommended_lbs_per_1000sqft * 100.0).min(999.0)
    } else {
        0.0
    };

    Ok(Json(NitrogenBudget {
        year,
        target_lbs_per_1000sqft: target.recommended_lbs_per_1000sqft,
        applied_lbs_per_1000sqft: total_n_applied,
        remaining_lbs_per_1000sqft: remaining,
        percent_of_target: percent,
        applications: n_applications,
        grass_type_target: target,
    }))
}

/// The application dates a year's budget counts: `[start, end)`, from Jan 1 through the
/// lawn's `today` at the latest, so a future-dated entry counts once its date arrives.
// @spec RULE-BUDGET-005
fn budget_window(year: i32, today: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let start = NaiveDate::from_ymd_opt(year, 1, 1)?;
    let next_year = NaiveDate::from_ymd_opt(year + 1, 1, 1)?;
    Some((start, next_year.min(today.succ_opt()?)))
}

#[cfg(all(test, feature = "db-tests"))]
mod db_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // @spec RULE-BUDGET-005
    #[test]
    fn this_years_budget_stops_at_today() {
        assert_eq!(
            budget_window(2026, d(2026, 10, 11)),
            Some((d(2026, 1, 1), d(2026, 10, 12)))
        );
    }

    // @spec RULE-BUDGET-005
    #[test]
    fn a_past_years_budget_covers_the_whole_year() {
        assert_eq!(
            budget_window(2025, d(2026, 10, 11)),
            Some((d(2025, 1, 1), d(2026, 1, 1)))
        );
    }
}
