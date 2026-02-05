use crate::models::{GrassType, IrrigationType, LawnProfile, SoilType};
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Name,
    GrassType,
    UsdaZone,
    SoilType,
    LawnSize,
    IrrigationType,
}

impl SettingsField {
    pub fn all() -> &'static [SettingsField] {
        &[
            SettingsField::Name,
            SettingsField::GrassType,
            SettingsField::UsdaZone,
            SettingsField::SoilType,
            SettingsField::LawnSize,
            SettingsField::IrrigationType,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingsField::Name => "Lawn Name",
            SettingsField::GrassType => "Grass Type",
            SettingsField::UsdaZone => "USDA Zone",
            SettingsField::SoilType => "Soil Type",
            SettingsField::LawnSize => "Lawn Size (sqft)",
            SettingsField::IrrigationType => "Irrigation",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SettingsField::Name => SettingsField::GrassType,
            SettingsField::GrassType => SettingsField::UsdaZone,
            SettingsField::UsdaZone => SettingsField::SoilType,
            SettingsField::SoilType => SettingsField::LawnSize,
            SettingsField::LawnSize => SettingsField::IrrigationType,
            SettingsField::IrrigationType => SettingsField::Name,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SettingsField::Name => SettingsField::IrrigationType,
            SettingsField::GrassType => SettingsField::Name,
            SettingsField::UsdaZone => SettingsField::GrassType,
            SettingsField::SoilType => SettingsField::UsdaZone,
            SettingsField::LawnSize => SettingsField::SoilType,
            SettingsField::IrrigationType => SettingsField::LawnSize,
        }
    }
}

pub struct SettingsScreen<'a> {
    pub profile: &'a LawnProfile,
    pub focused_field: SettingsField,
    pub editing: bool,
    pub edit_buffer: String,
}

impl<'a> SettingsScreen<'a> {
    pub fn new(profile: &'a LawnProfile) -> Self {
        Self {
            profile,
            focused_field: SettingsField::Name,
            editing: false,
            edit_buffer: String::new(),
        }
    }

    pub fn with_focus(mut self, field: SettingsField) -> Self {
        self.focused_field = field;
        self
    }

    pub fn editing(mut self, editing: bool, buffer: &str) -> Self {
        self.editing = editing;
        self.edit_buffer = buffer.to_string();
        self
    }

    fn get_field_value(&self, field: SettingsField) -> String {
        match field {
            SettingsField::Name => self.profile.name.clone(),
            SettingsField::GrassType => self.profile.grass_type.as_str().to_string(),
            SettingsField::UsdaZone => self.profile.usda_zone.clone(),
            SettingsField::SoilType => self
                .profile
                .soil_type
                .map(|s| s.as_str().to_string())
                .unwrap_or_else(|| "Not set".to_string()),
            SettingsField::LawnSize => self
                .profile
                .lawn_size_sqft
                .map(|s| format!("{:.0}", s))
                .unwrap_or_else(|| "Not set".to_string()),
            SettingsField::IrrigationType => self
                .profile
                .irrigation_type
                .map(|i| i.as_str().to_string())
                .unwrap_or_else(|| "Not set".to_string()),
        }
    }
}

impl Widget for SettingsScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(20),   // Form (6 fields * 3 lines + borders)
                Constraint::Length(5), // Help
                Constraint::Length(1), // Nav
            ])
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled("Settings", Theme::title()),
            Span::styled(" - Lawn Profile", Theme::dim()),
        ]);
        Paragraph::new(title).render(chunks[0], buf);

        // Form
        self.render_form(chunks[1], buf);

        // Help
        self.render_help(chunks[2], buf);

        // Navigation
        let nav = Line::from(vec![
            Span::styled("[↑↓]", Theme::nav_key()),
            Span::styled("Navigate ", Theme::nav_label()),
            Span::styled("[Enter]", Theme::nav_key()),
            Span::styled("Edit ", Theme::nav_label()),
            Span::styled("[Esc]", Theme::nav_key()),
            Span::styled("Cancel/Back ", Theme::nav_label()),
            Span::styled("[Ctrl+S]", Theme::nav_key()),
            Span::styled("Save", Theme::nav_label()),
        ]);
        Paragraph::new(nav).render(chunks[3], buf);
    }
}

impl SettingsScreen<'_> {
    fn render_form(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Lawn Profile")
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let field_height = 3;
        let constraints: Vec<Constraint> = SettingsField::all()
            .iter()
            .map(|_| Constraint::Length(field_height))
            .collect();

        let field_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, field) in SettingsField::all().iter().enumerate() {
            let is_focused = *field == self.focused_field;

            let value = if is_focused && self.editing {
                format!("{}_", self.edit_buffer)
            } else {
                self.get_field_value(*field)
            };

            let border_style = if is_focused {
                Theme::border_focused()
            } else {
                Theme::border()
            };

            let value_style = if is_focused && self.editing {
                Theme::highlight()
            } else if is_focused {
                Theme::selected()
            } else {
                Theme::normal()
            };

            let field_block = Block::default()
                .title(field.label())
                .borders(Borders::ALL)
                .border_style(border_style);

            let field_inner = field_block.inner(field_areas[i]);
            field_block.render(field_areas[i], buf);

            let para = Paragraph::new(Span::styled(value, value_style));
            para.render(field_inner, buf);
        }
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Field Options")
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let help_text = match self.focused_field {
            SettingsField::Name => "Enter a name for this lawn profile",
            SettingsField::GrassType => {
                "Options: Kentucky Bluegrass, Tall Fescue, Perennial Ryegrass, Fine Fescue, Bermuda, Zoysia, St. Augustine, Mixed"
            }
            SettingsField::UsdaZone => "Enter USDA hardiness zone (e.g., 7a, 6b, 8a)",
            SettingsField::SoilType => "Options: Clay, Loam, Sandy, Silt Loam, Clay Loam, Sandy Loam",
            SettingsField::LawnSize => "Enter lawn size in square feet",
            SettingsField::IrrigationType => "Options: In-Ground, Hose, None",
        };

        let para = Paragraph::new(Span::styled(help_text, Theme::dim()));
        para.render(inner, buf);
    }
}

// Helper types for grass type selection
pub const GRASS_TYPE_OPTIONS: &[GrassType] = &[
    GrassType::KentuckyBluegrass,
    GrassType::TallFescue,
    GrassType::PerennialRyegrass,
    GrassType::FineFescue,
    GrassType::Bermuda,
    GrassType::Zoysia,
    GrassType::StAugustine,
    GrassType::Mixed,
];

pub const SOIL_TYPE_OPTIONS: &[SoilType] = &[
    SoilType::Clay,
    SoilType::Loam,
    SoilType::Sandy,
    SoilType::SiltLoam,
    SoilType::ClayLoam,
    SoilType::SandyLoam,
];

pub const IRRIGATION_OPTIONS: &[IrrigationType] = &[
    IrrigationType::InGround,
    IrrigationType::Hose,
    IrrigationType::None,
];
