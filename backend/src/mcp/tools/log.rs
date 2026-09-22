//! The application log.

use super::{call, json_result, tool_error};
use crate::api;
use crate::api::applications::ListApplicationsQuery;
use crate::db::queries;
use crate::error::TurfOpsError;
use crate::mcp::TurfOpsMcp;
use crate::models::{Application, ApplicationType};
use axum::extract::{Query, State};
use chrono::{Days, NaiveDate};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;
use std::str::FromStr;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ApplicationsParams {
    /// Only this application type: PreEmergent, PostEmergent, Fertilizer, Fungicide,
    /// Insecticide, GrubControl, Overseed, Aeration, Dethatching, Lime, Sulfur, Wetting,
    /// Mowing, Other, Pruning, PlantFertilizer, Mulching, Deadheading, WinterProtection.
    #[serde(rename = "type")]
    pub app_type: Option<String>,
    /// Earliest application date, inclusive (YYYY-MM-DD).
    pub since: Option<String>,
    /// Latest application date, inclusive (YYYY-MM-DD).
    pub until: Option<String>,
    /// Most entries to return, newest first (default 20, max 100).
    pub limit: Option<i64>,
}

#[tool_router(router = log_tools, vis = "pub(in crate::mcp)")]
impl TurfOpsMcp {
    #[tool(
        description = "The application log, newest first: what was applied or done, when, \
                       product name and linked shelf `product_id`, rate, N-P-K, FRAC classes \
                       for fungicides, notes and the weather at the time. Filter by type and \
                       date range. Use to answer 'when did I last…' and to check what a \
                       season already has logged (e.g. an overseed blocks pre-emergent).",
        annotations(read_only_hint = true)
    )]
    async fn applications(
        &self,
        Parameters(params): Parameters<ApplicationsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let range = match date_range(params.since.as_deref(), params.until.as_deref()) {
            Ok(range) => range,
            Err(err) => return Ok(tool_error(err)),
        };
        let Some((start, end)) = range else {
            return call(api::applications::list_applications(
                State(self.state.clone()),
                Query(ListApplicationsQuery {
                    app_type: params.app_type,
                    limit: Some(limit),
                    offset: None,
                }),
            ))
            .await;
        };
        match self
            .in_range(params.app_type.as_deref(), start, end, limit)
            .await
        {
            Ok(apps) => json_result(&apps),
            Err(err) => Ok(tool_error(err)),
        }
    }
}

impl TurfOpsMcp {
    async fn in_range(
        &self,
        app_type: Option<&str>,
        start: NaiveDate,
        end_exclusive: NaiveDate,
        limit: i64,
    ) -> Result<Vec<Application>, TurfOpsError> {
        let app_type = app_type
            .map(|t| {
                ApplicationType::from_str(t).map_err(|_| {
                    TurfOpsError::InvalidData(format!("Unknown application type: {t}"))
                })
            })
            .transpose()?;
        let profile = queries::get_default_lawn_profile(&self.state.pool)
            .await?
            .ok_or_else(|| TurfOpsError::NotFound("No lawn profile found".into()))?;
        let profile_id = profile
            .id
            .ok_or_else(|| TurfOpsError::InvalidData("Profile missing ID".into()))?;
        let apps = queries::get_applications_for_profile_in_range(
            &self.state.pool,
            profile_id,
            start,
            end_exclusive,
        )
        .await?;
        Ok(apps
            .into_iter()
            .filter(|a| app_type.is_none_or(|t| a.application_type == t))
            .take(limit as usize)
            .collect())
    }
}

/// `[since, until]` as a half-open `[start, end)` range for the SQL query, or `None`
/// when neither bound is given. An open side is bounded by a date Postgres can store.
fn date_range(
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Option<(NaiveDate, NaiveDate)>, TurfOpsError> {
    if since.is_none() && until.is_none() {
        return Ok(None);
    }
    let parse = |label: &str, raw: &str| {
        NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").map_err(|_| {
            TurfOpsError::InvalidData(format!("`{label}` must be a date as YYYY-MM-DD, got {raw}"))
        })
    };
    let start = match since {
        Some(raw) => parse("since", raw)?,
        None => NaiveDate::from_ymd_opt(1900, 1, 1).expect("valid date"),
    };
    let end = match until {
        Some(raw) => parse("until", raw)?
            .checked_add_days(Days::new(1))
            .ok_or_else(|| TurfOpsError::InvalidData("`until` is out of range".into()))?,
        None => NaiveDate::from_ymd_opt(9999, 12, 31).expect("valid date"),
    };
    if start >= end {
        return Err(TurfOpsError::InvalidData(
            "`since` must not be after `until`".into(),
        ));
    }
    Ok(Some((start, end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn no_bounds_is_no_range() {
        assert_eq!(date_range(None, None).unwrap(), None);
    }

    #[test]
    fn until_is_inclusive() {
        let range = date_range(Some("2026-09-01"), Some("2026-09-22")).unwrap();
        assert_eq!(range, Some((d("2026-09-01"), d("2026-09-23"))));
    }

    #[test]
    fn a_single_day_is_a_valid_range() {
        let range = date_range(Some("2026-09-22"), Some("2026-09-22")).unwrap();
        assert_eq!(range, Some((d("2026-09-22"), d("2026-09-23"))));
    }

    #[test]
    fn open_sides_get_storable_bounds() {
        let (start, _) = date_range(None, Some("2026-01-01")).unwrap().unwrap();
        let (_, end) = date_range(Some("2026-01-01"), None).unwrap().unwrap();
        assert_eq!(start, d("1900-01-01"));
        assert_eq!(end, d("9999-12-31"));
    }

    #[test]
    fn bad_dates_and_inverted_ranges_are_rejected() {
        assert!(date_range(Some("09/01/2026"), None).is_err());
        assert!(date_range(Some("2026-09-22"), Some("2026-09-01")).is_err());
    }
}
