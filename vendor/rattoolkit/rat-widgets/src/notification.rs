//! Toast notification overlay

use std::time::Duration;
use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    pub created: Instant,
    pub ttl: Duration,
}

impl Notification {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Info,
            created: Instant::now(),
            ttl: Duration::from_secs(3),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Warning,
            created: Instant::now(),
            ttl: Duration::from_secs(5),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: NotificationLevel::Error,
            created: Instant::now(),
            ttl: Duration::from_secs(8),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created.elapsed() >= self.ttl
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (border_color, title) = match self.level {
            NotificationLevel::Info => (Color::Blue, " Info "),
            NotificationLevel::Warning => (Color::Yellow, " Warning "),
            NotificationLevel::Error => (Color::Red, " Error "),
        };

        let width = 50.min(area.width.saturating_sub(4));
        let height = 3;
        let x = area.width.saturating_sub(width).saturating_sub(2);
        let y = 1;
        let popup = Rect::new(x, y, width, height);

        frame.render_widget(Clear, popup);
        let block = Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(border_color));
        let paragraph = Paragraph::new(Line::from(self.message.clone())).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, popup);
    }
}