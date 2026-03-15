use crate::db::queries;
use crate::error::TurfOpsError;
use crate::models::{nitrogen_budget::annual_n_target, NitrogenApplication, NitrogenBudget};
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::{Datelike, Local, NaiveDate};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NitrogenBudgetQuery {
    pub year: Option<i32>,
}

/// GET /api/v1/nitrogen-budget?year=2026
/// Returns nitrogen budget for the requested year based on application history.
pub async fn get_nitrogen_budget(
    State(state): State<AppState>,
    Query(params): Query<NitrogenBudgetQuery>,
) -> Result<Json<NitrogenBudget>, TurfOpsError> {
    let year = params.year.unwrap_or_else(|| Local::now().year());

    let profile = queries::get_default_lawn_profile(&state.pool)
        .await?
        .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;

    let profile_id = profile
        .id
        .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;

    let start_date = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;
    let end_date = NaiveDate::from_ymd_opt(year + 1, 1, 1)
        .ok_or_else(|| TurfOpsError::InvalidData(format!("Invalid year: {}", year)))?;

    let apps = queries::get_applications_for_profile_in_range(
        &state.pool,
        profile_id,
        start_date,
        end_date,
    )
    .await?;

    let target = annual_n_target(profile.grass_type);

    // Calculate N applied from applications that have both nitrogen_pct and rate_per_1000sqft
    let mut n_applications = Vec::new();
    let mut total_n_applied = 0.0;

    for app in &apps {
        if let (Some(n_pct), Some(rate)) = (app.nitrogen_pct, app.rate_per_1000sqft) {
            if n_pct > 0.0 && rate > 0.0 {
                let n_lbs = n_pct / 100.0 * rate;
                total_n_applied += n_lbs;
                n_applications.push(NitrogenApplication {
                    date: app.application_date,
                    product_name: app.product_name.clone(),
                    nitrogen_pct: n_pct,
                    rate_per_1000sqft: rate,
                    n_lbs_per_1000sqft: n_lbs,
                });
            }
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
