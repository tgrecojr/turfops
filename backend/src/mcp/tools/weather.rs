//! Station weather history as daily rows. The REST `/historical` endpoint downsamples
//! hourly points for its charts, so it cannot give a true daily rain total; this tool
//! joins two daily lake reads instead: silver aggregated per local day (air, humidity,
//! rain) and the memoized climate record (5 cm soil, gold `gdd50`).

use super::{json_result, tool_error};
use crate::api;
use crate::datasources::weather::ClimateDay;
use crate::error::TurfOpsError;
use crate::mcp::TurfOpsMcp;
use crate::models::{celsius_to_fahrenheit, DailyWeather};
use chrono::{Duration, NaiveDate, Utc};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const DEFAULT_DAYS: i64 = 30;
const MAX_DAYS: i64 = 120;
const MM_PER_INCH: f64 = 25.4;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct WeatherHistoryParams {
    /// Days back from today (default 30, max 120).
    pub days: Option<i64>,
}

/// One station-local day. Any value can be missing (sensor outages, the lake's lag).
#[derive(Debug, Default, PartialEq, Serialize)]
pub struct HistoryDay {
    pub date: NaiveDate,
    pub air_mean_f: Option<f64>,
    pub air_min_f: Option<f64>,
    pub air_max_f: Option<f64>,
    pub rh_mean_pct: Option<f64>,
    pub rain_in: Option<f64>,
    pub soil_5cm_f: Option<f64>,
    pub gdd50: Option<f64>,
    /// Hourly observations behind the air / humidity / rain values (24 = complete day).
    pub hours_observed: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct WeatherHistory {
    pub start: NaiveDate,
    pub days: Vec<HistoryDay>,
    pub rain_total_in: f64,
    pub gdd50_total: f64,
    pub note: &'static str,
}

#[tool_router(router = weather_tools, vis = "pub(in crate::mcp)")]
impl TurfOpsMcp {
    #[tool(
        description = "Daily station weather for the last N days (default 30, max 120): air \
                       mean / min / max °F, mean humidity, rain (inches), 5 cm soil mean °F, \
                       GDD base 50 and hours observed, plus rain and GDD totals. Use for \
                       'how much rain have we had', 'has it been dry', recent soil trends. \
                       The newest day is usually partial (the lake lags about a day).",
        annotations(read_only_hint = true)
    )]
    async fn weather_history(
        &self,
        Parameters(params): Parameters<WeatherHistoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let days = params.days.unwrap_or(DEFAULT_DAYS).clamp(1, MAX_DAYS);
        match self.history(days).await {
            Ok(history) => json_result(&history),
            Err(err) => Ok(tool_error(err)),
        }
    }
}

impl TurfOpsMcp {
    async fn history(&self, days: i64) -> Result<WeatherHistory, TurfOpsError> {
        let client = self
            .state
            .sync_service
            .read()
            .await
            .weather_client()
            .cloned();
        let client = client.ok_or_else(|| {
            TurfOpsError::DataSourceUnavailable("weather data lake not configured".into())
        })?;
        let start = Utc::now().date_naive() - Duration::days(days);
        let (lake, climate) = tokio::try_join!(
            client.fetch_daily_disease_inputs(start),
            api::timing::climate_record(&self.state, &client),
        )?;
        Ok(join_days(&lake.days, &climate.days, start))
    }
}

