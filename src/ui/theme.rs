use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    // Base colors
    pub const FG: Color = Color::White;
    pub const DIM: Color = Color::DarkGray;
    pub const ACCENT: Color = Color::Green;
    pub const HIGHLIGHT: Color = Color::Cyan;

    // Status colors
    pub const SUCCESS: Color = Color::Green;
    pub const WARNING: Color = Color::Yellow;
    pub const ERROR: Color = Color::Red;

    // Environmental colors
    pub const TEMP_COLD: Color = Color::LightBlue;
    pub const TEMP_COOL: Color = Color::Cyan;
    pub const TEMP_WARM: Color = Color::Yellow;
    pub const TEMP_HOT: Color = Color::Red;
    pub const MOISTURE_DRY: Color = Color::Yellow;
    pub const MOISTURE_OK: Color = Color::Green;
    pub const MOISTURE_WET: Color = Color::LightBlue;

    // Styles
    pub fn title() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header() -> Style {
        Style::default().fg(Self::FG).add_modifier(Modifier::BOLD)
    }

    pub fn normal() -> Style {
        Style::default().fg(Self::FG)
    }

    pub fn dim() -> Style {
        Style::default().fg(Self::DIM)
    }

    pub fn highlight() -> Style {
        Style::default()
            .fg(Self::HIGHLIGHT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected() -> Style {
        Style::default()
            .bg(Color::DarkGray)
            .fg(Self::FG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS)
    }

    pub fn warning() -> Style {
        Style::default().fg(Self::WARNING)
    }

    pub fn temp_color(temp_f: f64) -> Color {
        if temp_f < 40.0 {
            Self::TEMP_COLD
        } else if temp_f < 60.0 {
            Self::TEMP_COOL
        } else if temp_f < 80.0 {
            Self::TEMP_WARM
        } else {
            Self::TEMP_HOT
        }
    }

    pub fn moisture_color(moisture: f64) -> Color {
        if moisture < 0.10 {
            Self::MOISTURE_DRY
        } else if moisture < 0.40 {
            Self::MOISTURE_OK
        } else {
            Self::MOISTURE_WET
        }
    }

    pub fn nav_key() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn nav_label() -> Style {
        Style::default().fg(Self::DIM)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::DIM)
    }

    pub fn border_focused() -> Style {
        Style::default().fg(Self::ACCENT)
    }
}
