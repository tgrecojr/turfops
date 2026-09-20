use crate::models::seasonal_plan::WindowConfidence;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

/// Response of `GET /api/v1/timing-windows`: seeding and pre-emergent windows derived
/// from the station's 5 cm soil-temperature record and its freeze climatology.
#[derive(Debug, Clone, Serialize)]
pub struct TimingResponse {
    pub generated_at: DateTime<Utc>,
    pub today: NaiveDate,
    pub station: String,
    pub soil: SoilNow,
    pub freeze: FreezeDates,
    pub windows: Vec<TimingWindow>,
    pub context: SeasonContext,
    pub series: Vec<SoilSeriesDay>,
    /// Complete calendar years behind the "typical" dates.
    pub history_years: Vec<i32>,
    pub data_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SoilNow {
    /// Trailing 5-day mean of the daily 5 cm soil temperature (°F).
    pub avg_5day_f: Option<f64>,
    pub as_of: Option<NaiveDate>,
    pub depth_cm: u32,
}

/// Day-of-year statistics for an event across the historical years, expressed as
/// dates in the season being evaluated.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DateStat {
    pub median: NaiveDate,
    /// 10th / 90th percentile (nearest rank). With few years these equal earliest/latest.
    pub p10: NaiveDate,
    pub p90: NaiveDate,
    pub earliest: NaiveDate,
    pub latest: NaiveDate,
    pub sample_count: usize,
    pub confidence: WindowConfidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct FreezeDates {
    /// Last spring / first fall day with a minimum air temperature ≤ 32 °F.
    pub last_spring: Option<DateStat>,
    pub first_fall: Option<DateStat>,
    pub last_spring_this_year: Option<NaiveDate>,
    pub first_fall_this_year: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowId {
    FallSeeding,
    SpringPreEmergent,
    FallPreEmergent,
    SpringSeeding,
    DormantSeeding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WindowPriority {
    Primary,
    Secondary,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WindowState {
    NotYet,
    OpeningSoon,
    /// Open, but outside the ideal part of the window (or the window has none).
    Open,
    Ideal,
    Closing,
    Closed,
    /// The matching application is already logged for this season.
    Done,
    /// Ruled out by another logged application (seed vs. pre-emergent).
    Blocked,
}

/// Where a boundary's date for the evaluated season came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DateSource {
    /// Seen in station data and held for the full sustain period.
    Observed,
    /// Seen in station data but not yet held long enough to confirm.
    Tentative,
    /// Only in the soil-temperature forecast.
    Forecast,
    /// Historical median / freeze climatology.
    Typical,
}

/// One edge of a window (opens, ideal-from, ideal-until, closes).
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryView {
    /// What triggers it, e.g. "5 cm soil falls to 70°F".
    pub label: String,
    pub typical: Option<DateStat>,
    /// Best date for the evaluated season. `None` when the trigger is overdue versus
    /// its typical date but still has not happened.
    pub date: Option<NaiveDate>,
    pub source: DateSource,
    pub passed: bool,
    /// Observed days the soil trigger has held so far (tentative crossings only).
    pub days_held: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingWindow {
    pub id: WindowId,
    pub name: String,
    pub priority: WindowPriority,
    /// Calendar year the window opens in.
    pub season_year: i32,
    pub state: WindowState,
    pub headline: String,
    pub detail: String,
    pub opens: BoundaryView,
    pub ideal_from: Option<BoundaryView>,
    pub ideal_until: Option<BoundaryView>,
    pub closes: BoundaryView,
    pub done_on: Option<NaiveDate>,
    /// Why the window is blocked, when it is.
    pub conflict: Option<String>,
    pub guidance: Vec<String>,
}

/// How this season is running against the station's own history.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SeasonContext {
    /// Mean air temperature over the last 30 days minus the same dates in prior years (°F).
    pub air_anomaly_30d_f: Option<f64>,
    /// Same for the 5 cm soil temperature over the last 7 days.
    pub soil_anomaly_7d_f: Option<f64>,
    pub summary: Option<String>,
    /// Freezes and heat in the forecast that matter for seed or herbicide timing.
    pub forecast_alerts: Vec<String>,
}

/// One calendar day of the soil chart: this year against last year and the typical year.
/// All values are 5-day trailing means at 5 cm (°F).
#[derive(Debug, Clone, Serialize)]
pub struct SoilSeriesDay {
    pub date: NaiveDate,
    pub this_year: Option<f64>,
    pub forecast: Option<f64>,
    pub last_year: Option<f64>,
    pub typical: Option<f64>,
}
