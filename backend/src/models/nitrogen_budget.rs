use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::GrassType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitrogenBudget {
    pub year: i32,
    pub target_lbs_per_1000sqft: f64,
    pub applied_lbs_per_1000sqft: f64,
    pub remaining_lbs_per_1000sqft: f64,
    pub percent_of_target: f64,
    pub applications: Vec<NitrogenApplication>,
    pub grass_type_target: GrassTypeNTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitrogenApplication {
    pub date: NaiveDate,
    pub product_name: Option<String>,
    pub nitrogen_pct: f64,
    pub rate_per_1000sqft: f64,
    pub n_lbs_per_1000sqft: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrassTypeNTarget {
    pub grass_type: GrassType,
    pub min_lbs_per_1000sqft: f64,
    pub max_lbs_per_1000sqft: f64,
    pub recommended_lbs_per_1000sqft: f64,
}

/// Get the recommended annual N target for a grass type (lbs N per 1000 sqft per year).
pub fn annual_n_target(grass_type: GrassType) -> GrassTypeNTarget {
    match grass_type {
        GrassType::TallFescue => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 2.0,
            max_lbs_per_1000sqft: 4.0,
            recommended_lbs_per_1000sqft: 3.5,
        },
        GrassType::KentuckyBluegrass => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 3.0,
            max_lbs_per_1000sqft: 5.0,
            recommended_lbs_per_1000sqft: 4.0,
        },
        GrassType::PerennialRyegrass => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 2.0,
            max_lbs_per_1000sqft: 4.0,
            recommended_lbs_per_1000sqft: 3.0,
        },
        GrassType::FineFescue => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 1.0,
            max_lbs_per_1000sqft: 3.0,
            recommended_lbs_per_1000sqft: 2.0,
        },
        GrassType::Bermuda => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 3.0,
            max_lbs_per_1000sqft: 6.0,
            recommended_lbs_per_1000sqft: 4.0,
        },
        GrassType::Zoysia => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 2.0,
            max_lbs_per_1000sqft: 4.0,
            recommended_lbs_per_1000sqft: 3.0,
        },
        GrassType::StAugustine => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 2.0,
            max_lbs_per_1000sqft: 5.0,
            recommended_lbs_per_1000sqft: 3.5,
        },
        GrassType::Mixed => GrassTypeNTarget {
            grass_type,
            min_lbs_per_1000sqft: 2.0,
            max_lbs_per_1000sqft: 4.0,
            recommended_lbs_per_1000sqft: 3.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n_target_tttf() {
        let target = annual_n_target(GrassType::TallFescue);
        assert_eq!(target.recommended_lbs_per_1000sqft, 3.5);
        assert_eq!(target.min_lbs_per_1000sqft, 2.0);
        assert_eq!(target.max_lbs_per_1000sqft, 4.0);
    }

    #[test]
    fn n_target_kbg() {
        let target = annual_n_target(GrassType::KentuckyBluegrass);
        assert_eq!(target.recommended_lbs_per_1000sqft, 4.0);
    }
}
