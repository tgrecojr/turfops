use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
}

impl FromStr for GrassType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "kentuckybluegrass" | "kentucky bluegrass" | "kbg" => Ok(GrassType::KentuckyBluegrass),
            "tallfescue" | "tall fescue" | "tttf" => Ok(GrassType::TallFescue),
            "perennialryegrass" | "perennial ryegrass" | "prg" => Ok(GrassType::PerennialRyegrass),
            "finefescue" | "fine fescue" => Ok(GrassType::FineFescue),
            "bermuda" => Ok(GrassType::Bermuda),
            "zoysia" => Ok(GrassType::Zoysia),
            "staugustine" | "st. augustine" | "st augustine" => Ok(GrassType::StAugustine),
            "mixed" => Ok(GrassType::Mixed),
            _ => Err(format!("Unknown grass type: {}", s)),
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
}

impl FromStr for SoilType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "clay" => Ok(SoilType::Clay),
            "loam" => Ok(SoilType::Loam),
            "sandy" => Ok(SoilType::Sandy),
            "siltloam" | "silt loam" => Ok(SoilType::SiltLoam),
            "clayloam" | "clay loam" => Ok(SoilType::ClayLoam),
            "sandyloam" | "sandy loam" => Ok(SoilType::SandyLoam),
            _ => Err(format!("Unknown soil type: {}", s)),
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
}

impl FromStr for IrrigationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "inground" | "in-ground" | "in ground" => Ok(IrrigationType::InGround),
            "hose" | "sprinkler" | "hose/sprinkler" => Ok(IrrigationType::Hose),
            "none" => Ok(IrrigationType::None),
            _ => Err(format!("Unknown irrigation type: {}", s)),
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
        assert_eq!(GrassType::from_str("TallFescue"), Ok(GrassType::TallFescue));
        assert_eq!(
            GrassType::from_str("tall fescue"),
            Ok(GrassType::TallFescue)
        );
        assert_eq!(GrassType::from_str("TTTF"), Ok(GrassType::TallFescue));
        assert_eq!(
            GrassType::from_str("KentuckyBluegrass"),
            Ok(GrassType::KentuckyBluegrass)
        );
        assert_eq!(GrassType::from_str("kbg"), Ok(GrassType::KentuckyBluegrass));
        assert_eq!(GrassType::from_str("bermuda"), Ok(GrassType::Bermuda));
        assert_eq!(GrassType::from_str("zoysia"), Ok(GrassType::Zoysia));
    }

    #[test]
    fn grass_type_from_str_invalid() {
        assert!(GrassType::from_str("unknown").is_err());
        assert!(GrassType::from_str("").is_err());
        assert!(GrassType::from_str("fescue").is_err());
    }

    #[test]
    fn grass_type_round_trip() {
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
                Ok(grass_type),
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
    fn grass_type_parse() {
        assert_eq!("TallFescue".parse::<GrassType>(), Ok(GrassType::TallFescue));
        assert!("unknown".parse::<GrassType>().is_err());
    }

    #[test]
    fn soil_type_from_str_valid() {
        assert_eq!(SoilType::from_str("clay"), Ok(SoilType::Clay));
        assert_eq!(SoilType::from_str("Loam"), Ok(SoilType::Loam));
        assert_eq!(SoilType::from_str("SANDY"), Ok(SoilType::Sandy));
        assert_eq!(SoilType::from_str("silt loam"), Ok(SoilType::SiltLoam));
        assert_eq!(SoilType::from_str("SiltLoam"), Ok(SoilType::SiltLoam));
    }

    #[test]
    fn soil_type_from_str_invalid() {
        assert!(SoilType::from_str("dirt").is_err());
        assert!(SoilType::from_str("").is_err());
    }

    #[test]
    fn irrigation_type_from_str_valid() {
        assert_eq!(
            IrrigationType::from_str("InGround"),
            Ok(IrrigationType::InGround)
        );
        assert_eq!(
            IrrigationType::from_str("in-ground"),
            Ok(IrrigationType::InGround)
        );
        assert_eq!(IrrigationType::from_str("hose"), Ok(IrrigationType::Hose));
        assert_eq!(IrrigationType::from_str("none"), Ok(IrrigationType::None));
    }

    #[test]
    fn irrigation_type_from_str_invalid() {
        assert!(IrrigationType::from_str("drip").is_err());
        assert!(IrrigationType::from_str("").is_err());
    }
}
