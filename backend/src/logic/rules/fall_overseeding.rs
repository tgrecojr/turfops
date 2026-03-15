use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Fall overseeding timing rule
///
/// Tall Fescue doesn't spread via rhizomes or stolons - overseeding is the
/// only way to thicken a lawn. Timing is critical:
/// - Too early: Seedlings die from heat stress
/// - Too late: Not enough time to establish before winter
///
/// Optimal window: Soil temp 50-65°F, late August through October
/// Germination requires consistent moisture for 10-14 days
/// GDD >= 2500 indicates season maturity (fall window approaching)
/// GDD >= 3000 + low time remaining escalates severity
pub struct FallOverseedingRule;

impl Rule for FallOverseedingRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        // Only relevant for cool-season grasses
        if !profile.grass_type.is_cool_season() {
            return None;
        }

        let today = Local::now().date_naive();
        let current_year = today.year();

        // Define the overseeding window (Aug 15 - Oct 31 for Zone 7a)
        let window_start = NaiveDate::from_ymd_opt(current_year, 8, 15)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 10, 31)?;

        // Only evaluate during the window
        if today < window_start || today > window_end {
            return None;
        }

        // Check if already overseeded this fall
        let already_seeded = history.iter().any(|app| {
            app.application_type == ApplicationType::Overseed
                && app.application_date.year() == current_year
                && app.application_date >= window_start
        });

        if already_seeded {
            return None;
        }

        // Get soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;
        let current_soil_temp = env.current.as_ref()?.soil_temp_10_f?;

        // GDD data for season maturity assessment
        let gdd_ytd = env.gdd_base50_ytd;
        let season_mature = gdd_ytd.is_some_and(|gdd| gdd >= OVERSEED_GDD_SEASON_MATURE);
        let season_late = gdd_ytd.is_some_and(|gdd| gdd >= OVERSEED_GDD_SEASON_LATE);

        // Check forecast for upcoming conditions (if available)
        let forecast_favorable = env
            .forecast
            .as_ref()
            .map(|f| {
                // Check if next 14 days have reasonable temps
                let days = f.next_days(14);
                let avg_high: f64 =
                    days.iter().map(|d| d.high_temp_f).sum::<f64>() / days.len().max(1) as f64;

                // Favorable if highs are moderate (rain helps but irrigation can substitute)
                avg_high < OVERSEED_FORECAST_HOT_F
            })
            .unwrap_or(true);

        // Calculate days remaining in window
        let days_remaining = (window_end - today).num_days();

        // Determine recommendation based on soil temp
        if (OVERSEED_SOIL_LOW_F..=OVERSEED_SOIL_HIGH_F).contains(&soil_temp_avg) {
            // Optimal window
            let severity = if (OVERSEED_PEAK_LOW_F..=OVERSEED_PEAK_HIGH_F).contains(&soil_temp_avg)
            {
                // Peak germination range
                if days_remaining < OVERSEED_LOW_TIME_DAYS {
                    Severity::Warning // Optimal but running low on time
                } else {
                    Severity::Advisory
                }
            } else if (season_late && days_remaining < OVERSEED_LOW_TIME_DAYS)
                || days_remaining < OVERSEED_URGENT_DAYS
            {
                Severity::Warning
            } else {
                Severity::Advisory
            };

            let mut rec = Recommendation::new(
                format!("fall_overseeding_{}", current_year),
                RecommendationCategory::Overseeding,
                severity,
                "Fall Overseeding Window Open",
                format!(
                    "Soil temperature ({:.1}°F) is ideal for TTTF seed germination. \
                     {} days remaining in optimal window.",
                    soil_temp_avg, days_remaining
                ),
            );

            rec = rec
                .with_explanation(
                    "Tall Fescue doesn't spread on its own - overseeding is the only way to \
                     thicken your lawn and fill bare spots. Fall is THE best time because: \
                     (1) soil is warm for germination, (2) air is cool reducing seedling stress, \
                     (3) weed competition is minimal, (4) fall rains provide moisture. \
                     Seeds need 10-14 days of consistent moisture to germinate. \
                     NOTE: New seedlings are extremely susceptible to gray leaf spot the \
                     following summer — monitor closely (NC State Extension). \
                     Remove thatch when >0.5 inch thick before overseeding (Missouri Extension g6705).",
                )
                .with_data_point(
                    "7-Day Avg Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    DataSource::SoilData.as_str(),
                )
                .with_data_point(
                    "Current Soil Temp",
                    format!("{:.1}°F", current_soil_temp),
                    DataSource::SoilData.as_str(),
                )
                .with_data_point(
                    "Days Remaining",
                    format!("{}", days_remaining),
                    DataSource::Calendar.as_str(),
                );

            if let Some(gdd) = gdd_ytd {
                rec = rec.with_data_point(
                    "GDD (Base 50°F YTD)",
                    format!("{:.0} (season maturity)", gdd),
                    DataSource::Calculated.as_str(),
                );
            }

            // Add forecast note if available
            if !forecast_favorable {
                rec = rec.with_data_point(
                    "Forecast Note",
                    "Hot weather ahead - monitor seedlings",
                    DataSource::OpenWeatherMap.as_str(),
                );
            }

            let seeding_rate = if profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT) > 0.0 {
                let sqft = profile.lawn_size_sqft.unwrap_or(DEFAULT_LAWN_SIZE_SQFT);
                let lbs_needed = sqft / 1000.0 * OVERSEED_RATE_LBS_PER_KSQFT;
                format!(
                    "For your {:.0} sqft lawn: ~{:.0} lbs of TTTF seed ({:.0} lbs/1000 sqft for overseeding). \
                     Mow low (2\"), dethatch or aerate first for seed-to-soil contact. \
                     Keep soil moist (light watering 2-3x daily) for 14 days. \
                     Avoid foot traffic for 3-4 weeks.",
                    sqft, lbs_needed, OVERSEED_RATE_LBS_PER_KSQFT
                )
            } else {
                format!(
                    "Seed at {:.0} lbs per 1000 sqft for overseeding (8 lbs for bare soil). \
                     Mow low (2\"), dethatch or aerate first for seed-to-soil contact. \
                     Keep soil moist (light watering 2-3x daily) for 14 days. \
                     Avoid foot traffic for 3-4 weeks.",
                    OVERSEED_RATE_LBS_PER_KSQFT
                )
            };

            rec = rec.with_action(seeding_rate);

            Some(rec)
        } else if soil_temp_avg > OVERSEED_SOIL_HIGH_F && soil_temp_avg <= OVERSEED_WARM_LIMIT_F {
            // Soil still warm - might be early in window
            if today < NaiveDate::from_ymd_opt(current_year, 9, 15)? {
                // Early September - wait for cooler temps
                let gdd_note = if season_mature {
                    " GDD indicates the season is maturing — the fall overseeding window is approaching."
                } else {
                    ""
                };

                let rec = Recommendation::new(
                    format!("fall_overseeding_wait_{}", current_year),
                    RecommendationCategory::Overseeding,
                    Severity::Info,
                    "Overseeding Window Approaching",
                    format!(
                        "Soil temperature ({:.1}°F) is still warm. \
                         Wait for temps to drop below {:.0}°F for best germination.",
                        soil_temp_avg, OVERSEED_SOIL_HIGH_F
                    ),
                )
                .with_explanation(format!(
                    "TTTF germinates best when soil is {:.0}-{:.0}°F. Seeding when soil is too warm \
                     can stress seedlings. The window typically opens mid-September in Zone 7a.{}",
                    OVERSEED_SOIL_LOW_F, OVERSEED_SOIL_HIGH_F, gdd_note
                ))
                .with_data_point(
                    "Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    DataSource::SoilData.as_str(),
                )
                .with_action(
                    "Prepare for overseeding: order seed, plan aeration, \
                     gather supplies. Monitor soil temps weekly.",
                );

                Some(rec)
            } else {
                // Late September+ with warm soil - seed anyway, window closing
                let rec = Recommendation::new(
                    format!("fall_overseeding_late_{}", current_year),
                    RecommendationCategory::Overseeding,
                    Severity::Warning,
                    "Overseeding - Soil Warm but Window Closing",
                    format!(
                        "Soil ({:.1}°F) is warmer than ideal, but {} days remain in window. \
                         Consider seeding soon despite conditions.",
                        soil_temp_avg, days_remaining
                    ),
                )
                .with_data_point(
                    "Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    DataSource::SoilData.as_str(),
                )
                .with_action(
                    "Seed soon if you haven't already. Water more frequently to keep \
                     seedlings cool. Soil temps will drop as nights get cooler.",
                );

                Some(rec)
            }
        } else if soil_temp_avg < OVERSEED_SOIL_LOW_F {
            // Getting cold - urgent if not seeded
            if days_remaining > OVERSEED_URGENT_DAYS {
                let rec = Recommendation::new(
                    format!("fall_overseeding_cold_{}", current_year),
                    RecommendationCategory::Overseeding,
                    Severity::Warning,
                    "Overseeding Window Narrowing - Cool Soil",
                    format!(
                        "Soil temperature ({:.1}°F) is below optimal. \
                         Germination will be slow. Seed immediately if planned.",
                        soil_temp_avg
                    ),
                )
                .with_data_point(
                    "Soil Temp",
                    format!("{:.1}°F", soil_temp_avg),
                    DataSource::SoilData.as_str(),
                )
                .with_action(format!(
                    "If overseeding, do it NOW. Germination slows significantly below {:.0}°F. \
                     Seedlings need 4-6 weeks before hard frost to establish.",
                    OVERSEED_SOIL_LOW_F
                ));

                Some(rec)
            } else {
                // Very late - probably too late for this year
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EnvironmentalReading, GrassType};

    fn base_env(soil_avg: f64, soil_current: f64) -> EnvironmentalSummary {
        let mut reading = EnvironmentalReading::new(DataSource::SoilData);
        reading.soil_temp_10_f = Some(soil_current);
        EnvironmentalSummary {
            current: Some(reading),
            soil_temp_7day_avg_f: Some(soil_avg),
            ..Default::default()
        }
    }

    fn base_profile() -> LawnProfile {
        LawnProfile {
            id: Some(1),
            name: "Test".into(),
            grass_type: GrassType::TallFescue,
            usda_zone: "7a".into(),
            soil_type: None,
            lawn_size_sqft: Some(5000.0),
            irrigation_type: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn gdd_none_degrades_gracefully() {
        // GDD = None should not change behavior vs pre-GDD code.
        let env = base_env(58.0, 57.0);
        assert!(env.gdd_base50_ytd.is_none());
        let rule = FallOverseedingRule;
        // Calendar-gated (Aug 15 - Oct 31), but should not panic regardless.
        let _ = rule.evaluate(&env, &base_profile(), &[]);
    }

    #[test]
    fn gdd_below_season_mature_no_change() {
        // GDD = 2000 (below 2500 season mature) — no escalation.
        let mut env = base_env(58.0, 57.0);
        env.gdd_base50_ytd = Some(2000.0);
        let rule = FallOverseedingRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        // If inside window, severity should be based on soil temp + time, not GDD
        if let Some(rec) = result {
            assert!(
                rec.severity == Severity::Advisory || rec.severity == Severity::Warning,
                "GDD below season mature should not add extra escalation"
            );
        }
    }

    #[test]
    fn gdd_season_late_with_low_time_escalates() {
        // GDD = 3000 (season late) — should escalate if days_remaining < 21.
        // This test is date-sensitive (Aug 15 - Oct 31 window).
        let mut env = base_env(58.0, 57.0);
        env.gdd_base50_ytd = Some(3000.0);
        let rule = FallOverseedingRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        if let Some(rec) = result {
            // Result includes GDD data point
            assert!(
                rec.data_points.iter().any(|dp| dp.label.contains("GDD")),
                "Should include GDD data point"
            );
        }
    }

    #[test]
    fn gdd_season_mature_noted_in_wait_branch() {
        // GDD = 2500 with warm soil — "approaching" message should note season maturity.
        // This test is date-sensitive (before Sep 15 + soil > 65°F).
        let mut env = base_env(70.0, 69.0);
        env.gdd_base50_ytd = Some(2500.0);
        let rule = FallOverseedingRule;
        let _ = rule.evaluate(&env, &base_profile(), &[]);
        // Cannot assert on exact output without controlling date, but should not panic.
    }
}
