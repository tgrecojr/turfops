//! The season: timing windows, the plan, GDD, the soil outlook and the nitrogen budget.

use super::{call, call_trimmed};
use crate::api;
use crate::api::gdd::GddQuery;
use crate::api::nitrogen_budget::NitrogenBudgetQuery;
use crate::api::seasonal_plan::SeasonalPlanQuery;
use crate::mcp::{trim, TurfOpsMcp};
use axum::extract::{Query, State};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct TimingParams {
    /// Include the daily 5 cm soil series behind the windows (this year, forecast, last
    /// year, typical year). Large; only for questions about the curve itself.
    #[serde(default)]
    pub detail: bool,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct YearParams {
    /// Calendar year; defaults to the current year.
    pub year: Option<i32>,
}

#[tool_router(router = season_tools, vis = "pub(in crate::mcp)")]
impl TurfOpsMcp {
    #[tool(
        description = "Seeding (fall / spring / dormant) and pre-emergent (spring / fall) \
                       windows with this season's state, each boundary's date and source \
                       (Observed / Tentative / Forecast / Typical with p10–p90), freeze \
                       dates and current 5 cm soil. Use for any 'when should I seed / put \
                       down pre-emergent / aerate' question. A boundary with `date: null` \
                       means later than usual — not yet seen — never unknown.",
        annotations(read_only_hint = true)
    )]
    async fn timing_windows(
        &self,
        Parameters(params): Parameters<TimingParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let handler = api::timing::get_timing_windows(State(self.state.clone()));
        if params.detail {
            call(handler).await
        } else {
            call_trimmed(handler, trim::timing_without_series).await
        }
    }

    #[tool(
        description = "The year's lawn-care plan: each activity (pre-emergent, feedings, \
                       grub control, seeding, aeration, weed control, winterizer, plant \
                       tasks) with its predicted date window from historical soil-temp \
                       crossings and its status (Upcoming / Active / Completed / Missed).",
        annotations(read_only_hint = true)
    )]
    async fn seasonal_plan(
        &self,
        Parameters(params): Parameters<YearParams>,
    ) -> Result<CallToolResult, ErrorData> {
        call(api::seasonal_plan::get_seasonal_plan(
            State(self.state.clone()),
            Query(SeasonalPlanQuery { year: params.year }),
        ))
        .await
    }

    #[tool(
        description = "Growing degree days (base 50°F) accumulated this year and the \
                       crabgrass germination model (status, estimated germination date). \
                       GDD drives grub control (500–700), spring nitrogen and spring \
                       broadleaf timing (50–150). Daily values: use `weather_history`.",
        annotations(read_only_hint = true)
    )]
    async fn gdd(
        &self,
        Parameters(params): Parameters<YearParams>,
    ) -> Result<CallToolResult, ErrorData> {
        call_trimmed(
            api::gdd::get_gdd(
                State(self.state.clone()),
                Query(GddQuery { year: params.year }),
            ),
            |v| trim::without(v, &["daily_history"]),
        )
        .await
    }

    #[tool(
        description = "10 cm soil temperature outlook for the forecast days (air-to-soil \
                       regression, with fit quality) and the predicted crossings of the \
                       45 / 60 / 65 / 75°F thresholds. Timing windows use 5 cm soil; this \
                       outlook is for the 10 cm thresholds (e.g. grub control 60–75°F).",
        annotations(read_only_hint = true)
    )]
    async fn soil_temp_outlook(&self) -> Result<CallToolResult, ErrorData> {
        call(api::soil_temp_prediction::get_soil_temp_forecast(State(
            self.state.clone(),
        )))
        .await
    }

    #[tool(
        description = "Lawn nitrogen budget for the year: target for the grass type, lbs \
                       N / 1000 sq ft applied so far (with each feeding), remaining and \
                       percent of target. Use before advising any fertilizer.",
        annotations(read_only_hint = true)
    )]
    async fn nitrogen_budget(
        &self,
        Parameters(params): Parameters<YearParams>,
    ) -> Result<CallToolResult, ErrorData> {
        call(api::nitrogen_budget::get_nitrogen_budget(
            State(self.state.clone()),
            Query(NitrogenBudgetQuery { year: params.year }),
        ))
        .await
    }
}
