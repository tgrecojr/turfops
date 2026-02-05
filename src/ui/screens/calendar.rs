use crate::models::Application;
use crate::ui::components::{ApplicationLegend, CalendarWidget};
use crate::ui::Theme;
use chrono::{Datelike, Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

pub struct CalendarScreen<'a> {
    pub year: i32,
    pub month: u32,
    pub selected_date: Option<NaiveDate>,
    pub applications: &'a [Application],
}

impl<'a> CalendarScreen<'a> {
    pub fn new(applications: &'a [Application]) -> Self {
        let now = Local::now();
        Self {
            year: now.year(),
            month: now.month(),
            selected_date: Some(now.date_naive()),
            applications,
        }
    }

    pub fn with_date(mut self, year: i32, month: u32) -> Self {
        self.year = year;
        self.month = month;
        self
    }

    pub fn selected(mut self, date: Option<NaiveDate>) -> Self {
        self.selected_date = date;
        self
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

    fn apps_for_selected(&self) -> Vec<&Application> {
        match self.selected_date {
            Some(date) => self
                .applications
                .iter()
                .filter(|a| a.application_date == date)
                .collect(),
            None => Vec::new(),
        }
    }
}

impl Widget for CalendarScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(10),   // Calendar + details
                Constraint::Length(1), // Nav
            ])
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled("Calendar View", Theme::title()),
            Span::styled(" - ", Theme::dim()),
            Span::styled("[←/→] Change month  [↑/↓] Select day", Theme::dim()),
        ]);
        Paragraph::new(title).render(chunks[0], buf);

        // Main content: calendar on left, details on right
        let content = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        // Calendar and legend
        let cal_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(7)])
            .split(content[0]);

        CalendarWidget::new(self.year, self.month, self.applications)
            .selected(self.selected_date)
            .render(cal_area[0], buf);

        let legend_block = Block::default()
            .title("Legend")
            .borders(Borders::ALL)
            .border_style(Theme::border());
        let legend_inner = legend_block.inner(cal_area[1]);
        legend_block.render(cal_area[1], buf);
        ApplicationLegend.render(legend_inner, buf);

        // Selected date details
        self.render_details(content[1], buf);

        // Navigation
        let nav = Line::from(vec![
            Span::styled("[1-5]", Theme::nav_key()),
            Span::styled("Screens ", Theme::nav_label()),
            Span::styled("[a]", Theme::nav_key()),
            Span::styled("Add App ", Theme::nav_label()),
            Span::styled("[Esc]", Theme::nav_key()),
            Span::styled("Back", Theme::nav_label()),
        ]);
        Paragraph::new(nav).render(chunks[2], buf);
    }
}

impl CalendarScreen<'_> {
    fn render_details(&self, area: Rect, buf: &mut Buffer) {
        let date_str = self
            .selected_date
            .map(|d| d.format("%B %d, %Y").to_string())
            .unwrap_or_else(|| "No date selected".to_string());

        let block = Block::default()
            .title(date_str)
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let apps = self.apps_for_selected();

        if apps.is_empty() {
            let para = Paragraph::new(Span::styled("No applications on this date", Theme::dim()));
            para.render(inner, buf);
            return;
        }

        let items: Vec<ListItem> = apps
            .iter()
            .map(|app| {
                let mut lines = vec![Line::from(vec![
                    Span::styled(
                        "● ",
                        ratatui::style::Style::default().fg(app.application_type.color()),
                    ),
                    Span::styled(app.application_type.as_str(), Theme::header()),
                ])];

                if let Some(ref product) = app.product_name {
                    lines.push(Line::from(vec![
                        Span::styled("  Product: ", Theme::dim()),
                        Span::styled(product, Theme::normal()),
                    ]));
                }

                if let Some(rate) = app.rate_per_1000sqft {
                    lines.push(Line::from(vec![
                        Span::styled("  Rate: ", Theme::dim()),
                        Span::styled(format!("{:.2}/1000sqft", rate), Theme::normal()),
                    ]));
                }

                if let Some(ref notes) = app.notes {
                    lines.push(Line::from(vec![
                        Span::styled("  Notes: ", Theme::dim()),
                        Span::styled(notes, Theme::normal()),
                    ]));
                }

                if let Some(ref weather) = app.weather_snapshot {
                    let mut weather_parts = Vec::new();
                    if let Some(t) = weather.soil_temp_10cm_f {
                        weather_parts.push(format!("Soil: {:.0}°F", t));
                    }
                    if let Some(t) = weather.ambient_temp_f {
                        weather_parts.push(format!("Air: {:.0}°F", t));
                    }
                    if !weather_parts.is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled("  Weather: ", Theme::dim()),
                            Span::styled(weather_parts.join(", "), Theme::normal()),
                        ]));
                    }
                }

                ListItem::new(lines)
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }
}
