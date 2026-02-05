use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

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
    Other,
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
            ApplicationType::Other => "Other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(['-', ' '], "").as_str() {
            "preemergent" => Some(ApplicationType::PreEmergent),
            "postemergent" => Some(ApplicationType::PostEmergent),
            "fertilizer" => Some(ApplicationType::Fertilizer),
            "fungicide" => Some(ApplicationType::Fungicide),
            "insecticide" => Some(ApplicationType::Insecticide),
            "grubcontrol" => Some(ApplicationType::GrubControl),
            "overseed" => Some(ApplicationType::Overseed),
            "aeration" => Some(ApplicationType::Aeration),
            "dethatching" => Some(ApplicationType::Dethatching),
            "lime" => Some(ApplicationType::Lime),
            "sulfur" => Some(ApplicationType::Sulfur),
            "wetting" | "wettingagent" => Some(ApplicationType::Wetting),
            "other" => Some(ApplicationType::Other),
            _ => None,
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            ApplicationType::PreEmergent => Color::Yellow,
            ApplicationType::PostEmergent => Color::LightYellow,
            ApplicationType::Fertilizer => Color::Green,
            ApplicationType::Fungicide => Color::Magenta,
            ApplicationType::Insecticide => Color::Red,
            ApplicationType::GrubControl => Color::LightRed,
            ApplicationType::Overseed => Color::Cyan,
            ApplicationType::Aeration => Color::Blue,
            ApplicationType::Dethatching => Color::LightBlue,
            ApplicationType::Lime => Color::White,
            ApplicationType::Sulfur => Color::LightYellow,
            ApplicationType::Wetting => Color::LightCyan,
            ApplicationType::Other => Color::Gray,
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
    pub created_at: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_type_from_str_valid() {
        assert_eq!(
            ApplicationType::from_str("PreEmergent"),
            Some(ApplicationType::PreEmergent)
        );
        assert_eq!(
            ApplicationType::from_str("pre-emergent"),
            Some(ApplicationType::PreEmergent)
        );
        assert_eq!(
            ApplicationType::from_str("pre emergent"),
            Some(ApplicationType::PreEmergent)
        );
        assert_eq!(
            ApplicationType::from_str("Fertilizer"),
            Some(ApplicationType::Fertilizer)
        );
        assert_eq!(
            ApplicationType::from_str("grubcontrol"),
            Some(ApplicationType::GrubControl)
        );
        assert_eq!(
            ApplicationType::from_str("grub control"),
            Some(ApplicationType::GrubControl)
        );
    }

    #[test]
    fn application_type_from_str_invalid() {
        assert_eq!(ApplicationType::from_str("unknown"), None);
        assert_eq!(ApplicationType::from_str(""), None);
        assert_eq!(ApplicationType::from_str("spray"), None);
    }
}
