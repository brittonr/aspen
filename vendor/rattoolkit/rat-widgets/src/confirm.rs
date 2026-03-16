//! Confirmation dialog

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
use ratatui::widgets::Wrap;

pub struct ConfirmDialog {
    pub message: String,
    pub selected: bool, // true = Yes, false = No
    pub visible: bool,
}

impl ConfirmDialog {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            selected: false,
            visible: true,
        }
    }

    pub fn toggle(&mut self) {
        self.selected = !self.selected;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let width = 40.min(area.width.saturating_sub(4));
        let height = 5;
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let yes_style = if self.selected {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let no_style = if !self.selected {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let content = vec![
            Line::from(self.message.clone()),
            Line::from(""),
            Line::from(vec![Span::styled("  [Yes] ", yes_style), Span::styled("  [No] ", no_style)]),
        ];

        let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, popup);
    }
}