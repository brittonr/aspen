//! Rendering and visual position calculations

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::Editor;

impl Editor {
    /// Compute the number of visual (wrapped) lines given an available width.
    /// `indicator_len` is the length of the prompt indicator on the first line.
    pub fn visual_line_count(&self, width: usize, indicator_len: usize) -> usize {
        if width == 0 {
            return self.lines.len();
        }
        self.lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let prefix_len = if i == 0 { indicator_len } else { 2 }; // "  " indent
                let content_len = prefix_len + line.len();
                if content_len == 0 {
                    1
                } else {
                    content_len.div_ceil(width) // ceil division
                }
            })
            .sum()
    }

    /// Compute the visual (x, y) cursor position accounting for wrapping.
    /// Returns (col, row) relative to the inner area of the editor widget.
    pub fn visual_cursor_position(&self, width: usize, indicator_len: usize) -> (u16, u16) {
        if width == 0 {
            return (0, 0);
        }

        let mut visual_row: usize = 0;

        // Count visual rows from lines before the cursor line
        for (i, line) in self.lines.iter().enumerate() {
            if i == self.cursor_line {
                break;
            }
            let prefix_len = if i == 0 { indicator_len } else { 2 };
            let content_len = prefix_len + line.len();
            visual_row += if content_len == 0 {
                1
            } else {
                content_len.div_ceil(width)
            };
        }

        // Now compute position within the cursor line
        let prefix_len = if self.cursor_line == 0 { indicator_len } else { 2 };
        let offset_in_line = prefix_len + self.cursor_col;
        visual_row += offset_in_line / width;
        let visual_col = offset_in_line % width;

        (visual_col as u16, visual_row as u16)
    }
}

/// Render the editor widget
pub fn render_editor(
    frame: &mut Frame,
    editor: &Editor,
    area: Rect,
    indicator: &str,
    border_color: Color,
    title: &str,
) {
    let lines: Vec<Line> = editor
        .content()
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                Line::from(vec![Span::raw(indicator), Span::raw(line.as_str())])
            } else {
                Line::from(Span::raw(format!("  {}", line)))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title.to_string());

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);

    // Place the cursor accounting for wrapping and borders
    let inner_width = area.width.saturating_sub(2) as usize; // subtract left+right border
    let (cx, cy) = editor.visual_cursor_position(inner_width, indicator.len());
    frame.set_cursor_position((
        area.x + 1 + cx, // +1 for left border
        area.y + 1 + cy, // +1 for top border
    ));
}