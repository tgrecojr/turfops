use crate::models::SoilType;

// Mehlich-3 nutrient thresholds (ppm)
pub const PHOSPHORUS_LOW: f64 = 15.0;
pub const PHOSPHORUS_HIGH: f64 = 50.0;
pub const POTASSIUM_LOW: f64 = 100.0;
pub const POTASSIUM_HIGH: f64 = 175.0;
pub const CALCIUM_LOW: f64 = 500.0;
pub const MAGNESIUM_LOW: f64 = 50.0;
pub const SULFUR_LOW: f64 = 10.0;
pub const IRON_LOW: f64 = 5.0;
pub const MANGANESE_LOW: f64 = 5.0;
pub const ZINC_LOW: f64 = 1.0;
pub const BORON_LOW: f64 = 0.5;
pub const COPPER_LOW: f64 = 0.5;

// pH targets
pub const PH_TARGET_COOL_SEASON: f64 = 6.5;
pub const PH_TARGET_WARM_SEASON: f64 = 6.0;

// Max application rates per treatment
pub const MAX_LIME_LBS_PER_1000SQFT: f64 = 50.0;
pub const MAX_SULFUR_LBS_PER_1000SQFT: f64 = 10.0;

/// Lime rate in lbs per 1000 sqft per pH unit to raise pH.
pub fn lime_rate_per_ph_unit(soil_type: SoilType) -> f64 {
    match soil_type {
        SoilType::Sandy => 25.0,
        SoilType::SandyLoam => 35.0,
        SoilType::Loam => 50.0,
        SoilType::SiltLoam => 60.0,
        SoilType::ClayLoam => 70.0,
        SoilType::Clay => 80.0,
    }
}

/// Sulfur rate in lbs per 1000 sqft per pH unit to lower pH.
pub fn sulfur_rate_per_ph_unit(soil_type: SoilType) -> f64 {
    match soil_type {
        SoilType::Sandy => 5.0,
        SoilType::SandyLoam => 7.0,
        SoilType::Loam => 10.0,
        SoilType::SiltLoam => 12.0,
        SoilType::ClayLoam => 15.0,
        SoilType::Clay => 18.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lime_rates_increase_with_clay_content() {
        assert!(lime_rate_per_ph_unit(SoilType::Sandy) < lime_rate_per_ph_unit(SoilType::Loam));
        assert!(lime_rate_per_ph_unit(SoilType::Loam) < lime_rate_per_ph_unit(SoilType::Clay));
    }

    #[test]
    fn sulfur_rates_increase_with_clay_content() {
        assert!(sulfur_rate_per_ph_unit(SoilType::Sandy) < sulfur_rate_per_ph_unit(SoilType::Loam));
        assert!(sulfur_rate_per_ph_unit(SoilType::Loam) < sulfur_rate_per_ph_unit(SoilType::Clay));
    }
}
