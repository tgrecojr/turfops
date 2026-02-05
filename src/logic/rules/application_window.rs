use super::Rule;
use crate::models::{
    Application, EnvironmentalSummary, LawnProfile, Recommendation, RecommendationCategory,
    Severity,
};
use chrono::{Datelike, NaiveDate};

/// Application window rule - identifies optimal windows for chemical applications
///
/// Good application conditions:
/// - No rain for 24h before AND 48h after
/// - Temperature 50-80°F
/// - Wind < 10mph
/// - Humidity < 85%
///
/// This rule provides advisory-level recommendations when good windows exist.
pub struct ApplicationWindowRule;

impl Rule for ApplicationWindowRule {
    fn id(&self) -> &'static str {
        "application_window"
    }

    fn name(&self) -> &'static str {
        "Optimal Application Window"
    }

    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        _profile: &LawnProfile,
        _history: &[Application],
    ) -> Option<Recommendation> {
        let forecast = env.forecast.as_ref()?;

        // Look for a good application window in the next 5 days
        let good_days: Vec<(NaiveDate, WindowQuality)> = forecast
            .daily_summary
            .iter()
            .take(5)
            .filter_map(|day| {
                let quality = self.assess_day_quality(day, env);
                if quality.is_good() {
                    Some((day.date, quality))
                } else {
                    None
                }
            })
            .collect();

        if good_days.is_empty() {
            // No good windows - provide advisory about current conditions
            return None;
        }

        // Find the best day
        let best = good_days
            .iter()
            .max_by_key(|(_, q)| q.score())
            .map(|(date, quality)| (*date, quality.clone()))?;

        Some(self.build_recommendation(&best.0, &best.1, good_days.len()))
    }
}

#[derive(Debug, Clone)]
struct WindowQuality {
    temp_ok: bool,
    wind_ok: bool,
    humidity_ok: bool,
    no_rain_before: bool,
    no_rain_after: bool,
    temp: f64,
    wind: f64,
    humidity: f64,
}

impl WindowQuality {
    fn is_good(&self) -> bool {
        // Must have no rain and acceptable temp
        self.no_rain_before && self.no_rain_after && self.temp_ok
    }

    fn score(&self) -> u32 {
        let mut score = 0;
        if self.temp_ok {
            score += 10;
        }
        if self.wind_ok {
            score += 5;
        }
        if self.humidity_ok {
            score += 3;
        }
        if self.no_rain_before {
            score += 10;
        }
        if self.no_rain_after {
            score += 15;
        }
        // Bonus for ideal temp range
        if self.temp >= 55.0 && self.temp <= 75.0 {
            score += 5;
        }
        score
    }

    fn describe(&self) -> String {
        let mut conditions = Vec::new();

        if self.temp >= 55.0 && self.temp <= 75.0 {
            conditions.push("ideal temps");
        } else if self.temp_ok {
            conditions.push("acceptable temps");
        }

        if self.wind_ok && self.wind < 5.0 {
            conditions.push("calm winds");
        } else if self.wind_ok {
            conditions.push("light winds");
        }

        if self.humidity_ok && self.humidity < 70.0 {
            conditions.push("low humidity");
        } else if self.humidity_ok {
            conditions.push("moderate humidity");
        }

        if conditions.is_empty() {
            "marginal conditions".to_string()
        } else {
            conditions.join(", ")
        }
    }
}

impl ApplicationWindowRule {
    fn assess_day_quality(
        &self,
        day: &crate::models::DailyForecast,
        env: &EnvironmentalSummary,
    ) -> WindowQuality {
        let forecast = env.forecast.as_ref().unwrap();

        // Check temp range (50-80°F)
        let avg_temp = (day.high_temp_f + day.low_temp_f) / 2.0;
        let temp_ok = avg_temp >= 50.0 && day.high_temp_f <= 85.0;

        // Check wind (<10mph)
        let wind_ok = day.avg_wind_speed_mph < 10.0;

        // Check humidity (<85%)
        let humidity_ok = day.avg_humidity < 85.0;

        // Check for rain (need dry 24h before and 48h after)
        let day_idx = forecast
            .daily_summary
            .iter()
            .position(|d| d.date == day.date)
            .unwrap_or(0);

        // Check day before
        let no_rain_before = if day_idx > 0 {
            forecast.daily_summary[day_idx - 1].total_precipitation_mm < 2.5
                && forecast.daily_summary[day_idx - 1].max_precipitation_prob < 0.5
        } else {
            // Check recent precipitation from environmental data
            env.precipitation_7day_total_mm.unwrap_or(0.0) < 25.0
        };

        // Check current day and next day
        let current_dry = day.total_precipitation_mm < 2.5 && day.max_precipitation_prob < 0.5;
        let next_day_dry = forecast
            .daily_summary
            .get(day_idx + 1)
            .map(|d| d.total_precipitation_mm < 2.5 && d.max_precipitation_prob < 0.5)
            .unwrap_or(true);

        let no_rain_after = current_dry && next_day_dry;

        WindowQuality {
            temp_ok,
            wind_ok,
            humidity_ok,
            no_rain_before,
            no_rain_after,
            temp: avg_temp,
            wind: day.avg_wind_speed_mph,
            humidity: day.avg_humidity,
        }
    }

    fn build_recommendation(
        &self,
        date: &NaiveDate,
        quality: &WindowQuality,
        total_good_days: usize,
    ) -> Recommendation {
        let day_name = match date.weekday() {
            chrono::Weekday::Mon => "Monday",
            chrono::Weekday::Tue => "Tuesday",
            chrono::Weekday::Wed => "Wednesday",
            chrono::Weekday::Thu => "Thursday",
            chrono::Weekday::Fri => "Friday",
            chrono::Weekday::Sat => "Saturday",
            chrono::Weekday::Sun => "Sunday",
        };

        let title = format!("Good Application Window: {}", day_name);

        let description = format!(
            "{} ({}) shows {} for lawn product applications. \
             {} good day(s) in the next 5-day forecast.",
            day_name,
            date.format("%b %d"),
            quality.describe(),
            total_good_days
        );

        Recommendation::new(
            "application_window",
            RecommendationCategory::ApplicationTiming,
            Severity::Info,
            title,
            description,
        )
        .with_explanation(
            "Optimal conditions for fertilizer, herbicide, and fungicide applications include: \
             dry conditions (no rain 24h before, 48h after), moderate temperatures (50-80°F), \
             low wind (<10mph to prevent drift), and moderate humidity (<85%). \
             Early morning applications are often best.",
        )
        .with_data_point(
            "Expected Temp",
            format!("{:.0}°F", quality.temp),
            "OpenWeatherMap",
        )
        .with_data_point(
            "Wind Speed",
            format!("{:.1}mph", quality.wind),
            "OpenWeatherMap",
        )
        .with_data_point(
            "Humidity",
            format!("{:.0}%", quality.humidity),
            "OpenWeatherMap",
        )
        .with_action(format!(
            "Plan applications for {} if weather holds. \
             Check forecast morning-of to confirm conditions. \
             Apply in early morning for best results.",
            day_name
        ))
    }
}
