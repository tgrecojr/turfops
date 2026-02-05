use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrassType {
    KentuckyBluegrass,
    TallFescue,
    PerennialRyegrass,
    FineFescue,
    Bermuda,
    Zoysia,
    StAugustine,
    Mixed,
}

impl GrassType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GrassType::KentuckyBluegrass => "Kentucky Bluegrass",
            GrassType::TallFescue => "Tall Fescue",
            GrassType::PerennialRyegrass => "Perennial Ryegrass",
            GrassType::FineFescue => "Fine Fescue",
            GrassType::Bermuda => "Bermuda",
            GrassType::Zoysia => "Zoysia",
            GrassType::StAugustine => "St. Augustine",
            GrassType::Mixed => "Mixed",
        }
    }

    pub fn is_cool_season(&self) -> bool {
        matches!(
            self,
            GrassType::KentuckyBluegrass
                | GrassType::TallFescue
                | GrassType::PerennialRyegrass
                | GrassType::FineFescue
        )
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "kentuckybluegrass" | "kentucky bluegrass" | "kbg" => {
                Some(GrassType::KentuckyBluegrass)
            }
            "tallfescue" | "tall fescue" | "tttf" => Some(GrassType::TallFescue),
            "perennialryegrass" | "perennial ryegrass" | "prg" => {
                Some(GrassType::PerennialRyegrass)
            }
            "finefescue" | "fine fescue" => Some(GrassType::FineFescue),
            "bermuda" => Some(GrassType::Bermuda),
            "zoysia" => Some(GrassType::Zoysia),
            "staugustine" | "st. augustine" | "st augustine" => Some(GrassType::StAugustine),
            "mixed" => Some(GrassType::Mixed),
            _ => None,
        }
    }
}

impl std::fmt::Display for GrassType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoilType {
    Clay,
    Loam,
    Sandy,
    SiltLoam,
    ClayLoam,
    SandyLoam,
}

impl SoilType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SoilType::Clay => "Clay",
            SoilType::Loam => "Loam",
            SoilType::Sandy => "Sandy",
            SoilType::SiltLoam => "Silt Loam",
            SoilType::ClayLoam => "Clay Loam",
            SoilType::SandyLoam => "Sandy Loam",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "clay" => Some(SoilType::Clay),
            "loam" => Some(SoilType::Loam),
            "sandy" => Some(SoilType::Sandy),
            "siltloam" | "silt loam" => Some(SoilType::SiltLoam),
            "clayloam" | "clay loam" => Some(SoilType::ClayLoam),
            "sandyloam" | "sandy loam" => Some(SoilType::SandyLoam),
            _ => None,
        }
    }
}

impl std::fmt::Display for SoilType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrrigationType {
    InGround,
    Hose,
    None,
}

impl IrrigationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IrrigationType::InGround => "In-Ground",
            IrrigationType::Hose => "Hose/Sprinkler",
            IrrigationType::None => "None",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "inground" | "in-ground" | "in ground" => Some(IrrigationType::InGround),
            "hose" | "sprinkler" | "hose/sprinkler" => Some(IrrigationType::Hose),
            "none" => Some(IrrigationType::None),
            _ => None,
        }
    }
}

impl std::fmt::Display for IrrigationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawnProfile {
    pub id: Option<i64>,
    pub name: String,
    pub grass_type: GrassType,
    pub usda_zone: String,
    pub soil_type: Option<SoilType>,
    pub lawn_size_sqft: Option<f64>,
    pub irrigation_type: Option<IrrigationType>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl LawnProfile {
    pub fn new(name: String, grass_type: GrassType, usda_zone: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: None,
            name,
            grass_type,
            usda_zone,
            soil_type: None,
            lawn_size_sqft: None,
            irrigation_type: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl Default for LawnProfile {
    fn default() -> Self {
        Self::new(
            "Main Lawn".to_string(),
            GrassType::TallFescue,
            "7a".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grass_type_from_str_valid() {
        assert_eq!(
            GrassType::from_str("TallFescue"),
            Some(GrassType::TallFescue)
        );
        assert_eq!(
            GrassType::from_str("tall fescue"),
            Some(GrassType::TallFescue)
        );
        assert_eq!(GrassType::from_str("TTTF"), Some(GrassType::TallFescue));
        assert_eq!(
            GrassType::from_str("KentuckyBluegrass"),
            Some(GrassType::KentuckyBluegrass)
        );
        assert_eq!(
            GrassType::from_str("kbg"),
            Some(GrassType::KentuckyBluegrass)
        );
        assert_eq!(GrassType::from_str("bermuda"), Some(GrassType::Bermuda));
        assert_eq!(GrassType::from_str("zoysia"), Some(GrassType::Zoysia));
    }

    #[test]
    fn grass_type_from_str_invalid() {
        assert_eq!(GrassType::from_str("unknown"), None);
        assert_eq!(GrassType::from_str(""), None);
        assert_eq!(GrassType::from_str("fescue"), None);
    }

    #[test]
    fn grass_type_round_trip() {
        // Test that Debug format round-trips through from_str
        for grass_type in [
            GrassType::TallFescue,
            GrassType::KentuckyBluegrass,
            GrassType::PerennialRyegrass,
            GrassType::FineFescue,
            GrassType::Bermuda,
            GrassType::Zoysia,
            GrassType::StAugustine,
            GrassType::Mixed,
        ] {
            let debug_str = format!("{:?}", grass_type);
            assert_eq!(
                GrassType::from_str(&debug_str),
                Some(grass_type),
                "Round-trip failed for {:?}",
                grass_type
            );
        }
    }

    #[test]
    fn grass_type_is_cool_season() {
        assert!(GrassType::TallFescue.is_cool_season());
        assert!(GrassType::KentuckyBluegrass.is_cool_season());
        assert!(GrassType::PerennialRyegrass.is_cool_season());
        assert!(GrassType::FineFescue.is_cool_season());
        assert!(!GrassType::Bermuda.is_cool_season());
        assert!(!GrassType::Zoysia.is_cool_season());
        assert!(!GrassType::StAugustine.is_cool_season());
    }

    #[test]
    fn soil_type_from_str_valid() {
        assert_eq!(SoilType::from_str("clay"), Some(SoilType::Clay));
        assert_eq!(SoilType::from_str("Loam"), Some(SoilType::Loam));
        assert_eq!(SoilType::from_str("SANDY"), Some(SoilType::Sandy));
        assert_eq!(SoilType::from_str("silt loam"), Some(SoilType::SiltLoam));
        assert_eq!(SoilType::from_str("SiltLoam"), Some(SoilType::SiltLoam));
    }

    #[test]
    fn soil_type_from_str_invalid() {
        assert_eq!(SoilType::from_str("dirt"), None);
        assert_eq!(SoilType::from_str(""), None);
    }

    #[test]
    fn irrigation_type_from_str_valid() {
        assert_eq!(
            IrrigationType::from_str("InGround"),
            Some(IrrigationType::InGround)
        );
        assert_eq!(
            IrrigationType::from_str("in-ground"),
            Some(IrrigationType::InGround)
        );
        assert_eq!(IrrigationType::from_str("hose"), Some(IrrigationType::Hose));
        assert_eq!(IrrigationType::from_str("none"), Some(IrrigationType::None));
    }

    #[test]
    fn irrigation_type_from_str_invalid() {
        assert_eq!(IrrigationType::from_str("drip"), None);
        assert_eq!(IrrigationType::from_str(""), None);
    }
}
