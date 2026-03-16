use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Fitted linear regression model: soil_temp = slope * lagged_air_temp + intercept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilTempModel {
    pub slope: f64,
    pub intercept: f64,
    pub lag_days: u32,
    pub r_squared: f64,
    pub training_window_days: u32,
    pub fitted_at: DateTime<Utc>,
}

/// A single day's predicted soil temperature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilTempPrediction {
    pub date: NaiveDate,
    pub predicted_soil_temp_f: f64,
    pub confidence: PredictionConfidence,
    pub air_temp_used_f: f64,
    pub source_description: String,
}

/// Confidence level for a prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionConfidence {
    High,
    Medium,
    Low,
}

impl PredictionConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            PredictionConfidence::High => "High",
            PredictionConfidence::Medium => "Medium",
            PredictionConfidence::Low => "Low",
        }
    }
}

impl std::fmt::Display for PredictionConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Direction a threshold is being crossed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossingDirection {
    Rising,
    Falling,
}

impl std::fmt::Display for CrossingDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrossingDirection::Rising => write!(f, "Rising"),
            CrossingDirection::Falling => write!(f, "Falling"),
        }
    }
}

/// A predicted threshold crossing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdPrediction {
    pub threshold_name: String,
    pub threshold_temp_f: f64,
    pub estimated_crossing_date: NaiveDate,
    pub days_until_crossing: i64,
    pub confidence: PredictionConfidence,
    pub direction: CrossingDirection,
}

/// Full API response for soil temperature forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilTempForecast {
    pub predictions: Vec<SoilTempPrediction>,
    pub threshold_crossings: Vec<ThresholdPrediction>,
    pub model_info: SoilTempModelInfo,
    pub generated_at: DateTime<Utc>,
}

/// Model metadata for display (excludes internal coefficients)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilTempModelInfo {
    pub r_squared: f64,
    pub lag_days: u32,
    pub training_window_days: u32,
    pub quality: String,
}

impl From<&SoilTempModel> for SoilTempModelInfo {
    fn from(model: &SoilTempModel) -> Self {
        let quality = if model.r_squared >= 0.7 {
            "Good"
        } else if model.r_squared >= 0.5 {
            "Fair"
        } else {
            "Poor"
        }
        .to_string();

        Self {
            r_squared: model.r_squared,
            lag_days: model.lag_days,
            training_window_days: model.training_window_days,
            quality,
        }
    }
}
