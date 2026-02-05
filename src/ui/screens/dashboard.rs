use crate::models::{Application, EnvironmentalSummary, LawnProfile, Recommendation};
use crate::ui::components::{humidity_gauge, moisture_gauge, temperature_gauge};
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

pub struct DashboardScreen<'a> {
    pub profile: Option<&'a LawnProfile>,
    pub env_summary: &'a EnvironmentalSummary,
    pub recommendations: &'a [Recommendation],
    pub recent_apps: &'a [Application],
    pub status_message: Option<&'a str>,
}

impl<'a> DashboardScreen<'a> {
    pub fn new(
        profile: Option<&'a LawnProfile>,
        env_summary: &'a EnvironmentalSummary,
        recommendations: &'a [Recommendation],
        recent_apps: &'a [Application],
    ) -> Self {
        Self {
            profile,
            env_summary,
            recommendations,
            recent_apps,
            status_message: None,
        }
    }

    pub fn with_status(mut self, status: Option<&'a str>) -> Self {
        self.status_message = status;
        self
    }
}

impl Widget for DashboardScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Main layout: top info, gauges, alerts, recent
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(5), // Gauges row
                Constraint::Min(8),    // Alerts and recent apps
                Constraint::Length(1), // Status message
                Constraint::Length(1), // Nav bar
            ])
            .split(area);

        // Header with profile info
        self.render_header(chunks[0], buf);

        // Gauges row
        self.render_gauges(chunks[1], buf);

        // Split middle section for alerts and recent apps
        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        self.render_alerts(middle[0], buf);
        self.render_recent_apps(middle[1], buf);

        // Status message
        self.render_status_message(chunks[3], buf);

        // Nav bar
        self.render_status(chunks[4], buf);
    }
}

impl DashboardScreen<'_> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let title = match self.profile {
            Some(p) => format!(
                "TurfOps - {} ({} - Zone {})",
                p.name, p.grass_type, p.usda_zone
            ),
            None => "TurfOps - No Profile Configured".to_string(),
        };

        let block = Block::default()
            .title(Span::styled(title, Theme::title()))
            .borders(Borders::BOTTOM)
            .border_style(Theme::border());

        let last_updated = self
            .env_summary
            .last_updated
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());

        let info = format!("Last updated: {}", last_updated);
        let para = Paragraph::new(Span::styled(info, Theme::dim())).block(block);
        para.render(area, buf);
    }

    fn render_gauges(&self, area: Rect, buf: &mut Buffer) {
        let gauge_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .split(area);

        let current = self.env_summary.current.as_ref();

        // Soil temp
        let soil_temp = current.and_then(|c| c.soil_temp_10_f);
        temperature_gauge("Soil 10cm", soil_temp).render(gauge_chunks[0], buf);

        // Ambient temp
        let ambient_temp = current.and_then(|c| c.ambient_temp_f);
        temperature_gauge("Ambient", ambient_temp).render(gauge_chunks[1], buf);

        // Humidity
        let humidity = current.and_then(|c| c.humidity_percent);
        humidity_gauge("Humidity", humidity).render(gauge_chunks[2], buf);

        // Soil moisture
        let moisture = current.and_then(|c| c.primary_soil_moisture());
        moisture_gauge("Moisture", moisture).render(gauge_chunks[3], buf);

        // 7-day avg soil temp
        let avg_temp = self.env_summary.soil_temp_7day_avg_f;
        temperature_gauge("7d Avg Soil", avg_temp).render(gauge_chunks[4], buf);
    }

    fn render_alerts(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Span::styled("Active Alerts", Theme::header()))
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let active: Vec<&Recommendation> = self
            .recommendations
            .iter()
            .filter(|r| r.is_active())
            .take(3) // Fewer items since each takes 2 lines
            .collect();

        if active.is_empty() {
            let para = Paragraph::new(Span::styled("No active alerts", Theme::dim()));
            para.render(inner, buf);
            return;
        }

        let items: Vec<ListItem> = active
            .iter()
            .map(|r| {
                let severity_style = Style::default().fg(r.severity.color());
                // Title line
                let title_line = Line::from(vec![
                    Span::styled(format!("{} ", r.severity.symbol()), severity_style),
                    Span::styled(&r.title, severity_style),
                ]);
                // Description line (indented, dimmed)
                let desc_line = Line::from(vec![
                    Span::styled("  ", Theme::dim()),
                    Span::styled(&r.description, Theme::dim()),
                ]);
                ListItem::new(vec![title_line, desc_line])
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }

    fn render_recent_apps(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(Span::styled("Recent Applications", Theme::header()))
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        if self.recent_apps.is_empty() {
            let para = Paragraph::new(Span::styled("No applications recorded", Theme::dim()));
            para.render(inner, buf);
            return;
        }

        let items: Vec<ListItem> = self
            .recent_apps
            .iter()
            .take(5)
            .map(|app| {
                let type_style = Style::default().fg(app.application_type.color());
                let line = Line::from(vec![
                    Span::styled(
                        app.application_date.format("%m/%d").to_string(),
                        Theme::dim(),
                    ),
                    Span::raw(" "),
                    Span::styled(app.application_type.as_str(), type_style),
                    Span::raw(" "),
                    Span::styled(app.product_name.as_deref().unwrap_or(""), Theme::normal()),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }

    fn render_status_message(&self, area: Rect, buf: &mut Buffer) {
        if let Some(msg) = self.status_message {
            let style = if msg.contains("OFFLINE") || msg.contains("failed") {
                Theme::warning()
            } else {
                Theme::success()
            };
            let para = Paragraph::new(Span::styled(msg, style));
            para.render(area, buf);
        }
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let nav = Line::from(vec![
            Span::styled("[1]", Theme::nav_key()),
            Span::styled("Dashboard ", Theme::nav_label()),
            Span::styled("[2]", Theme::nav_key()),
            Span::styled("Calendar ", Theme::nav_label()),
            Span::styled("[3]", Theme::nav_key()),
            Span::styled("Apps ", Theme::nav_label()),
            Span::styled("[4]", Theme::nav_key()),
            Span::styled("Env ", Theme::nav_label()),
            Span::styled("[5]", Theme::nav_key()),
            Span::styled("Recs ", Theme::nav_label()),
            Span::styled("[s]", Theme::nav_key()),
            Span::styled("Settings ", Theme::nav_label()),
            Span::styled("[r]", Theme::nav_key()),
            Span::styled("Refresh ", Theme::nav_label()),
            Span::styled("[q]", Theme::nav_key()),
            Span::styled("Quit", Theme::nav_label()),
        ]);

        let para = Paragraph::new(nav);
        para.render(area, buf);
    }
}
