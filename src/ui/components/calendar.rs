use crate::models::{Application, ApplicationType};
use crate::ui::Theme;
use chrono::{Datelike, Local, NaiveDate};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

pub struct CalendarWidget<'a> {
    year: i32,
    month: u32,
    applications: &'a [Application],
    selected_date: Option<NaiveDate>,
}

impl<'a> CalendarWidget<'a> {
    pub fn new(year: i32, month: u32, applications: &'a [Application]) -> Self {
        Self {
            year,
            month,
            applications,
            selected_date: None,
        }
    }

    pub fn selected(mut self, date: Option<NaiveDate>) -> Self {
        self.selected_date = date;
        self
    }

    fn get_applications_for_date(&self, date: NaiveDate) -> Vec<&Application> {
        self.applications
            .iter()
            .filter(|a| a.application_date == date)
            .collect()
    }

    fn days_in_month(&self) -> u32 {
        let next_month = if self.month == 12 {
            NaiveDate::from_ymd_opt(self.year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(self.year, self.month + 1, 1)
        };

        next_month
            .and_then(|d| d.pred_opt())
            .map(|d| d.day())
            .unwrap_or(30)
    }

    fn first_day_of_month(&self) -> u32 {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .map(|d| d.weekday().num_days_from_sunday())
            .unwrap_or(0)
    }
}

impl Widget for CalendarWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let month_name = match self.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };

        let title = format!("{} {}", month_name, self.year);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Theme::border());

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 21 || inner.height < 8 {
            return;
        }

        // Render day headers
        let headers = "Su Mo Tu We Th Fr Sa";
        let header_line = Line::from(Span::styled(headers, Theme::dim()));
        buf.set_line(inner.x, inner.y, &header_line, inner.width);

        let today = Local::now().date_naive();
        let days_in_month = self.days_in_month();
        let first_day = self.first_day_of_month();

        let mut day = 1u32;
        let mut row = 1u16;

        while day <= days_in_month {
            let mut line_spans: Vec<Span> = Vec::new();

            for col in 0..7 {
                if row == 1 && col < first_day {
                    line_spans.push(Span::raw("   "));
                } else if day <= days_in_month {
                    let date = NaiveDate::from_ymd_opt(self.year, self.month, day);
                    let apps = date
                        .map(|d| self.get_applications_for_date(d))
                        .unwrap_or_default();

                    let is_today = date.map(|d| d == today).unwrap_or(false);
                    let is_selected = self.selected_date == date;

                    let day_str = format!("{:2}", day);

                    let style = if is_selected {
                        Theme::selected()
                    } else if is_today {
                        Theme::highlight()
                    } else if !apps.is_empty() {
                        // Color by first application type
                        Style::default().fg(apps[0].application_type.color())
                    } else {
                        Theme::normal()
                    };

                    line_spans.push(Span::styled(day_str, style));
                    line_spans.push(Span::raw(" "));

                    day += 1;
                } else {
                    line_spans.push(Span::raw("   "));
                }
            }

            let line = Line::from(line_spans);
            if inner.y + row < inner.y + inner.height {
                buf.set_line(inner.x, inner.y + row, &line, inner.width);
            }

            row += 1;
            if day > days_in_month {
                break;
            }
        }
    }
}

pub struct ApplicationLegend;

impl Widget for ApplicationLegend {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let types = [
            ApplicationType::PreEmergent,
            ApplicationType::Fertilizer,
            ApplicationType::Fungicide,
            ApplicationType::GrubControl,
            ApplicationType::Overseed,
        ];

        let mut y = area.y;
        for app_type in types {
            if y >= area.y + area.height {
                break;
            }

            let line = Line::from(vec![
                Span::styled("■ ", Style::default().fg(app_type.color())),
                Span::styled(app_type.as_str(), Theme::dim()),
            ]);

            buf.set_line(area.x, y, &line, area.width);
            y += 1;
        }
    }
}
