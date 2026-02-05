use crate::config::Config;
use crate::db::Database;
use crate::error::Result;
use crate::logic::RulesEngine;
use crate::models::{
    Application, ApplicationType, EnvironmentalReading, EnvironmentalSummary, LawnProfile,
    Recommendation,
};
use crate::ui::screens::SettingsField;
use chrono::{Datelike, Local, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Calendar,
    Applications,
    Environmental,
    Recommendations,
    Settings,
}

impl Screen {
    pub fn from_key(c: char) -> Option<Self> {
        match c {
            '1' => Some(Screen::Dashboard),
            '2' => Some(Screen::Calendar),
            '3' => Some(Screen::Applications),
            '4' => Some(Screen::Environmental),
            '5' => Some(Screen::Recommendations),
            's' | 'S' => Some(Screen::Settings),
            _ => None,
        }
    }
}

pub struct DashboardState {
    // Dashboard is mostly read-only
}

impl DashboardState {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct CalendarState {
    pub year: i32,
    pub month: u32,
    pub selected_date: Option<NaiveDate>,
}

impl CalendarState {
    pub fn new() -> Self {
        let now = Local::now();
        Self {
            year: now.year(),
            month: now.month(),
            selected_date: Some(now.date_naive()),
        }
    }

    pub fn prev_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
    }

    pub fn next_month(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
    }
}

pub struct ApplicationsState {
    pub selected_index: usize,
    pub filter_type: Option<ApplicationType>,
}

