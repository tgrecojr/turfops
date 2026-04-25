use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationType {
    PreEmergent,
    PostEmergent,
    Fertilizer,
    Fungicide,
    Insecticide,
    GrubControl,
    Overseed,
    Aeration,
    Dethatching,
    Lime,
    Sulfur,
    Wetting,
    Mowing,
    Other,
    // Plant-scoped types (carry plant_id on Application)
    Pruning,
    PlantFertilizer,
    Mulching,
    Deadheading,
    WinterProtection,
}

impl ApplicationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationType::PreEmergent => "Pre-Emergent",
            ApplicationType::PostEmergent => "Post-Emergent",
            ApplicationType::Fertilizer => "Fertilizer",
            ApplicationType::Fungicide => "Fungicide",
            ApplicationType::Insecticide => "Insecticide",
            ApplicationType::GrubControl => "Grub Control",
            ApplicationType::Overseed => "Overseed",
            ApplicationType::Aeration => "Aeration",
            ApplicationType::Dethatching => "Dethatching",
            ApplicationType::Lime => "Lime",
            ApplicationType::Sulfur => "Sulfur",
            ApplicationType::Wetting => "Wetting Agent",
            ApplicationType::Mowing => "Mowing",
            ApplicationType::Other => "Other",
            ApplicationType::Pruning => "Pruning",
            ApplicationType::PlantFertilizer => "Plant Fertilizer",
            ApplicationType::Mulching => "Mulching",
            ApplicationType::Deadheading => "Deadheading",
            ApplicationType::WinterProtection => "Winter Protection",
        }
    }

    /// Where this application type can be logged: against a specific plant, against
    /// the turf, or either.
    pub fn scope(&self) -> ApplicationScope {
        match self {
            ApplicationType::Pruning
            | ApplicationType::PlantFertilizer
            | ApplicationType::Mulching
            | ApplicationType::Deadheading
            | ApplicationType::WinterProtection => ApplicationScope::PlantRequired,
            ApplicationType::Fertilizer
            | ApplicationType::Fungicide
            | ApplicationType::Insecticide
            | ApplicationType::Wetting
            | ApplicationType::Other => ApplicationScope::Universal,
            ApplicationType::PreEmergent
            | ApplicationType::PostEmergent
            | ApplicationType::GrubControl
            | ApplicationType::Overseed
            | ApplicationType::Aeration
            | ApplicationType::Dethatching
            | ApplicationType::Lime
            | ApplicationType::Sulfur
            | ApplicationType::Mowing => ApplicationScope::TurfOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationScope {
    /// plant_id must be set (e.g. Pruning, Mulching).
    PlantRequired,
    /// plant_id may be set or null (e.g. Insecticide on a viburnum, or on the turf).
    Universal,
    /// plant_id must be null (e.g. Aeration, PreEmergent).
    TurfOnly,
}

impl FromStr for ApplicationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', ' '], "").as_str() {
            "preemergent" => Ok(ApplicationType::PreEmergent),
            "postemergent" => Ok(ApplicationType::PostEmergent),
            "fertilizer" => Ok(ApplicationType::Fertilizer),
            "fungicide" => Ok(ApplicationType::Fungicide),
            "insecticide" => Ok(ApplicationType::Insecticide),
            "grubcontrol" => Ok(ApplicationType::GrubControl),
            "overseed" => Ok(ApplicationType::Overseed),
            "aeration" => Ok(ApplicationType::Aeration),
            "dethatching" => Ok(ApplicationType::Dethatching),
            "lime" => Ok(ApplicationType::Lime),
            "sulfur" => Ok(ApplicationType::Sulfur),
            "wetting" | "wettingagent" => Ok(ApplicationType::Wetting),
            "mowing" | "mow" => Ok(ApplicationType::Mowing),
            "other" => Ok(ApplicationType::Other),
            "pruning" | "prune" => Ok(ApplicationType::Pruning),
            "plantfertilizer" => Ok(ApplicationType::PlantFertilizer),
            "mulching" | "mulch" => Ok(ApplicationType::Mulching),
            "deadheading" | "deadhead" => Ok(ApplicationType::Deadheading),
            "winterprotection" => Ok(ApplicationType::WinterProtection),
            _ => Err(format!("Unknown application type: {}", s)),
        }
    }
}

impl std::fmt::Display for ApplicationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSnapshot {
    pub soil_temp_10cm_f: Option<f64>,
    pub ambient_temp_f: Option<f64>,
    pub humidity_percent: Option<f64>,
    pub soil_moisture: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: Option<i64>,
    pub lawn_profile_id: i64,
    pub application_type: ApplicationType,
    pub product_name: Option<String>,
    pub application_date: NaiveDate,
    pub rate_per_1000sqft: Option<f64>,
    pub coverage_sqft: Option<f64>,
    pub notes: Option<String>,
    pub weather_snapshot: Option<WeatherSnapshot>,
    pub nitrogen_pct: Option<f64>,
    pub phosphorus_pct: Option<f64>,
    pub potassium_pct: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plant_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up_date: Option<NaiveDate>,
    pub created_at: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_type_from_str_valid() {
        assert_eq!(
            ApplicationType::from_str("PreEmergent"),
            Ok(ApplicationType::PreEmergent)
        );
        assert_eq!(
            ApplicationType::from_str("pre-emergent"),
            Ok(ApplicationType::PreEmergent)
        );
        assert_eq!(
            ApplicationType::from_str("pre emergent"),
            Ok(ApplicationType::PreEmergent)
        );
        assert_eq!(
            ApplicationType::from_str("Fertilizer"),
            Ok(ApplicationType::Fertilizer)
        );
        assert_eq!(
            ApplicationType::from_str("grubcontrol"),
            Ok(ApplicationType::GrubControl)
        );
        assert_eq!(
            ApplicationType::from_str("grub control"),
            Ok(ApplicationType::GrubControl)
        );
    }

    #[test]
    fn application_type_from_str_invalid() {
        assert!(ApplicationType::from_str("unknown").is_err());
        assert!(ApplicationType::from_str("").is_err());
        assert!(ApplicationType::from_str("spray").is_err());
    }
}
