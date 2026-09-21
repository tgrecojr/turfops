use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};

/// Irrigation forecast rule - recommends irrigation based on forecast drought
///
/// Conditions:
/// - No significant rain (<0.1") forecasted for next 5 days
/// - Current soil moisture below threshold
///
/// Severity levels:
/// - Advisory: No rain 5 days, moisture 0.15-0.20
/// - Warning: No rain 5 days, moisture 0.10-0.15
/// - Critical: No rain 5 days, moisture < 0.10
pub struct IrrigationForecastRule;

impl Rule for IrrigationForecastRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        let forecast = env.forecast.as_ref()?;
        let current = env.current.as_ref()?;
        let soil_moisture = current.primary_soil_moisture()?;

        // Skip if soil moisture is adequate
        if soil_moisture >= SOIL_MOISTURE_ADEQUATE {
            return None;
        }

        // Only the forecast *amount* counts: a 50% chance of a trace won't relieve dry soil.
        // Calculate total precipitation expected in next 5 days
        let total_precip: f64 = forecast
            .next_days(5)
            .iter()
            .map(|d| d.total_precipitation_mm)
            .sum();

        // If meaningful rain expected (>2.5mm / 0.1"), no recommendation
        if total_precip > PRECIP_TRACE_MM {
            return None;
        }

        // Determine severity based on soil moisture
        let severity = if soil_moisture < SOIL_MOISTURE_DROUGHT {
            Severity::Critical
        } else if soil_moisture < SOIL_MOISTURE_IRRIGATION_WARNING {
            Severity::Warning
        } else {
            Severity::Advisory
        };

        // Calculate days until next rain (if any)
        let dry_days = forecast
            .daily_summary
            .iter()
            .take_while(|d| {
                d.total_precipitation_mm < PRECIP_TRACE_MM
                    && d.max_precipitation_prob < PRECIP_PROB_LIKELY
            })
            .count();

        Some(self.build_recommendation(severity, soil_moisture, dry_days, profile))
    }
}

impl IrrigationForecastRule {
    fn build_recommendation(
        &self,
        severity: Severity,
        soil_moisture: f64,
        dry_days: usize,
        _profile: &LawnProfile,
    ) -> Recommendation {
        let title = match severity {
            Severity::Critical => "Irrigation Urgently Needed",
            Severity::Warning => "Irrigation Recommended Soon",
            _ => "Consider Irrigation",
        };

        let description = format!(
            "Soil moisture is low ({:.0}%) and no significant rain is forecasted for {} days. \
             Cool-season grasses need consistent moisture.",
            soil_moisture * 100.0,
            dry_days
        );

        let action = match severity {
            Severity::Critical => {
                "Water immediately. Apply 1-1.5 inches over the next 2-3 days to prevent \
                 drought stress. Water early morning (4-7 AM) to minimize evaporation and disease."
            }
            Severity::Warning => {
                "Plan to irrigate within the next 1-2 days. Apply 1 inch of water per session. \
                 Deep, infrequent watering encourages deeper root growth — better than shallow daily watering."
            }
            _ => {
                "Monitor soil moisture and plan irrigation if conditions don't change. \
                 Consider a deep watering session in early morning."
            }
        };

        Recommendation::new(
            "irrigation_forecast",
            RecommendationCategory::Irrigation,
            severity,
            title,
            description,
        )
        .with_explanation(
            "Tall Fescue requires 1-1.5 inches of water per week during the growing season. \
             When soil moisture drops below 10-15% and no rain is expected, supplemental \
             irrigation prevents drought stress, thinning, and weed invasion. Water deeply \
             (to 6 inches) to encourage deep root growth.",
        )
        .with_data_point(
            "Soil Moisture",
            format!("{:.0}%", soil_moisture * 100.0),
            DataSource::SoilData.as_str(),
        )
        .with_data_point(
            "Dry Days Forecast",
            format!("{} days", dry_days),
            DataSource::OpenWeatherMap.as_str(),
        )
        .with_action(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        EnvironmentalReading, ForecastLocation, ForecastPoint, GrassType, WeatherCondition,
        WeatherForecast,
    };
    use chrono::{Duration, Utc};

    fn profile() -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type: GrassType::TallFescue,
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Dry soil (8%) plus a forecast of one 3-hourly point per `(mm, probability)`.
    fn env(points: &[(f64, f64)]) -> EnvironmentalSummary {
        let mut reading = EnvironmentalReading::new(DataSource::SoilData);
        reading.soil_moisture_10 = Some(0.08);
        let hourly = points
            .iter()
            .enumerate()
            .map(|(i, (mm, prob))| ForecastPoint {
                timestamp: Utc::now() + Duration::hours(3 * (i as i64 + 1)),
                temp_f: 80.0,
                feels_like_f: 80.0,
                humidity_percent: 50.0,
                precipitation_mm: *mm,
                precipitation_prob: *prob,
                wind_speed_mph: 5.0,
                wind_gust_mph: None,
                cloud_cover_percent: 20.0,
                weather_condition: WeatherCondition::default(),
            })
            .collect();
        EnvironmentalSummary {
            current: Some(reading),
            forecast: Some(WeatherForecast {
                fetched_at: Utc::now(),
                location: ForecastLocation {
                    city: "Test".into(),
                    country: "US".into(),
                    latitude: 0.0,
                    longitude: 0.0,
                    utc_offset_seconds: 0,
                },
                hourly,
                daily_summary: Vec::new(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn a_trace_or_a_coin_flip_does_not_cancel_a_drought_alert() {
        // 0.3 mm of drizzle and one 50% slot with no accumulation: still bone dry.
        let rec = IrrigationForecastRule
            .evaluate(&env(&[(0.3, 0.2), (0.0, 0.5)]), &profile(), &[])
            .expect("dry soil with no meaningful rain needs irrigation");
        assert_eq!(rec.severity, Severity::Critical);
    }
}
