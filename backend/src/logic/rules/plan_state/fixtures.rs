//! Fixtures for the plan-state tests: a 2026 season, conditions and a feed.

use super::*;
pub(super) use crate::models::{
    ApplicationType, DataSource, EnvironmentalReading, GrassType, RecommendationCategory, Severity,
};
pub(super) use SeasonState::*;

pub(super) fn d(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

/// Conditions: 7-day soil average, year-to-date GDD, live 10 cm soil.
pub(super) fn env(soil: Option<f64>, gdd: Option<f64>, live: Option<f64>) -> EnvironmentalSummary {
    let mut reading = EnvironmentalReading::new(DataSource::SoilData);
    reading.soil_temp_10_f = live;
    EnvironmentalSummary {
        current: Some(reading),
        soil_temp_7day_avg_f: soil,
        gdd_base50_ytd: gdd,
        ..Default::default()
    }
}

pub(super) fn mild() -> EnvironmentalSummary {
    env(Some(58.0), Some(100.0), Some(58.0))
}

pub(super) fn profile() -> LawnProfile {
    LawnProfile {
        id: Some(1),
        name: "Test".into(),
        grass_type: GrassType::TallFescue,
        usda_zone: "7a".into(),
        soil_type: None,
        lawn_size_sqft: None,
        irrigation_type: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

pub(super) fn applied(kind: ApplicationType, on: NaiveDate) -> Application {
    Application {
        id: None,
        lawn_profile_id: 1,
        application_type: kind,
        product_name: None,
        application_date: on,
        rate_per_1000sqft: None,
        coverage_sqft: None,
        notes: None,
        weather_snapshot: None,
        nitrogen_pct: None,
        phosphorus_pct: None,
        potassium_pct: None,
        plant_id: None,
        follow_up_date: None,
        frac_classes: None,
        product_id: None,
        created_at: chrono::Utc::now(),
    }
}

pub(super) fn fed(on: NaiveDate) -> Application {
    applied(ApplicationType::Fertilizer, on)
}

pub(super) fn rec(id: &str) -> Recommendation {
    Recommendation::new(
        id,
        RecommendationCategory::Fertilizer,
        Severity::Advisory,
        "t",
        "d",
    )
}

pub(super) fn feeding(id: &str) -> Recommendation {
    rec(id).with_feeding(0.5)
}

pub(super) fn past_target(id: &str) -> Recommendation {
    let mut r = feeding(id);
    r.feeding.as_mut().unwrap().target_reached = true;
    r
}

pub(super) struct Case {
    today: NaiveDate,
    env: EnvironmentalSummary,
    history: Vec<Application>,
    feed: Vec<Recommendation>,
}

impl Case {
    pub(super) fn on(today: NaiveDate) -> Self {
        Case {
            today,
            env: mild(),
            history: vec![],
            feed: vec![],
        }
    }
    pub(super) fn env(mut self, env: EnvironmentalSummary) -> Self {
        self.env = env;
        self
    }
    pub(super) fn log(mut self, app: Application) -> Self {
        self.history.push(app);
        self
    }
    pub(super) fn feed(mut self, rec: Recommendation) -> Self {
        self.feed.push(rec);
        self
    }
    pub(super) fn state(&self, activity: &str) -> SeasonState {
        let profile = profile();
        let states = season_states(&SeasonInputs {
            today: self.today,
            env: &self.env,
            profile: &profile,
            history: &self.history,
            feed: &self.feed,
        });
        states
            .iter()
            .find(|(id, _)| *id == activity)
            .map(|(_, s)| *s)
            .unwrap_or_else(|| panic!("no state for {activity}"))
    }
}
