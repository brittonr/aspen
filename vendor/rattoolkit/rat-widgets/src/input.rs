//! Single-line input dialog

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

pub struct InputDialog {
    pub title: String,
    pub value: String,
    pub visible: bool,
}

impl InputDialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            value: String::new(),
            visible: true,
        }
    }

    pub fn type_char(&mut self, c: char) {
        self.value.push(c);
    }

    pub fn backspace(&mut self) {
        self.value.pop();
    }

    pub fn submit(&mut self) -> String {
        let val = std::mem::take(&mut self.value);
        self.visible = false;
        val
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let width = 50.min(area.width.saturating_sub(4));
        let height = 3;
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let input_line = Line::from(vec![
            Span::styled(&self.value, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::White).add_modifier(Modifier::SLOW_BLINK)),
        ]);
        frame.render_widget(Paragraph::new(input_line), inner);
    }
}