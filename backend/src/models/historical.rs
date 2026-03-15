use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalData {
    pub range: String,
    pub soil_temp_10_f: Vec<TimeSeriesPoint>,
    pub ambient_temp_f: Vec<TimeSeriesPoint>,
    pub humidity_percent: Vec<TimeSeriesPoint>,
    pub soil_moisture_10: Vec<TimeSeriesPoint>,
    pub precipitation_mm: Vec<TimeSeriesPoint>,
    pub gdd_accumulation: Vec<TimeSeriesPoint>,
}
