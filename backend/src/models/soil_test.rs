use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilTest {
    pub id: Option<i64>,
    pub lawn_profile_id: i64,
    pub test_date: NaiveDate,
    pub lab_name: Option<String>,
    pub ph: f64,
    pub buffer_ph: Option<f64>,
    pub phosphorus_ppm: Option<f64>,
    pub potassium_ppm: Option<f64>,
    pub calcium_ppm: Option<f64>,
    pub magnesium_ppm: Option<f64>,
    pub sulfur_ppm: Option<f64>,
    pub iron_ppm: Option<f64>,
    pub manganese_ppm: Option<f64>,
    pub zinc_ppm: Option<f64>,
    pub boron_ppm: Option<f64>,
    pub copper_ppm: Option<f64>,
    pub organic_matter_pct: Option<f64>,
    pub cec: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NutrientLevel {
    Low,
    Adequate,
    High,
}

impl NutrientLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            NutrientLevel::Low => "Low",
            NutrientLevel::Adequate => "Adequate",
            NutrientLevel::High => "High",
        }
    }
}

impl std::fmt::Display for NutrientLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoilTestSummary {
    pub soil_test: SoilTest,
    pub ph_recommendation: Option<PhRecommendation>,
    pub npk_recommendation: Option<NpkRecommendation>,
    pub micronutrient_recommendations: Vec<MicronutrientRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhRecommendation {
    pub current_ph: f64,
    pub target_ph: f64,
    pub amendment: String,
    pub rate_lbs_per_1000sqft: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpkRecommendation {
    pub phosphorus_level: NutrientLevel,
    pub potassium_level: NutrientLevel,
    pub recommended_ratio: String,
    pub nitrogen_rate_lbs_per_1000sqft: f64,
    pub phosphorus_rate_lbs_per_1000sqft: f64,
    pub potassium_rate_lbs_per_1000sqft: f64,
    pub product_rate_lbs_per_1000sqft: f64,
    pub example_product_ratio: String,
    pub remaining_n_budget_lbs_per_1000sqft: f64,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicronutrientRecommendation {
    pub nutrient: String,
    pub current_ppm: f64,
    pub threshold_ppm: f64,
    pub level: NutrientLevel,
    pub suggestion: String,
}
