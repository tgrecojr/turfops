//! The lawn as a whole: profile, the one-call snapshot, conditions and the feed.

use super::{call, call_trimmed, error_message, json_result, to_value};
use crate::api;
use crate::api::applications::ListApplicationsQuery;
use crate::api::nitrogen_budget::NitrogenBudgetQuery;
use crate::api::recommendations::ListRecommendationsQuery;
use crate::error::TurfOpsError;
use crate::mcp::{trim, TurfOpsMcp};
use axum::extract::{Query, State};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

/// Latest log entries in the snapshot (the dashboard's count); `applications` has more.
const SNAPSHOT_RECENT_APPLICATIONS: i64 = 5;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct RecommendationsParams {
    /// Also return recommendations the owner already dismissed or marked addressed
    /// (flagged as such). Default false: only what is active now.
    #[serde(default)]
    pub include_inactive: bool,
}

#[tool_router(router = lawn_tools, vis = "pub(in crate::mcp)")]
impl TurfOpsMcp {
    #[tool(
        description = "The lawn's profile: name, grass type, USDA zone, soil type, size in \
                       sq ft and irrigation. Use to tailor advice to this lawn.",
        annotations(read_only_hint = true)
    )]
    async fn lawn_profile(&self) -> Result<CallToolResult, ErrorData> {
        call(api::profile::get_profile(State(self.state.clone()))).await
    }

    #[tool(
        description = "CALL FIRST for any question about what to do on this lawn now or \
                       this season. One call returns: the profile, current conditions + \
                       daily forecast, the full active recommendation feed, the 5 latest \
                       logged applications, data-source status, the seeding / pre-emergent \
                       timing windows with this season's state, a per-disease risk headline \
                       (tier, action, protected-through) and the nitrogen budget totals. \
                       A part that fails is null with an entry in `notes`.",
        annotations(read_only_hint = true)
    )]
    async fn lawn_snapshot(&self) -> Result<CallToolResult, ErrorData> {
        let state = || State(self.state.clone());
        let (profile, conditions, feed, recent, timing, disease, nitrogen) = tokio::join!(
            to_value(api::profile::get_profile(state())),
            to_value(api::environmental::get_environmental(state())),
            to_value(api::recommendations::list_recommendations(
                state(),
                Query(ListRecommendationsQuery {
                    include_inactive: false,
                }),
            )),
            to_value(api::applications::list_applications(
                state(),
                Query(ListApplicationsQuery {
                    app_type: None,
                    limit: Some(SNAPSHOT_RECENT_APPLICATIONS),
                    offset: None,
                }),
            )),
            to_value(api::timing::get_timing_windows(state())),
            to_value(api::disease_risk::get_disease_risk(state())),
            to_value(api::nitrogen_budget::get_nitrogen_budget(
                state(),
                Query(NitrogenBudgetQuery { year: None }),
            )),
        );
        let connections = self
            .state
            .sync_service
            .read()
            .await
            .check_connections()
            .await;

        let mut notes = Vec::new();
        let mut part = |result: Result<_, TurfOpsError>, name, shape: fn(Value) -> Value| {
            trim::part(result.map(shape).map_err(error_message), name, &mut notes)
        };
        let parts = vec![
            ("profile", part(profile, "profile", identity)),
            ("conditions", part(conditions, "conditions", without_hourly)),
            ("recommendations", part(feed, "recommendations", identity)),
            (
                "recent_applications",
                part(recent, "recent_applications", identity),
            ),
            (
                "timing_windows",
                part(timing, "timing_windows", trim::timing_without_series),
            ),
            (
                "disease_risk",
                part(disease, "disease_risk", |v| trim::disease_headlines(&v)),
            ),
            (
                "nitrogen_budget",
                part(nitrogen, "nitrogen_budget", trim::nitrogen_totals),
            ),
        ];
        let mut snapshot = trim::object(parts);
        snapshot["data_sources"] = serde_json::to_value(connections).unwrap_or_default();
        snapshot["notes"] = notes.into();
        json_result(&snapshot)
    }

    #[tool(
        description = "Current conditions: live soil temp / moisture (station, with \
                       `soil_observed_at`), patio air temp / humidity, 7-day averages and \
                       rain total, soil trend, YTD GDD (base 50°F) and the daily forecast \
                       summaries. Use for watering, mowing, spraying-weather questions.",
        annotations(read_only_hint = true)
    )]
    async fn current_conditions(&self) -> Result<CallToolResult, ErrorData> {
        call_trimmed(
            api::environmental::get_environmental(State(self.state.clone())),
            without_hourly,
        )
        .await
    }

    #[tool(
        description = "The recommendation feed exactly as the app shows it: each item's id, \
                       category, severity, title, explanation, suggested action, the data \
                       points behind it, and `inventory` (whether the shelf covers what it \
                       calls for). When an item already answers the user's question, cite it.",
        annotations(read_only_hint = true)
    )]
    async fn recommendations(
        &self,
        Parameters(params): Parameters<RecommendationsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        call(api::recommendations::list_recommendations(
            State(self.state.clone()),
            Query(ListRecommendationsQuery {
                include_inactive: params.include_inactive,
            }),
        ))
        .await
    }
}

fn identity(value: Value) -> Value {
    value
}

fn without_hourly(mut summary: Value) -> Value {
    trim::drop_hourly_forecast(&mut summary);
    summary
}
