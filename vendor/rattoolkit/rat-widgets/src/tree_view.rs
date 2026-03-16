//! Tree view for session navigation

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

pub struct TreeNode {
    pub label: String,
    pub id: String,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub depth: usize,
}

pub struct TreeView {
    pub root: Vec<TreeNode>,
    pub selected: usize,
    pub visible: bool,
    flat_cache: Vec<(String, String, usize)>, // (label, id, depth)
}

impl TreeView {
    pub fn new(root: Vec<TreeNode>) -> Self {
        let mut tv = Self {
            root,
            selected: 0,
            visible: false,
            flat_cache: Vec::new(),
        };
        tv.rebuild_cache();
        tv
    }

    fn rebuild_cache(&mut self) {
        self.flat_cache.clear();
        fn flatten(nodes: &[TreeNode], cache: &mut Vec<(String, String, usize)>) {
            for node in nodes {
                cache.push((node.label.clone(), node.id.clone(), node.depth));
                if node.expanded {
                    flatten(&node.children, cache);
                }
            }
        }
        flatten(&self.root, &mut self.flat_cache);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }
    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(self.flat_cache.len().saturating_sub(1));
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.flat_cache.get(self.selected).map(|(_, id, _)| id.as_str())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let width = 50.min(area.width.saturating_sub(4));
        let height = 20.min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let popup = Rect::new(x, y, width, height);

        frame.render_widget(Clear, popup);
        let block = Block::default()
            .title(" Sessions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let lines: Vec<Line> = self
            .flat_cache
            .iter()
            .enumerate()
            .map(|(i, (label, _, depth))| {
                let indent = "  ".repeat(*depth);
                let style = if i == self.selected {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                Line::from(Span::styled(format!("{}{}", indent, label), style))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }
}