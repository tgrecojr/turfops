use crate::models::EnvironmentalReading;

/// Calculate Growing Degree Days (GDD)
/// Base temperature is typically 50°F for cool-season grasses
pub fn calculate_gdd(readings: &[EnvironmentalReading], base_temp_f: f64) -> f64 {
    readings
        .iter()
        .filter_map(|r| r.ambient_temp_f)
        .map(|temp| {
            let gdd = temp - base_temp_f;
            if gdd > 0.0 {
                gdd
            } else {
                0.0
            }
        })
        .sum::<f64>()
        / 24.0 // Convert hourly to daily approximation
}

/// Calculate average soil temperature over a period
pub fn average_soil_temp(readings: &[EnvironmentalReading], depth_cm: u32) -> Option<f64> {
    let temps: Vec<f64> = readings
        .iter()
        .filter_map(|r| match depth_cm {
            5 => r.soil_temp_5_f,
            10 => r.soil_temp_10_f,
            20 => r.soil_temp_20_f,
            50 => r.soil_temp_50_f,
            100 => r.soil_temp_100_f,
            _ => r.soil_temp_10_f, // Default to 10cm
        })
        .collect();

    if temps.is_empty() {
        None
    } else {
        Some(temps.iter().sum::<f64>() / temps.len() as f64)
    }
}

/// Calculate average soil moisture over a period
pub fn average_soil_moisture(readings: &[EnvironmentalReading], depth_cm: u32) -> Option<f64> {
    let moisture: Vec<f64> = readings
        .iter()
        .filter_map(|r| match depth_cm {
            5 => r.soil_moisture_5,
            10 => r.soil_moisture_10,
            20 => r.soil_moisture_20,
            50 => r.soil_moisture_50,
            100 => r.soil_moisture_100,
            _ => r.soil_moisture_10,
        })
        .collect();

    if moisture.is_empty() {
        None
    } else {
        Some(moisture.iter().sum::<f64>() / moisture.len() as f64)
    }
}

/// Calculate total precipitation over a period
pub fn total_precipitation(readings: &[EnvironmentalReading]) -> f64 {
    readings
        .iter()
        .filter_map(|r| r.precipitation_mm)
        .filter(|p| *p >= 0.0)
        .sum()
}

/// Check if humidity has been sustained above threshold
pub fn sustained_high_humidity(
    readings: &[EnvironmentalReading],
    threshold: f64,
    hours: usize,
) -> bool {
    let recent: Vec<f64> = readings
        .iter()
        .take(hours)
        .filter_map(|r| r.humidity_percent)
        .collect();

    if recent.len() < hours / 2 {
        return false; // Not enough data
    }

    recent.iter().all(|h| *h >= threshold)
}

/// Calculate nitrogen applied this season (lbs N per 1000 sqft)
pub fn nitrogen_this_season(
    applications: &[crate::models::Application],
    season_start: chrono::NaiveDate,
) -> f64 {
    use crate::models::ApplicationType;

    applications
        .iter()
        .filter(|a| {
            a.application_type == ApplicationType::Fertilizer && a.application_date >= season_start
        })
        .filter_map(|a| a.rate_per_1000sqft)
        .sum()
}
