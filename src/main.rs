mod app;
mod config;
mod datasources;
mod db;
mod error;
mod logic;
mod models;
mod ui;

use app::{App, Screen};
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use db::Database;
use error::Result;
use logic::DataSyncService;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use ui::screens::{
    ApplicationsScreen, CalendarScreen, DashboardScreen, EnvironmentalScreen,
    RecommendationsScreen, SettingsScreen,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    // Load configuration
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!("Please copy config/config.yaml.example to config/config.yaml");
            std::process::exit(1);
        }
    };

    // Initialize database
    let db = Database::open()?;

    // Create app
    let mut app = App::new(config.clone(), db)?;

    // Create default profile if none exists
    if app.lawn_profile.is_none() {
        app.create_default_profile()?;
        app.set_status("Created default lawn profile - update in Settings");
    }

    // Initialize data sync service
    let mut data_sync = DataSyncService::new(config, app.db.clone());

    // Try to initialize and fetch initial data
    match data_sync.initialize().await {
        Ok(()) => {
            let status = data_sync.check_connections().await;
            let mut status_parts = Vec::new();
            if status.soildata {
                status_parts.push("SoilData: OK");
            } else {
                status_parts.push("SoilData: OFFLINE");
            }
            if status.homeassistant {
                status_parts.push("HomeAssistant: OK");
            } else {
                status_parts.push("HomeAssistant: OFFLINE");
            }

            if let Ok(summary) = data_sync.refresh().await {
                app.update_environmental(summary);
            }
            app.set_status(&status_parts.join(" | "));
        }
        Err(e) => {
            tracing::warn!("Failed to initialize data sources: {}", e);
            app.set_status(&format!("Init failed: {}", e));
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the main loop
    let result = run_app(&mut terminal, &mut app, &mut data_sync).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    data_sync: &mut DataSyncService,
) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| {
            let area = f.area();

            match app.screen {
                Screen::Dashboard => {
                    let profile = app.lawn_profile.as_ref();
                    let recent = app.recent_applications(5);
                    let recent_vec: Vec<_> = recent.into_iter().cloned().collect();
                    let screen = DashboardScreen::new(
                        profile,
                        &app.env_summary,
                        &app.recommendations,
                        &recent_vec,
                    )
                    .with_status(app.status_message.as_deref());
                    f.render_widget(screen, area);
                }
                Screen::Calendar => {
                    let screen = CalendarScreen::new(&app.applications)
                        .with_date(app.calendar_state.year, app.calendar_state.month)
                        .selected(app.calendar_state.selected_date);
                    f.render_widget(screen, area);
                }
                Screen::Applications => {
                    let screen = ApplicationsScreen::new(&app.applications)
                        .with_selection(app.applications_state.selected_index)
                        .with_filter(app.applications_state.filter_type);
                    f.render_widget(screen, area);
                }
                Screen::Environmental => {
                    let screen = EnvironmentalScreen::new(&app.env_summary);
                    f.render_widget(screen, area);
                }
                Screen::Recommendations => {
                    let screen = RecommendationsScreen::new(&app.recommendations)
                        .with_selection(app.recommendations_state.selected_index);
                    f.render_widget(screen, area);
                }
                Screen::Settings => {
                    if let Some(ref profile) = app.lawn_profile {
                        let screen = SettingsScreen::new(profile)
                            .with_focus(app.settings_state.focused_field)
                            .editing(app.settings_state.editing, &app.settings_state.edit_buffer);
                        f.render_widget(screen, area);
                    }
                }
            }
        })?;

        // Handle input with timeout for async operations
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Global key handling
                match key.code {
                    KeyCode::Char('q') if !app.settings_state.editing => {
                        app.quit();
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit();
                    }
                    KeyCode::Esc if !app.settings_state.editing => {
                        // Go back to dashboard
                        app.switch_screen(Screen::Dashboard);
                    }
                    KeyCode::Char(c) if !app.settings_state.editing => {
                        if let Some(screen) = Screen::from_key(c) {
                            app.switch_screen(screen);
                        } else {
                            // Screen-specific key handling
                            handle_screen_input(app, key.code, key.modifiers);
                        }
                    }
                    _ => {
                        handle_screen_input(app, key.code, key.modifiers);
                    }
                }
            }
        }

        // Handle refresh request
        if app.needs_refresh {
            app.needs_refresh = false;
            app.refreshing = true;
            match data_sync.refresh().await {
                Ok(summary) => {
                    app.update_environmental(summary);
                    app.set_status("Data refreshed");
                }
                Err(e) => {
                    app.set_status(&format!("Refresh failed: {}", e));
                }
            }
            app.refreshing = false;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_screen_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match app.screen {
        Screen::Dashboard => handle_dashboard_input(app, code),
        Screen::Calendar => handle_calendar_input(app, code),
        Screen::Applications => handle_applications_input(app, code),
        Screen::Environmental => handle_environmental_input(app, code),
        Screen::Recommendations => handle_recommendations_input(app, code),
        Screen::Settings => handle_settings_input(app, code, modifiers),
    }
}

