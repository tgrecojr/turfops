use crate::models::{EnvironmentalSummary, Trend};
use crate::ui::components::{humidity_gauge, moisture_gauge, temperature_gauge};
use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget},
};

pub struct EnvironmentalScreen<'a> {
    pub summary: &'a EnvironmentalSummary,
}

impl<'a> EnvironmentalScreen<'a> {
    pub fn new(summary: &'a EnvironmentalSummary) -> Self {
        Self { summary }
    }
}

impl Widget for EnvironmentalScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(5), // Current conditions gauges
                Constraint::Length(8), // Soil temps table
                Constraint::Min(6),    // 7-day summary
                Constraint::Length(1), // Nav
            ])
            .split(area);

        // Title
        let last_updated = self
            .summary
            .last_updated
            .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "Never".to_string());

        let title = Line::from(vec![
            Span::styled("Environmental Data", Theme::title()),
            Span::styled(" - Last updated: ", Theme::dim()),
            Span::styled(last_updated, Theme::normal()),
        ]);
        Paragraph::new(title).render(chunks[0], buf);

        // Current conditions gauges
        self.render_current_gauges(chunks[1], buf);

        // Soil temps at depths
        self.render_soil_depths(chunks[2], buf);

        // 7-day summary
        self.render_summary(chunks[3], buf);

        // Navigation
        let nav = Line::from(vec![
            Span::styled("[r]", Theme::nav_key()),
            Span::styled("Refresh ", Theme::nav_label()),
            Span::styled("[1-5]", Theme::nav_key()),
            Span::styled("Screens ", Theme::nav_label()),
            Span::styled("[Esc]", Theme::nav_key()),
            Span::styled("Back", Theme::nav_label()),
        ]);
        Paragraph::new(nav).render(chunks[4], buf);
    }
}

impl EnvironmentalScreen<'_> {
    fn render_current_gauges(&self, area: Rect, buf: &mut Buffer) {
        let gauge_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);

        let current = self.summary.current.as_ref();

        let ambient = current.and_then(|c| c.ambient_temp_f);
        temperature_gauge("Ambient Temp", ambient).render(gauge_chunks[0], buf);

        let soil = current.and_then(|c| c.soil_temp_10_f);
        temperature_gauge("Soil Temp (10cm)", soil).render(gauge_chunks[1], buf);

        let humidity = current.and_then(|c| c.humidity_percent);
        humidity_gauge("Humidity", humidity).render(gauge_chunks[2], buf);

        let moisture = current.and_then(|c| c.primary_soil_moisture());
        moisture_gauge("Soil Moisture", moisture).render(gauge_chunks[3], buf);
    }

    fn render_soil_depths(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Soil Conditions by Depth")
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let current = match self.summary.current.as_ref() {
            Some(c) => c,
            None => {
                let para = Paragraph::new(Span::styled("No data available", Theme::dim()));
                para.render(inner, buf);
                return;
            }
        };

        let header = Row::new(vec![
            Cell::from("Depth"),
            Cell::from("Temperature"),
            Cell::from("Moisture"),
        ])
        .style(Theme::header());

        let depths = [
            ("5 cm", current.soil_temp_5_f, current.soil_moisture_5),
            ("10 cm", current.soil_temp_10_f, current.soil_moisture_10),
            ("20 cm", current.soil_temp_20_f, current.soil_moisture_20),
            ("50 cm", current.soil_temp_50_f, current.soil_moisture_50),
            ("100 cm", current.soil_temp_100_f, current.soil_moisture_100),
        ];

        let rows: Vec<Row> = depths
            .iter()
            .map(|(depth, temp, moisture)| {
                let temp_str = temp
                    .map(|t| format!("{:.1}°F", t))
                    .unwrap_or_else(|| "-".to_string());
                let temp_color = temp.map(Theme::temp_color).unwrap_or(Theme::DIM);

                let moisture_str = moisture
                    .map(|m| format!("{:.3}", m))
                    .unwrap_or_else(|| "-".to_string());
                let moisture_color = moisture.map(Theme::moisture_color).unwrap_or(Theme::DIM);

                Row::new(vec![
                    Cell::from(*depth),
                    Cell::from(temp_str).style(ratatui::style::Style::default().fg(temp_color)),
                    Cell::from(moisture_str)
                        .style(ratatui::style::Style::default().fg(moisture_color)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Length(15),
        ];

        let table = Table::new(rows, widths).header(header);
        table.render(inner, buf);
    }

    fn render_summary(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("7-Day Summary")
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines = Vec::new();

        // Soil temp average
        if let Some(avg) = self.summary.soil_temp_7day_avg_f {
            let trend_str = match self.summary.soil_temp_trend {
                Trend::Rising => " ↑",
                Trend::Falling => " ↓",
                Trend::Stable => " →",
                Trend::Unknown => "",
            };
            lines.push(Line::from(vec![
                Span::styled("Avg Soil Temp (10cm): ", Theme::dim()),
                Span::styled(
                    format!("{:.1}°F{}", avg, trend_str),
                    ratatui::style::Style::default().fg(Theme::temp_color(avg)),
                ),
            ]));
        }

        // Ambient temp average
        if let Some(avg) = self.summary.ambient_temp_7day_avg_f {
            lines.push(Line::from(vec![
                Span::styled("Avg Ambient Temp: ", Theme::dim()),
                Span::styled(
                    format!("{:.1}°F", avg),
                    ratatui::style::Style::default().fg(Theme::temp_color(avg)),
                ),
            ]));
        }

        // Humidity average
        if let Some(avg) = self.summary.humidity_7day_avg {
            let color = if avg > 80.0 {
                Theme::WARNING
            } else {
                Theme::SUCCESS
            };
            lines.push(Line::from(vec![
                Span::styled("Avg Humidity: ", Theme::dim()),
                Span::styled(
                    format!("{:.0}%", avg),
                    ratatui::style::Style::default().fg(color),
                ),
            ]));
        }

        // Precipitation
        if let Some(precip) = self.summary.precipitation_7day_total_mm {
            lines.push(Line::from(vec![
                Span::styled("Total Precipitation: ", Theme::dim()),
                Span::styled(format!("{:.1} mm", precip), Theme::normal()),
            ]));
        }

        // Data sources
        lines.push(Line::from(vec![]));
        lines.push(Line::from(vec![
            Span::styled("Sources: ", Theme::dim()),
            Span::styled("NOAA USCRN (soil) • Patio Sensor (ambient)", Theme::dim()),
        ]));

        let para = Paragraph::new(lines);
        para.render(inner, buf);
    }
}