impl ApplicationsState {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            filter_type: None,
        }
    }

    pub fn next(&mut self, max: usize) {
        if max > 0 && self.selected_index < max - 1 {
            self.selected_index += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn cycle_filter(&mut self) {
        self.filter_type = match self.filter_type {
            None => Some(ApplicationType::PreEmergent),
            Some(ApplicationType::PreEmergent) => Some(ApplicationType::Fertilizer),
            Some(ApplicationType::Fertilizer) => Some(ApplicationType::Fungicide),
            Some(ApplicationType::Fungicide) => Some(ApplicationType::GrubControl),
            Some(ApplicationType::GrubControl) => Some(ApplicationType::Overseed),
            Some(ApplicationType::Overseed) => None,
            Some(_) => None,
        };
        self.selected_index = 0;
    }
}

pub struct RecommendationsState {
    pub selected_index: usize,
}

impl RecommendationsState {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn next(&mut self, max: usize) {
        if max > 0 && self.selected_index < max - 1 {
            self.selected_index += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
}

pub struct SettingsState {
    pub focused_field: SettingsField,
    pub editing: bool,
    pub edit_buffer: String,
    pub profile_modified: bool,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            focused_field: SettingsField::Name,
            editing: false,
            edit_buffer: String::new(),
            profile_modified: false,
        }
    }

    pub fn next_field(&mut self) {
        self.focused_field = self.focused_field.next();
    }

    pub fn prev_field(&mut self) {
        self.focused_field = self.focused_field.prev();
    }

    pub fn start_editing(&mut self, current_value: &str) {
        self.editing = true;
        self.edit_buffer = current_value.to_string();
    }

    pub fn cancel_editing(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn finish_editing(&mut self) -> String {
        self.editing = false;
        self.profile_modified = true;
        std::mem::take(&mut self.edit_buffer)
    }
}

pub struct App {
    pub screen: Screen,
    pub should_quit: bool,
    pub config: Config,
    pub db: Database,

    // Data
    pub lawn_profile: Option<LawnProfile>,
    pub applications: Vec<Application>,
    pub env_summary: EnvironmentalSummary,
    pub env_history: Vec<EnvironmentalReading>,
    pub recommendations: Vec<Recommendation>,

    // Screen states
    pub dashboard_state: DashboardState,
    pub calendar_state: CalendarState,
    pub applications_state: ApplicationsState,
    pub recommendations_state: RecommendationsState,
    pub settings_state: SettingsState,

    // Services
    pub rules_engine: RulesEngine,

    // UI state
    pub status_message: Option<String>,
    pub refreshing: bool,
    pub needs_refresh: bool,
}

impl App {
    pub fn new(config: Config, db: Database) -> Result<Self> {
        // Load lawn profile
        let lawn_profile = db.get_default_lawn_profile()?;

        // Load applications
        let applications = match &lawn_profile {
            Some(p) => db.get_applications_for_profile(p.id.unwrap())?,
            None => Vec::new(),
        };

        // Load cached environmental data
        let env_history = db.get_cached_readings(168)?; // 7 days

        Ok(Self {
            screen: Screen::Dashboard,
            should_quit: false,
            config,
            db,
            lawn_profile,
            applications,
            env_summary: EnvironmentalSummary::default(),
            env_history,
            recommendations: Vec::new(),
            dashboard_state: DashboardState::new(),
            calendar_state: CalendarState::new(),
            applications_state: ApplicationsState::new(),
            recommendations_state: RecommendationsState::new(),
            settings_state: SettingsState::new(),
            rules_engine: RulesEngine::new(),
            status_message: None,
            refreshing: false,
            needs_refresh: false,
        })
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn set_status(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn request_refresh(&mut self) {
        self.needs_refresh = true;
        self.set_status("Refreshing data...");
    }

    pub fn update_environmental(&mut self, summary: EnvironmentalSummary) {
        self.env_summary = summary;
        self.evaluate_rules();
    }

    pub fn evaluate_rules(&mut self) {
        if let Some(ref profile) = self.lawn_profile {
            self.recommendations =
                self.rules_engine
                    .evaluate(&self.env_summary, profile, &self.applications);
        }
    }

    pub fn reload_applications(&mut self) -> Result<()> {
        if let Some(ref profile) = self.lawn_profile {
            self.applications = self.db.get_applications_for_profile(profile.id.unwrap())?;
        }
        Ok(())
    }

    pub fn add_application(&mut self, app: Application) -> Result<i64> {
        let id = self.db.create_application(&app)?;
        self.reload_applications()?;
        self.evaluate_rules();
        Ok(id)
    }

    pub fn delete_application(&mut self, id: i64) -> Result<()> {
        self.db.delete_application(id)?;
        self.reload_applications()?;
        self.evaluate_rules();
        Ok(())
    }

    pub fn save_lawn_profile(&mut self, profile: LawnProfile) -> Result<()> {
        if profile.id.is_some() {
            self.db.update_lawn_profile(&profile)?;
        } else {
            let id = self.db.create_lawn_profile(&profile)?;
            let mut p = profile;
            p.id = Some(id);
            self.lawn_profile = Some(p);
            return Ok(());
        }
        self.lawn_profile = Some(profile);
        self.evaluate_rules();
        Ok(())
    }

    pub fn create_default_profile(&mut self) -> Result<()> {
        let profile = self.profile_from_config();
        let id = self.db.create_lawn_profile(&profile)?;
        let mut p = profile;
        p.id = Some(id);
        self.lawn_profile = Some(p);
        Ok(())
    }

    fn profile_from_config(&self) -> LawnProfile {
        use crate::models::{GrassType, IrrigationType, SoilType};

        let cfg = &self.config.lawn;
        let now = chrono::Utc::now();

        LawnProfile {
            id: None,
            name: cfg.name.clone(),
            grass_type: GrassType::from_str(&cfg.grass_type).unwrap_or(GrassType::TallFescue),
            usda_zone: cfg.usda_zone.clone(),
            soil_type: cfg.soil_type.as_ref().and_then(|s| SoilType::from_str(s)),
            lawn_size_sqft: cfg.lawn_size_sqft,
            irrigation_type: cfg
                .irrigation_type
                .as_ref()
                .and_then(|i| IrrigationType::from_str(i)),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn recent_applications(&self, count: usize) -> Vec<&Application> {
        self.applications.iter().take(count).collect()
    }

    pub fn active_recommendations(&self) -> Vec<&Recommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.is_active())
            .collect()
    }
}
