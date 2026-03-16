use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationCategory {
    PreEmergent,
    GrubControl,
    Fertilizer,
    Fungicide,
    Overseeding,
    Irrigation,
    Mowing,
    FrostWarning,
    HeatStress,
    ApplicationTiming,
    DiseasePressure,
    Herbicide,
    Aeration,
    SoilTempForecast,
    General,
}

impl RecommendationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecommendationCategory::PreEmergent => "Pre-Emergent",
            RecommendationCategory::GrubControl => "Grub Control",
            RecommendationCategory::Fertilizer => "Fertilizer",
            RecommendationCategory::Fungicide => "Fungicide",
            RecommendationCategory::Overseeding => "Overseeding",
            RecommendationCategory::Irrigation => "Irrigation",
            RecommendationCategory::Mowing => "Mowing",
            RecommendationCategory::FrostWarning => "Frost Warning",
            RecommendationCategory::HeatStress => "Heat Stress",
            RecommendationCategory::ApplicationTiming => "Application Timing",
            RecommendationCategory::DiseasePressure => "Disease Pressure",
            RecommendationCategory::Herbicide => "Herbicide",
            RecommendationCategory::Aeration => "Aeration",
            RecommendationCategory::SoilTempForecast => "Soil Temp Forecast",
            RecommendationCategory::General => "General",
        }
    }
}

impl std::fmt::Display for RecommendationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Advisory,
    Warning,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "Info",
            Severity::Advisory => "Advisory",
            Severity::Warning => "Warning",
            Severity::Critical => "Critical",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub label: String,
    pub value: String,
    pub source: String,
}

impl DataPoint {
    pub fn new(label: &str, value: impl std::fmt::Display, source: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            source: source.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub category: RecommendationCategory,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub explanation: String,
    pub data_points: Vec<DataPoint>,
    pub suggested_action: Option<String>,
    pub created_at: DateTime<Utc>,
    pub dismissed: bool,
    pub addressed: bool,
}

impl Recommendation {
    pub fn new(
        id: impl Into<String>,
        category: RecommendationCategory,
        severity: Severity,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            category,
            severity,
            title: title.into(),
            description: description.into(),
            explanation: String::new(),
            data_points: Vec::new(),
            suggested_action: None,
            created_at: Utc::now(),
            dismissed: false,
            addressed: false,
        }
    }

    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = explanation.into();
        self
    }

    pub fn with_data_point(
        mut self,
        label: &str,
        value: impl std::fmt::Display,
        source: &str,
    ) -> Self {
        self.data_points.push(DataPoint::new(label, value, source));
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.suggested_action = Some(action.into());
        self
    }

    pub fn is_active(&self) -> bool {
        !self.dismissed && !self.addressed
    }
}
