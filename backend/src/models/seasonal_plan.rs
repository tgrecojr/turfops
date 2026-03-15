use chrono::NaiveDate;
use serde::Serialize;

/// A complete seasonal plan for a given year.
#[derive(Debug, Clone, Serialize)]
pub struct SeasonalPlan {
    pub year: i32,
    pub activities: Vec<PlannedActivity>,
    pub data_years_used: i32,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// A single planned lawn care activity with predicted date window.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedActivity {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub date_window: DateWindow,
    pub status: ActivityStatus,
    pub details: ActivityDetails,
}

/// Predicted date window based on historical threshold crossings.
#[derive(Debug, Clone, Serialize)]
pub struct DateWindow {
    pub predicted_start: NaiveDate,
    pub predicted_end: NaiveDate,
    pub earliest_historical: Option<NaiveDate>,
    pub latest_historical: Option<NaiveDate>,
    pub confidence: WindowConfidence,
}

/// How confident we are in the predicted window based on data availability.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum WindowConfidence {
    High,
    Medium,
    Low,
}

/// Activity status computed at response time from application history.
#[derive(Debug, Clone, Serialize)]
pub enum ActivityStatus {
    Upcoming,
    Active,
    Completed,
    Missed,
}

/// Additional details about the activity.
#[derive(Debug, Clone, Serialize)]
pub struct ActivityDetails {
    pub soil_temp_trigger: Option<String>,
    pub product_suggestions: Vec<String>,
    pub rate: Option<String>,
    pub notes: Option<String>,
}

/// A threshold crossing record from historical data.
#[derive(Debug, Clone)]
pub struct ThresholdCrossing {
    pub year: i32,
    pub threshold_name: String,
    pub crossing_date: NaiveDate,
    pub avg_soil_temp_f: f64,
}

/// Daily soil temperature average from NOAA (server-side aggregated).
#[derive(Debug, Clone)]
pub struct DailySoilTempAvg {
    pub date: NaiveDate,
    pub avg_temp_f: f64,
}
