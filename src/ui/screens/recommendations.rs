use crate::models::Recommendation;
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap},
};

pub struct RecommendationsScreen<'a> {
    pub recommendations: &'a [Recommendation],
    pub selected_index: usize,
}

impl<'a> RecommendationsScreen<'a> {
    pub fn new(recommendations: &'a [Recommendation]) -> Self {
        Self {
            recommendations,
            selected_index: 0,
        }
    }

    pub fn with_selection(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    fn active_recommendations(&self) -> Vec<&Recommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.is_active())
            .collect()
    }
}

impl Widget for RecommendationsScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Min(10),   // Content
                Constraint::Length(1), // Nav
            ])
            .split(area);

        // Title
        let active_count = self.active_recommendations().len();
        let title = Line::from(vec![
            Span::styled("Recommendations", Theme::title()),
            Span::styled(format!(" ({} active)", active_count), Theme::dim()),
        ]);
        Paragraph::new(title).render(chunks[0], buf);

        // Content: list on left, details on right
        let content = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        self.render_list(content[0], buf);
        self.render_details(content[1], buf);

        // Navigation
        let nav = Line::from(vec![
            Span::styled("[↑↓]", Theme::nav_key()),
            Span::styled("Navigate ", Theme::nav_label()),
            Span::styled("[Enter]", Theme::nav_key()),
            Span::styled("Mark Addressed ", Theme::nav_label()),
            Span::styled("[x]", Theme::nav_key()),
            Span::styled("Dismiss ", Theme::nav_label()),
            Span::styled("[Esc]", Theme::nav_key()),
            Span::styled("Back", Theme::nav_label()),
        ]);
        Paragraph::new(nav).render(chunks[2], buf);
    }
}

impl RecommendationsScreen<'_> {
    fn render_list(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Active Alerts")
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let active = self.active_recommendations();

        if active.is_empty() {
            let para = Paragraph::new(Span::styled("No active recommendations", Theme::dim()));
            para.render(inner, buf);
            return;
        }

        let items: Vec<ListItem> = active
            .iter()
            .enumerate()
            .map(|(i, rec)| {
                let style = if i == self.selected_index {
                    Theme::selected()
                } else {
                    Style::default()
                };

                let severity_style = Style::default().fg(rec.severity.color());
                let category_style = Style::default().fg(rec.category.color());

                let line = Line::from(vec![
                    Span::styled(format!("{} ", rec.severity.symbol()), severity_style),
                    Span::styled(rec.category.as_str(), category_style),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items);
        list.render(inner, buf);
    }

    #[allow(clippy::vec_init_then_push)]
    fn render_details(&self, area: Rect, buf: &mut Buffer) {
        let active = self.active_recommendations();

        let selected = active.get(self.selected_index);

        let block = Block::default()
            .title("Details")
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let rec = match selected {
            Some(r) => r,
            None => {
                let para = Paragraph::new(Span::styled(
                    "Select a recommendation to view details",
                    Theme::dim(),
                ));
                para.render(inner, buf);
                return;
            }
        };

        let mut lines = Vec::new();

        // Title
        lines.push(Line::from(vec![Span::styled(&rec.title, Theme::header())]));
        lines.push(Line::from(vec![]));

        // Severity and category
        lines.push(Line::from(vec![
            Span::styled("Severity: ", Theme::dim()),
            Span::styled(
                rec.severity.as_str(),
                Style::default().fg(rec.severity.color()),
            ),
            Span::styled("  Category: ", Theme::dim()),
            Span::styled(
                rec.category.as_str(),
                Style::default().fg(rec.category.color()),
            ),
        ]));
        lines.push(Line::from(vec![]));

        // Description
        lines.push(Line::from(vec![Span::styled("Description:", Theme::dim())]));
        lines.push(Line::from(vec![Span::styled(
            &rec.description,
            Theme::normal(),
        )]));
        lines.push(Line::from(vec![]));

        // Data points
        if !rec.data_points.is_empty() {
            lines.push(Line::from(vec![Span::styled("Data Points:", Theme::dim())]));
            for dp in &rec.data_points {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", dp.label), Theme::dim()),
                    Span::styled(&dp.value, Theme::highlight()),
                    Span::styled(format!(" ({})", dp.source), Theme::dim()),
                ]));
            }
            lines.push(Line::from(vec![]));
        }

        // Explanation
        if !rec.explanation.is_empty() {
            lines.push(Line::from(vec![Span::styled("Why:", Theme::dim())]));
            lines.push(Line::from(vec![Span::styled(
                &rec.explanation,
                Theme::normal(),
            )]));
            lines.push(Line::from(vec![]));
        }

        // Suggested action
        if let Some(ref action) = rec.suggested_action {
            lines.push(Line::from(vec![Span::styled(
                "Suggested Action:",
                Theme::dim(),
            )]));
            lines.push(Line::from(vec![Span::styled(action, Theme::success())]));
        }

        let para = Paragraph::new(lines).wrap(Wrap { trim: true });
        para.render(inner, buf);
    }
}