/// Merge the silver daily aggregates with the climate record, one row per date from
/// `start`, ascending.
fn join_days(
    observed: &[DailyWeather],
    climate: &[ClimateDay],
    start: NaiveDate,
) -> WeatherHistory {
    let mut rows: BTreeMap<NaiveDate, HistoryDay> = BTreeMap::new();
    for day in observed.iter().filter(|d| d.date >= start) {
        let r = entry(&mut rows, day.date);
        r.air_mean_f = Some(round1(celsius_to_fahrenheit(day.temp_mean_c)));
        r.air_min_f = Some(round1(celsius_to_fahrenheit(day.temp_min_c)));
        r.air_max_f = Some(round1(celsius_to_fahrenheit(day.temp_max_c)));
        r.rh_mean_pct = Some(round1(day.rh_mean));
        r.rain_in = Some(round2(day.precip_mm / MM_PER_INCH));
        r.hours_observed = Some(day.hours_covered);
    }
    for day in climate.iter().filter(|d| d.date >= start) {
        let r = entry(&mut rows, day.date);
        r.soil_5cm_f = day.soil_temp_5_f.map(round1);
        r.gdd50 = day.gdd50.map(round1);
        // Gold's air values fill a day silver has nothing for.
        r.air_mean_f = r.air_mean_f.or(day.air_avg_f.map(round1));
        r.air_min_f = r.air_min_f.or(day.air_min_f.map(round1));
    }
    let days: Vec<HistoryDay> = rows.into_values().collect();
    WeatherHistory {
        start,
        rain_total_in: round2(days.iter().filter_map(|d| d.rain_in).sum()),
        gdd50_total: round1(days.iter().filter_map(|d| d.gdd50).sum()),
        days,
        note: "Station-local days from NOAA USCRN. A day with fewer than 24 hours observed \
               is partial; its totals are low and its min / max may be missing the dawn \
               low or afternoon high.",
    }
}

fn entry(rows: &mut BTreeMap<NaiveDate, HistoryDay>, date: NaiveDate) -> &mut HistoryDay {
    rows.entry(date).or_insert_with(|| HistoryDay {
        date,
        ..HistoryDay::default()
    })
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn observed(date: &str, mean_c: f64, precip_mm: f64, hours: f64) -> DailyWeather {
        DailyWeather {
            date: d(date),
            temp_mean_c: mean_c,
            temp_min_c: mean_c - 5.0,
            temp_max_c: mean_c + 5.0,
            rh_mean: 72.44,
            hours_rh90: 0.0,
            leaf_wetness_hours: 0.0,
            precip_mm,
            dew_point_max_c: 10.0,
            hours_covered: hours,
            is_forecast: false,
            covers_dawn: true,
            covers_afternoon: true,
        }
    }

    fn climate(date: &str, soil: Option<f64>, gdd: Option<f64>) -> ClimateDay {
        ClimateDay {
            date: d(date),
            soil_temp_5_f: soil,
            air_min_f: Some(50.0),
            air_avg_f: Some(60.0),
            gdd50: gdd,
        }
    }

    #[test]
    fn joins_by_date_converts_units_and_totals() {
        let history = join_days(
            &[
                observed("2026-09-20", 20.0, 25.4, 24.0),
                observed("2026-09-21", 10.0, 12.7, 9.0),
            ],
            &[
                climate("2026-09-20", Some(68.26), Some(18.0)),
                climate("2026-09-21", Some(66.0), Some(4.5)),
            ],
            d("2026-09-20"),
        );
        assert_eq!(history.days.len(), 2);
        let first = &history.days[0];
        assert_eq!(first.date, d("2026-09-20"));
        assert_eq!(first.air_mean_f, Some(68.0));
        assert_eq!(first.air_min_f, Some(59.0));
        assert_eq!(first.air_max_f, Some(77.0));
        assert_eq!(first.rh_mean_pct, Some(72.4));
        assert_eq!(first.rain_in, Some(1.0));
        assert_eq!(first.soil_5cm_f, Some(68.3));
        assert_eq!(history.days[1].hours_observed, Some(9.0));
        assert_eq!(history.rain_total_in, 1.5);
        assert_eq!(history.gdd50_total, 22.5);
    }

    #[test]
    fn days_before_start_are_dropped_and_gold_fills_missing_air() {
        let history = join_days(
            &[observed("2026-09-10", 20.0, 0.0, 24.0)],
            &[
                climate("2026-09-10", None, None),
                climate("2026-09-21", None, Some(3.0)),
            ],
            d("2026-09-15"),
        );
        assert_eq!(history.days.len(), 1);
        let only = &history.days[0];
        assert_eq!(only.date, d("2026-09-21"));
        assert_eq!(only.air_mean_f, Some(60.0));
        assert_eq!(only.air_min_f, Some(50.0));
        assert_eq!(only.air_max_f, None);
        assert_eq!(only.rain_in, None);
    }
}
