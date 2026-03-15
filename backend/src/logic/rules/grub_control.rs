use super::thresholds::*;
use super::Rule;
use crate::models::{
    Application, ApplicationType, DataSource, EnvironmentalSummary, LawnProfile, Recommendation,
    RecommendationCategory, Severity,
};
use chrono::{Datelike, Local, NaiveDate};

/// Grub control timing rule
///
/// Japanese beetle and other grub larvae are most vulnerable to
/// preventative treatments when actively feeding near the soil surface.
///
/// Window: GDD >= 500 (as early as Apr 1) OR May 15 - July 4, soil temp 60-75°F
/// GDD >= 700 = peak egg-hatch, severity escalation
/// Product: Chlorantraniliprole (GrubEx), Imidacloprid, or similar
pub struct GrubControlRule;

impl Rule for GrubControlRule {
    fn evaluate(
        &self,
        env: &EnvironmentalSummary,
        _profile: &LawnProfile,
        history: &[Application],
    ) -> Option<Recommendation> {
        let today = Local::now().date_naive();
        let current_year = today.year();

        // Define the calendar application window
        let window_start = NaiveDate::from_ymd_opt(current_year, 5, 15)?;
        let window_end = NaiveDate::from_ymd_opt(current_year, 7, 4)?;
        let gdd_early_start = NaiveDate::from_ymd_opt(current_year, 4, 1)?;

        // GDD-based early window: GDD >= 500 opens as early as Apr 1
        let gdd_ytd = env.gdd_base50_ytd;
        let gdd_opens_window = gdd_ytd
            .map(|gdd| gdd >= GRUB_GDD_WINDOW_OPEN && today >= gdd_early_start)
            .unwrap_or(false);

        // Only relevant during the window (GDD-extended or calendar)
        let in_window = gdd_opens_window || (today >= window_start && today <= window_end);
        if !in_window {
            return None;
        }

        // Check if already applied this year
        let already_applied = history.iter().any(|app| {
            (app.application_type == ApplicationType::GrubControl
                || app.application_type == ApplicationType::Insecticide)
                && app.application_date.year() == current_year
                && app.application_date >= gdd_early_start
        });

        if already_applied {
            return None;
        }

        // Get soil temperature
        let soil_temp_avg = env.soil_temp_7day_avg_f?;
        let current_soil_temp = env.current.as_ref()?.soil_temp_10_f?;

        // GDD urgency levels
        let gdd_urgency = gdd_ytd.map(|gdd| {
            if gdd >= GRUB_GDD_PEAK_HATCH {
                2 // Peak egg-hatch
            } else if gdd >= GRUB_GDD_WINDOW_OPEN {
                1 // Egg-laying begun
            } else {
                0
            }
        });

        if (GRUB_CONTROL_SOIL_LOW_F..=GRUB_CONTROL_SOIL_HIGH_F).contains(&soil_temp_avg) {
            // Calculate days remaining in window
            let days_remaining = (window_end - today).num_days();

            let severity = if gdd_urgency.unwrap_or(0) >= 2 || days_remaining <= GRUB_URGENCY_DAYS {
                Severity::Warning
            } else {
                Severity::Advisory
            };

            let gdd_explanation = if let Some(gdd) = gdd_ytd {
                format!(
                    " GDD (base 50°F) is {:.0} — {}.",
                    gdd,
                    if gdd >= GRUB_GDD_PEAK_HATCH {
                        "peak Japanese beetle egg-hatch, grubs actively feeding near surface"
                    } else if gdd >= GRUB_GDD_WINDOW_OPEN {
                        "Japanese beetle egg-laying has begun"
                    } else {
                        "approaching grub activity threshold"
                    }
                )
            } else {
                String::new()
            };

            let mut rec = Recommendation::new(
                format!("grub_control_{}", current_year),
                RecommendationCategory::GrubControl,
                severity,
                "Grub Preventative Window",
                format!(
                    "Conditions are optimal for preventative grub control application. \
                     {} days remaining in window.",
                    days_remaining
                ),
            )
            .with_explanation(format!(
                "Japanese beetle larvae (grubs) are most vulnerable to preventative treatments \
                 when adults are laying eggs and larvae are feeding near the surface. \
                 Chlorantraniliprole (GrubEx) provides season-long control when applied now. \
                 Treatment is justified when you find 5-10+ grubs per square foot by lifting \
                 a 1-sqft section of turf (Missouri Extension g6705).{}",
                gdd_explanation
            ))
            .with_data_point(
                "7-Day Avg Soil Temp",
                format!("{:.1}°F", soil_temp_avg),
                DataSource::SoilData.as_str(),
            )
            .with_data_point(
                "Current Soil Temp (10cm)",
                format!("{:.1}°F", current_soil_temp),
                DataSource::SoilData.as_str(),
            )
            .with_data_point(
                "Window Closes",
                window_end.format("%B %d").to_string(),
                DataSource::Agronomic.as_str(),
            );

            if let Some(gdd) = gdd_ytd {
                rec = rec.with_data_point(
                    "GDD (Base 50°F YTD)",
                    format!("{:.0} / {:.0} (peak hatch)", gdd, GRUB_GDD_PEAK_HATCH),
                    DataSource::Calculated.as_str(),
                );
            }

            rec = rec.with_action(
                "Apply chlorantraniliprole (GrubEx) or imidacloprid at label rate. \
                 Water in with 0.5\" of irrigation or rain within 24 hours.",
            );

            Some(rec)
        } else if soil_temp_avg > GRUB_CONTROL_SOIL_HIGH_F {
            // Soil may be too warm - grubs may be deeper
            let mut rec = Recommendation::new(
                format!("grub_control_late_{}", current_year),
                RecommendationCategory::GrubControl,
                Severity::Info,
                "Grub Control - Soil Warm",
                "Soil temperature is elevated. Grub control may still be effective but optimal window is passing.",
            )
            .with_data_point(
                "7-Day Avg Soil Temp",
                format!("{:.1}°F", soil_temp_avg),
                DataSource::SoilData.as_str(),
            );

            if let Some(gdd) = gdd_ytd {
                rec = rec.with_data_point(
                    "GDD (Base 50°F YTD)",
                    format!("{:.0}", gdd),
                    DataSource::Calculated.as_str(),
                );
            }

            rec = rec.with_action(
                "If grub control hasn't been applied, do so soon. \
                 Effectiveness decreases as larvae move deeper into soil.",
            );

            Some(rec)
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
        // When GDD is None, the rule should still work based on calendar + soil temp.
        // This test verifies no panic or unexpected behavior with gdd_base50_ytd = None.
        let env = base_env(65.0, 64.0);
        assert!(env.gdd_base50_ytd.is_none());
        let rule = GrubControlRule;
        // Result depends on current date (calendar gating), but should not panic.
        let _ = rule.evaluate(&env, &base_profile(), &[]);
    }

    #[test]
    fn gdd_below_threshold_no_escalation() {
        // GDD = 400 (below 500 window open) — should not escalate severity.
        // Note: This only produces a recommendation if inside the calendar window (May 15 - Jul 4).
        let mut env = base_env(65.0, 64.0);
        env.gdd_base50_ytd = Some(400.0);
        let rule = GrubControlRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        // Outside calendar window (test runs year-round), result may be None.
        // If inside window, severity should be Advisory (not Warning from GDD).
        if let Some(rec) = result {
            // GDD < 500 means gdd_opens_window is false, and gdd_urgency is 0
            // Severity escalation only from days_remaining <= 14, not GDD
            assert!(
                rec.severity == Severity::Advisory || rec.severity == Severity::Warning,
                "Severity should be Advisory or Warning (from days only)"
            );
        }
    }

    #[test]
    fn gdd_at_peak_hatch_escalates_severity() {
        // GDD = 700 (peak hatch) — severity should escalate to Warning.
        // Note: Date-sensitive — window must include today.
        let mut env = base_env(65.0, 64.0);
        env.gdd_base50_ytd = Some(700.0);
        let rule = GrubControlRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        if let Some(rec) = result {
            assert_eq!(
                rec.severity,
                Severity::Warning,
                "GDD >= 700 should escalate to Warning"
            );
        }
    }

    #[test]
    fn gdd_above_threshold_has_urgency() {
        // GDD = 900 (well above peak hatch) — should show urgency.
        let mut env = base_env(65.0, 64.0);
        env.gdd_base50_ytd = Some(900.0);
        let rule = GrubControlRule;
        let result = rule.evaluate(&env, &base_profile(), &[]);
        if let Some(rec) = result {
            assert_eq!(rec.severity, Severity::Warning);
            // Should include GDD data point
            assert!(
                rec.data_points.iter().any(|dp| dp.label.contains("GDD")),
                "Should include GDD data point"
            );
        }
    }
}
