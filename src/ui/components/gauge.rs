use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct GaugeWidget<'a> {
    title: &'a str,
    value: Option<f64>,
    unit: &'a str,
    min: f64,
    max: f64,
    thresholds: Vec<(f64, Color)>,
    precision: usize,
}

impl<'a> GaugeWidget<'a> {
    pub fn new(title: &'a str, value: Option<f64>, unit: &'a str) -> Self {
        Self {
            title,
            value,
            unit,
            min: 0.0,
            max: 100.0,
            thresholds: Vec::new(),
            precision: 1,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn thresholds(mut self, thresholds: Vec<(f64, Color)>) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    fn get_color(&self, value: f64) -> Color {
        for (threshold, color) in self.thresholds.iter().rev() {
            if value >= *threshold {
                return *color;
            }
        }
        Theme::FG
    }
}

impl Widget for GaugeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        match self.value {
            Some(value) => {
                let color = self.get_color(value);
                let value_str = format!("{:.prec$}{}", value, self.unit, prec = self.precision);

                // Render value
                let value_line =
                    Line::from(vec![Span::styled(value_str, Style::default().fg(color))]);

                let para = Paragraph::new(value_line);
                para.render(inner, buf);

                // Render bar if space allows
                if inner.height >= 2 {
                    let bar_area = Rect {
                        x: inner.x,
                        y: inner.y + 1,
                        width: inner.width,
                        height: 1,
                    };

                    let ratio = ((value - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
                    let filled = (bar_area.width as f64 * ratio) as u16;

                    for x in bar_area.x..bar_area.x + bar_area.width {
                        let ch = if x < bar_area.x + filled {
                            '█'
                        } else {
                            '░'
                        };
                        buf[(x, bar_area.y)].set_char(ch).set_fg(color);
                    }
                }
            }
            None => {
                let na_line = Line::from(vec![Span::styled("N/A", Theme::dim())]);
                let para = Paragraph::new(na_line);
                para.render(inner, buf);
            }
        }
    }
}

pub fn temperature_gauge(title: &str, value: Option<f64>) -> GaugeWidget<'_> {
    GaugeWidget::new(title, value, "°F")
        .range(0.0, 120.0)
        .thresholds(vec![
            (0.0, Theme::TEMP_COLD),
            (40.0, Theme::TEMP_COOL),
            (60.0, Theme::TEMP_WARM),
            (85.0, Theme::TEMP_HOT),
        ])
}

pub fn moisture_gauge(title: &str, value: Option<f64>) -> GaugeWidget<'_> {
    GaugeWidget::new(title, value, "")
        .range(0.0, 0.5)
        .precision(2)
        .thresholds(vec![
            (0.0, Theme::MOISTURE_DRY),
            (0.10, Theme::MOISTURE_OK),
            (0.40, Theme::MOISTURE_WET),
        ])
}

pub fn humidity_gauge(title: &str, value: Option<f64>) -> GaugeWidget<'_> {
    GaugeWidget::new(title, value, "%")
        .range(0.0, 100.0)
        .precision(0)
        .thresholds(vec![
            (0.0, Theme::SUCCESS),
            (80.0, Theme::WARNING),
            (90.0, Theme::ERROR),
        ])
}