fn handle_dashboard_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('r') => {
            app.request_refresh();
        }
        KeyCode::Char('a') => {
            // Quick add - switch to applications screen
            app.switch_screen(Screen::Applications);
        }
        _ => {}
    }
}

fn handle_calendar_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Left => app.calendar_state.prev_month(),
        KeyCode::Right => app.calendar_state.next_month(),
        KeyCode::Char('a') => {
            app.switch_screen(Screen::Applications);
        }
        _ => {}
    }
}

fn handle_applications_input(app: &mut App, code: KeyCode) {
    let count = app.applications.len();
    match code {
        KeyCode::Up => app.applications_state.prev(),
        KeyCode::Down => app.applications_state.next(count),
        KeyCode::Char('f') => app.applications_state.cycle_filter(),
        KeyCode::Char('d') => {
            // Delete selected
            if let Some(selected_app) = app.applications.get(app.applications_state.selected_index)
            {
                if let Some(id) = selected_app.id {
                    let _ = app.delete_application(id);
                }
            }
        }
        _ => {}
    }
}

fn handle_environmental_input(app: &mut App, code: KeyCode) {
    if let KeyCode::Char('r') = code {
        app.request_refresh();
    }
}

fn handle_recommendations_input(app: &mut App, code: KeyCode) {
    let count = app.active_recommendations().len();
    match code {
        KeyCode::Up => app.recommendations_state.prev(),
        KeyCode::Down => app.recommendations_state.next(count),
        KeyCode::Enter => {
            // Mark as addressed
            if let Some(rec) = app
                .recommendations
                .get_mut(app.recommendations_state.selected_index)
            {
                rec.addressed = true;
            }
        }
        KeyCode::Char('x') => {
            // Dismiss
            if let Some(rec) = app
                .recommendations
                .get_mut(app.recommendations_state.selected_index)
            {
                rec.dismissed = true;
            }
        }
        _ => {}
    }
}

fn handle_settings_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if app.settings_state.editing {
        // Editing mode
        match code {
            KeyCode::Esc => {
                app.settings_state.cancel_editing();
            }
            KeyCode::Enter => {
                let value = app.settings_state.finish_editing();
                let field = app.settings_state.focused_field;
                // Apply the value to the profile
                if let Some(ref mut profile) = app.lawn_profile {
                    apply_field_value(profile, field, &value);
                }
                // Save the profile (separate borrow scope)
                if let Some(profile) = app.lawn_profile.clone() {
                    let _ = app.save_lawn_profile(profile);
                }
            }
            KeyCode::Backspace => {
                app.settings_state.edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.settings_state.edit_buffer.push(c);
            }
            _ => {}
        }
    } else {
        // Navigation mode
        match code {
            KeyCode::Up => app.settings_state.prev_field(),
            KeyCode::Down => app.settings_state.next_field(),
            KeyCode::Tab => app.settings_state.next_field(),
            KeyCode::Enter => {
                // Start editing
                if let Some(ref profile) = app.lawn_profile {
                    let current = get_field_value(profile, app.settings_state.focused_field);
                    app.settings_state.start_editing(&current);
                }
            }
            KeyCode::Char('s') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Save profile
                if let Some(ref profile) = app.lawn_profile {
                    let _ = app.save_lawn_profile(profile.clone());
                    app.set_status("Profile saved");
                }
            }
            _ => {}
        }
    }
}

fn get_field_value(profile: &models::LawnProfile, field: ui::screens::SettingsField) -> String {
    use ui::screens::SettingsField;
    match field {
        SettingsField::Name => profile.name.clone(),
        SettingsField::GrassType => format!("{:?}", profile.grass_type),
        SettingsField::UsdaZone => profile.usda_zone.clone(),
        SettingsField::SoilType => profile
            .soil_type
            .map(|s| format!("{:?}", s))
            .unwrap_or_default(),
        SettingsField::LawnSize => profile
            .lawn_size_sqft
            .map(|s| s.to_string())
            .unwrap_or_default(),
        SettingsField::IrrigationType => profile
            .irrigation_type
            .map(|i| format!("{:?}", i))
            .unwrap_or_default(),
    }
}

fn apply_field_value(
    profile: &mut models::LawnProfile,
    field: ui::screens::SettingsField,
    value: &str,
) {
    use models::{GrassType, IrrigationType, SoilType};
    use ui::screens::SettingsField;

    match field {
        SettingsField::Name => {
            if !value.is_empty() {
                profile.name = value.to_string();
            }
        }
        SettingsField::GrassType => {
            if let Some(gt) = GrassType::from_str(value) {
                profile.grass_type = gt;
            }
        }
        SettingsField::UsdaZone => {
            if !value.is_empty() {
                profile.usda_zone = value.to_string();
            }
        }
        SettingsField::SoilType => {
            profile.soil_type = SoilType::from_str(value);
        }
        SettingsField::LawnSize => {
            profile.lawn_size_sqft = value.parse().ok();
        }
        SettingsField::IrrigationType => {
            profile.irrigation_type = IrrigationType::from_str(value);
        }
    }
}
