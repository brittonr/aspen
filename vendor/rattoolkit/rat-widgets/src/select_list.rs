//! Selection list dialog

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;

pub struct SelectList {
    pub title: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub visible: bool,
}

impl SelectList {
    pub fn new(title: impl Into<String>, items: Vec<String>) -> Self {
        Self {
            title: title.into(),
            items,
            selected: 0,
            visible: true,
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(self.items.len().saturating_sub(1));
    }

    pub fn select(&self) -> Option<String> {
        self.items.get(self.selected).cloned()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let width = 40.min(area.width.saturating_sub(4));
        let height = (self.items.len() as u16 + 2).min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default().bg(Color::DarkGray).fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Span::styled(item.clone(), style))
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, popup);
    }
}