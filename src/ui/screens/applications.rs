use crate::models::{Application, ApplicationType};
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Widget},
};

pub struct ApplicationsScreen<'a> {
    pub applications: &'a [Application],
    pub selected_index: usize,
    pub filter_type: Option<ApplicationType>,
}

impl<'a> ApplicationsScreen<'a> {
    pub fn new(applications: &'a [Application]) -> Self {
        Self {
            applications,
            selected_index: 0,
            filter_type: None,
        }
    }

    pub fn with_selection(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    pub fn with_filter(mut self, filter: Option<ApplicationType>) -> Self {
        self.filter_type = filter;
        self
    }

    fn filtered_apps(&self) -> Vec<&Application> {
        match self.filter_type {
            Some(t) => self
                .applications
                .iter()
                .filter(|a| a.application_type == t)
                .collect(),
            None => self.applications.iter().collect(),
        }
    }
}

impl Widget for ApplicationsScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header + filter
                Constraint::Min(10),   // Table
                Constraint::Length(1), // Nav
            ])
            .split(area);

        // Header
        self.render_header(chunks[0], buf);

        // Table
        self.render_table(chunks[1], buf);

        // Navigation
        let nav = Line::from(vec![
            Span::styled("[a]", Theme::nav_key()),
            Span::styled("Add ", Theme::nav_label()),
            Span::styled("[e]", Theme::nav_key()),
            Span::styled("Edit ", Theme::nav_label()),
            Span::styled("[d]", Theme::nav_key()),
            Span::styled("Delete ", Theme::nav_label()),
            Span::styled("[f]", Theme::nav_key()),
            Span::styled("Filter ", Theme::nav_label()),
            Span::styled("[↑↓]", Theme::nav_key()),
            Span::styled("Navigate ", Theme::nav_label()),
            Span::styled("[Esc]", Theme::nav_key()),
            Span::styled("Back", Theme::nav_label()),
        ]);
        Paragraph::new(nav).render(chunks[2], buf);
    }
}

impl ApplicationsScreen<'_> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let filter_str = match self.filter_type {
            Some(t) => format!("Filter: {}", t.as_str()),
            None => "All Applications".to_string(),
        };

        let count = self.filtered_apps().len();

        let block = Block::default()
            .title(Span::styled("Applications Log", Theme::title()))
            .borders(Borders::BOTTOM)
            .border_style(Theme::border());

        let info = Line::from(vec![
            Span::styled(filter_str, Theme::dim()),
            Span::styled(format!(" ({} records)", count), Theme::dim()),
        ]);

        let para = Paragraph::new(info).block(block);
        para.render(area, buf);
    }

    fn render_table(&self, area: Rect, buf: &mut Buffer) {
        let apps = self.filtered_apps();

        let header_cells = ["Date", "Type", "Product", "Rate", "Notes"]
            .iter()
            .map(|h| Cell::from(*h).style(Theme::header()));

        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = apps
            .iter()
            .enumerate()
            .map(|(i, app)| {
                let style = if i == self.selected_index {
                    Theme::selected()
                } else {
                    Theme::normal()
                };

                let type_style = Style::default().fg(app.application_type.color());

                let cells = vec![
                    Cell::from(app.application_date.format("%Y-%m-%d").to_string()),
                    Cell::from(app.application_type.as_str()).style(type_style),
                    Cell::from(app.product_name.as_deref().unwrap_or("-")),
                    Cell::from(
                        app.rate_per_1000sqft
                            .map(|r| format!("{:.2}", r))
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                    Cell::from(
                        app.notes
                            .as_ref()
                            .map(|n| truncate(n, 30))
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                ];

                Row::new(cells).style(style)
            })
            .collect();

        let widths = [
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Theme::border()),
            )
            .highlight_style(Theme::selected());

        // Create TableState for highlighting
        let mut state = TableState::default();
        state.select(Some(self.selected_index));

        // Render with state
        ratatui::widgets::StatefulWidget::render(table, area, buf, &mut state);
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
