use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantType {
    Shrub,
    Tree,
    Perennial,
    Annual,
    Vine,
    Groundcover,
    Other,
}

impl PlantType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlantType::Shrub => "Shrub",
            PlantType::Tree => "Tree",
            PlantType::Perennial => "Perennial",
            PlantType::Annual => "Annual",
            PlantType::Vine => "Vine",
            PlantType::Groundcover => "Groundcover",
            PlantType::Other => "Other",
        }
    }
}

impl FromStr for PlantType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "shrub" => Ok(PlantType::Shrub),
            "tree" => Ok(PlantType::Tree),
            "perennial" => Ok(PlantType::Perennial),
            "annual" => Ok(PlantType::Annual),
            "vine" => Ok(PlantType::Vine),
            "groundcover" | "ground cover" => Ok(PlantType::Groundcover),
            "other" => Ok(PlantType::Other),
            _ => Err(format!("Unknown plant type: {}", s)),
        }
    }
}

impl std::fmt::Display for PlantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Pruning,
    Fertilizing,
    Mulching,
    Watering,
    PestInspection,
    Deadheading,
    WinterProtection,
    Other,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Pruning => "Pruning",
            TaskType::Fertilizing => "Fertilizing",
            TaskType::Mulching => "Mulching",
            TaskType::Watering => "Watering",
            TaskType::PestInspection => "Pest Inspection",
            TaskType::Deadheading => "Deadheading",
            TaskType::WinterProtection => "Winter Protection",
            TaskType::Other => "Other",
        }
    }

    /// The ApplicationType variant(s) that represent "completing" this maintenance task.
    /// Used to match logged Applications to PlantMaintenance tasks when computing status.
    pub fn matching_application_types(&self) -> &'static [super::ApplicationType] {
        use super::ApplicationType;
        match self {
            TaskType::Pruning => &[ApplicationType::Pruning],
            TaskType::Fertilizing => &[ApplicationType::PlantFertilizer],
            TaskType::Mulching => &[ApplicationType::Mulching],
            TaskType::Deadheading => &[ApplicationType::Deadheading],
            TaskType::WinterProtection => &[ApplicationType::WinterProtection],
            TaskType::Watering | TaskType::PestInspection | TaskType::Other => &[],
        }
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskFrequency {
    Once,
    Twice,
    Monthly,
    AsNeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentificationConfidence {
    High,
    Medium,
    Low,
}

/// A single maintenance task the LLM returned for a plant.
/// Windows are stored as "MM-DD" so they apply year-over-year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceTask {
    pub task_type: TaskType,
    pub window_start_month_day: String,
    pub window_end_month_day: String,
    pub frequency: TaskFrequency,
    pub description: String,
    pub severity: super::Severity,
    pub zone_note: Option<String>,
}

/// Full cached plan for a plant, as produced by the LLM and stored in JSONB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantMaintenancePlan {
    pub identified_name: String,
    pub scientific_name: Option<String>,
    pub identification_confidence: IdentificationConfidence,
    pub summary: String,
    pub tasks: Vec<MaintenanceTask>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plant {
    pub id: Option<i64>,
    pub lawn_profile_id: i64,
    pub common_name: String,
    pub scientific_name: Option<String>,
    pub plant_type: PlantType,
    pub location: Option<String>,
    pub planting_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub maintenance_plan: PlantMaintenancePlan,
    pub plan_generated_at: DateTime<Utc>,
    pub plan_model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plant_type_from_str_valid() {
        assert_eq!(PlantType::from_str("Shrub"), Ok(PlantType::Shrub));
        assert_eq!(PlantType::from_str("tree"), Ok(PlantType::Tree));
        assert_eq!(
            PlantType::from_str("ground cover"),
            Ok(PlantType::Groundcover)
        );
    }

    #[test]
    fn plant_type_from_str_invalid() {
        assert!(PlantType::from_str("moss").is_err());
        assert!(PlantType::from_str("").is_err());
    }

    #[test]
    fn task_type_matches_pruning_app() {
        let matches = TaskType::Pruning.matching_application_types();
        assert!(matches.contains(&super::super::ApplicationType::Pruning));
    }
}
