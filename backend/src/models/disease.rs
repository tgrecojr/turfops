use crate::models::{DiseaseManagement, FungicideRecord};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Turf diseases tracked by the per-disease risk models (cool-season, TTTF-relevant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Disease {
    BrownPatch,
    DollarSpot,
    PythiumBlight,
    GrayLeafSpot,
    RedThread,
}

impl Disease {
    /// Stable slug used in URLs and recommendation ids.
    pub fn slug(&self) -> &'static str {
        match self {
            Disease::BrownPatch => "brown-patch",
            Disease::DollarSpot => "dollar-spot",
            Disease::PythiumBlight => "pythium-blight",
            Disease::GrayLeafSpot => "gray-leaf-spot",
            Disease::RedThread => "red-thread",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Disease::BrownPatch => "Brown Patch",
            Disease::DollarSpot => "Dollar Spot",
            Disease::PythiumBlight => "Pythium Blight",
            Disease::GrayLeafSpot => "Gray Leaf Spot",
            Disease::RedThread => "Red Thread",
        }
    }

    pub fn pathogen(&self) -> &'static str {
        match self {
            Disease::BrownPatch => "Rhizoctonia solani",
            Disease::DollarSpot => "Clarireedia jacksonii",
            Disease::PythiumBlight => "Pythium aphanidermatum",
            Disease::GrayLeafSpot => "Pyricularia oryzae",
            Disease::RedThread => "Laetisaria fuciformis",
        }
    }
}

/// Common risk scale every model is read off onto. Each model keeps its own native
/// score; only the tier is comparable across diseases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskTier {
    Low,
    Moderate,
    High,
    Severe,
}

impl RiskTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskTier::Low => "Low",
            RiskTier::Moderate => "Moderate",
            RiskTier::High => "High",
            RiskTier::Severe => "Severe",
        }
    }

    /// One tier higher, saturating at Severe.
    pub fn raised(&self) -> RiskTier {
        match self {
            RiskTier::Low => RiskTier::Moderate,
            RiskTier::Moderate => RiskTier::High,
            RiskTier::High | RiskTier::Severe => RiskTier::Severe,
        }
    }
}

/// One local calendar day of disease-model weather inputs. Temperatures stay in °C
/// because the published models are specified in °C.
#[derive(Debug, Clone, PartialEq)]
pub struct DailyWeather {
    pub date: NaiveDate,
    pub temp_mean_c: f64,
    pub temp_min_c: f64,
    pub temp_max_c: f64,
    pub rh_mean: f64,
    /// Hours with RH ≥ 90%.
    pub hours_rh90: f64,
    /// Estimated leaf wetness: hours with RH ≥ 90% or measurable precipitation.
    pub leaf_wetness_hours: f64,
    pub precip_mm: f64,
    pub dew_point_max_c: f64,
    /// Hours of data behind this day (24 = complete).
    pub hours_covered: f64,
    /// True when any part of the day comes from the forecast rather than observations.
    pub is_forecast: bool,
    /// The data includes the pre-dawn hours, where the day's low and its dew live.
    pub covers_dawn: bool,
    /// The data includes the afternoon, where the day's high lives.
    pub covers_afternoon: bool,
}

/// Local hours that must be represented for a day's minimum / maximum to be real. Both
/// spans are wider than the forecast's 3-hour step, so a covered span always has a point.
pub const DAWN_HOURS: std::ops::RangeInclusive<u32> = 3..=8;
pub const AFTERNOON_HOURS: std::ops::RangeInclusive<u32> = 13..=17;

/// Lawn-history context that modifies weather-driven risk.
#[derive(Debug, Clone, Default)]
pub struct DiseaseContext {
    /// Days since the most recent overseeding, if any on record.
    pub days_since_overseed: Option<i64>,
    /// Days since the most recent fertilizer application, if any on record.
    pub days_since_fertilizer: Option<i64>,
    /// Fungicide applications from the past year, most recent first.
    pub fungicide_apps: Vec<FungicideRecord>,
    /// Season-level resistance warning from the rotation analysis.
    pub rotation_warning: Option<String>,
    /// Fungicides on the shelf (usable ones only), so the pick can prefer what is owned.
    pub shelf: Vec<super::ShelfFungicide>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyRisk {
    pub date: NaiveDate,
    /// Model-native score for the day (see `RiskScale`).
    pub value: f64,
    pub tier: RiskTier,
    pub is_forecast: bool,
    /// Day has under 12 hours of data behind it.
    pub partial: bool,
}

/// How a contributing factor currently bears on disease development.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FactorStatus {
    Unfavorable,
    Marginal,
    Favorable,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskFactor {
    pub label: String,
    pub value: String,
    pub status: FactorStatus,
    pub note: String,
}

/// Native scale of a model's score: gauge bounds and the lower bound of each tier
/// above Low.
#[derive(Debug, Clone, Serialize)]
pub struct RiskScale {
    pub min: f64,
    pub max: f64,
    pub unit: String,
    pub moderate_at: f64,
    pub high_at: f64,
    pub severe_at: f64,
}

impl RiskScale {
    pub fn tier_for(&self, value: f64) -> RiskTier {
        if value >= self.severe_at {
            RiskTier::Severe
        } else if value >= self.high_at {
            RiskTier::High
        } else if value >= self.moderate_at {
            RiskTier::Moderate
        } else {
            RiskTier::Low
        }
    }
}

/// The "how this is calculated" content for a model.
#[derive(Debug, Clone, Serialize)]
pub struct Methodology {
    pub model_name: String,
    pub citation: String,
    /// False for composite heuristics with no single peer-reviewed validation.
    pub validated: bool,
    pub summary: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiseaseRisk {
    pub disease: Disease,
    pub slug: String,
    pub name: String,
    pub pathogen: String,
    pub tier: RiskTier,
    /// Set when lawn history raised the tier above what the weather score alone gives.
    pub tier_note: Option<String>,
    pub score: f64,
    /// Human-readable score, e.g. "E-index 6.1" or "42% probability".
    pub score_label: String,
    /// The day the headline tier/score describes.
    pub as_of: NaiveDate,
    pub scale: RiskScale,
    pub daily: Vec<DailyRisk>,
    pub factors: Vec<RiskFactor>,
    pub summary: String,
    pub methodology: Methodology,
    pub management: DiseaseManagement,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiseaseRiskResponse {
    pub generated_at: DateTime<Utc>,
    /// Today's date at the station (the headline `as_of` can trail it).
    pub today: NaiveDate,
    pub station: String,
    pub diseases: Vec<DiseaseRisk>,
    /// Caveats about the inputs (lake lag, missing forecast, ...).
    pub data_notes: Vec<String>,
}
