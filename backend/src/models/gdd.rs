use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyGdd {
    pub date: NaiveDate,
    pub high_temp_f: f64,
    pub low_temp_f: f64,
    pub gdd_base50: f64,
    pub cumulative_gdd_base50: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GddSummary {
    pub year: i32,
    pub current_gdd_total: f64,
    pub daily_history: Vec<DailyGdd>,
    pub crabgrass_model: CrabgrassModel,
    pub last_computed_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrabgrassModel {
    pub germination_threshold: f64,
    pub current_gdd: f64,
    pub status: CrabgrassStatus,
    pub estimated_germination_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrabgrassStatus {
    PreGermination,
    ApproachingGermination,
    GerminationLikely,
    PostGermination,
}

impl CrabgrassStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CrabgrassStatus::PreGermination => "Pre-Germination",
            CrabgrassStatus::ApproachingGermination => "Approaching Germination",
            CrabgrassStatus::GerminationLikely => "Germination Likely",
            CrabgrassStatus::PostGermination => "Post-Germination",
        }
    }
}

impl std::fmt::Display for CrabgrassStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
