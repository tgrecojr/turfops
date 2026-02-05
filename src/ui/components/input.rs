use crate::ui::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct InputWidget<'a> {
    label: &'a str,
    value: &'a str,
    focused: bool,
    cursor_position: usize,
}

impl<'a> InputWidget<'a> {
    pub fn new(label: &'a str, value: &'a str) -> Self {
        Self {
            label,
            value,
            focused: false,
            cursor_position: value.len(),
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn cursor(mut self, position: usize) -> Self {
        self.cursor_position = position;
        self
    }
}

impl Widget for InputWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let block = Block::default()
            .title(self.label)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.focused {
            // Show cursor
            let display_value = if self.cursor_position < self.value.len() {
                let (before, after) = self.value.split_at(self.cursor_position);
                let cursor_char = after.chars().next().unwrap_or(' ');
                let rest = if after.len() > 1 { &after[1..] } else { "" };

                Line::from(vec![
                    Span::raw(before),
                    Span::styled(cursor_char.to_string(), Theme::selected()),
                    Span::raw(rest),
                ])
            } else {
                Line::from(vec![
                    Span::raw(self.value),
                    Span::styled(" ", Theme::selected()),
                ])
            };

            let para = Paragraph::new(display_value);
            para.render(inner, buf);
        } else {
            let para = Paragraph::new(self.value);
            para.render(inner, buf);
        }
    }
}

pub struct SelectWidget<'a> {
    label: &'a str,
    options: &'a [&'a str],
    selected: usize,
    focused: bool,
}

impl<'a> SelectWidget<'a> {
    pub fn new(label: &'a str, options: &'a [&'a str], selected: usize) -> Self {
        Self {
            label,
            options,
            selected,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl Widget for SelectWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            Theme::border_focused()
        } else {
            Theme::border()
        };

        let block = Block::default()
            .title(self.label)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        let value = self.options.get(self.selected).unwrap_or(&"");
        let display = if self.focused {
            format!("< {} >", value)
        } else {
            value.to_string()
        };

        let style = if self.focused {
            Theme::highlight()
        } else {
            Theme::normal()
        };

        let para = Paragraph::new(Span::styled(display, style));
        para.render(inner, buf);
    }
}
