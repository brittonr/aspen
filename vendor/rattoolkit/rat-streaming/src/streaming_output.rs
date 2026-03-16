//! Streaming output buffer for in-progress tool executions.
//!
//! Provides a per-tool scrollable line buffer with head/tail truncation,
//! auto-follow, and scroll state. Replaces the fixed 8-line tail window
//! with a rich, interactive streaming output view.

use std::collections::HashMap;

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

/// Configuration for the streaming output buffer.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum lines to keep in the buffer before truncating.
    /// Head + tail must be ≤ max_lines.
    pub max_lines: usize,
    /// Lines to keep at the head (beginning) of output.
    pub head_lines: usize,
    /// Lines to keep at the tail (end) of output.
    pub tail_lines: usize,
    /// Default number of visible lines when rendering inline in chat.
    pub visible_lines: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_lines: 2000,
            head_lines: 200,
            tail_lines: 200,
            visible_lines: 16,
        }
    }
}

/// A scrollable output buffer for a single tool execution.
///
/// Stores lines with head/tail truncation when output exceeds `max_lines`.
/// Maintains scroll state and auto-follow mode for live viewing.
#[derive(Debug)]
pub struct StreamingOutput {
    /// Head lines (first N lines of output).
    head: Vec<String>,
    /// Tail lines (last N lines of output — ring buffer).
    tail: Vec<String>,
    /// Total lines received (including those dropped by truncation).
    total_lines: usize,
    /// Total bytes received.
    total_bytes: usize,
    /// Number of lines dropped (between head and tail).
    omitted: usize,
    /// Scroll offset (0 = top of visible window).
    scroll_offset: usize,
    /// Whether to auto-scroll to the bottom on new output.
    auto_follow: bool,
    /// Whether the user has explicitly focused this tool's output.
    pub focused: bool,
    /// Configuration.
    config: StreamingConfig,
}

impl StreamingOutput {
    /// Create a new streaming output buffer with default config.
    pub fn new() -> Self {
        Self::with_config(StreamingConfig::default())
    }

    /// Create with custom config.
    pub fn with_config(config: StreamingConfig) -> Self {
        Self {
            head: Vec::new(),
            tail: Vec::new(),
            total_lines: 0,
            total_bytes: 0,
            omitted: 0,
            scroll_offset: 0,
            auto_follow: true,
            focused: false,
            config,
        }
    }

    /// Append a line of output.
    pub fn push_line(&mut self, line: &str) {
        self.total_lines += 1;
        self.total_bytes += line.len() + 1; // +1 for newline

        if self.head.len() < self.config.head_lines {
            // Still filling the head buffer.
            self.head.push(line.to_string());
        } else {
            // Head is full. Accumulate into tail, which acts as a ring buffer
            // keeping the last `tail_lines` entries.
            if self.tail.len() >= self.config.tail_lines {
                // Tail is full — drop the oldest tail entry.
                self.tail.remove(0);
                self.omitted += 1;
            }
            self.tail.push(line.to_string());
        }

        // Auto-follow: keep scroll at bottom.
        if self.auto_follow {
            self.scroll_to_bottom();
        }
    }

    /// Push multi-line text (splits on newlines).
    pub fn push_text(&mut self, text: &str) {
        for line in text.lines() {
            self.push_line(line);
        }
    }

    /// Total number of displayable lines (head + omission marker + tail).
    pub fn display_line_count(&self) -> usize {
        let base = self.head.len() + self.tail.len();
        if self.omitted > 0 {
            base + 1 // +1 for the omission marker line
        } else {
            base
        }
    }

    /// Total lines received (including truncated).
    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    /// Total bytes received.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Number of lines omitted by truncation.
    pub fn omitted(&self) -> usize {
        self.omitted
    }

    /// Whether auto-follow is enabled.
    pub fn auto_follow(&self) -> bool {
        self.auto_follow
    }

    /// Toggle auto-follow mode.
    pub fn toggle_auto_follow(&mut self) {
        self.auto_follow = !self.auto_follow;
        if self.auto_follow {
            self.scroll_to_bottom();
        }
    }

    /// Scroll up by `n` lines.
    pub fn scroll_up(&mut self, n: usize) {
        self.auto_follow = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down by `n` lines within the visible window.
    pub fn scroll_down(&mut self, n: usize, visible_height: usize) {
        self.auto_follow = false;
        let max = self.display_line_count().saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + n).min(max);
        // Re-enable auto-follow if we're at the bottom.
        if self.scroll_offset >= max {
            self.auto_follow = true;
        }
    }

    /// Scroll to the very top.
    pub fn scroll_to_top(&mut self) {
        self.auto_follow = false;
        self.scroll_offset = 0;
    }

    /// Scroll to the very bottom.
    pub fn scroll_to_bottom(&mut self) {
        // We'll clamp in render; just set a large value.
        self.scroll_offset = usize::MAX;
        self.auto_follow = true;
    }

    /// Render lines for the inline chat view.
    ///
    /// Returns the visible slice of lines based on scroll state and
    /// `visible_height`. Lines are prefixed with the chat border and
    /// tool-output style.
    ///
    /// `visible_height` is how many lines of output to show (not counting
    /// the stats footer).
    pub fn render_lines<'a>(&mut self, visible_height: usize, border_style: Style) -> Vec<Line<'a>> {
        let output_style = Style::default().fg(Color::DarkGray);
        let omit_style = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);

        // Build the full logical line list.
        let display_count = self.display_line_count();

        // Clamp scroll offset.
        let max_offset = display_count.saturating_sub(visible_height);
        if self.auto_follow || self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }

        let start = self.scroll_offset;
        let end = (start + visible_height).min(display_count);

        let mut result = Vec::with_capacity(end - start);

        for i in start..end {
            let line = self.get_display_line(i);
            match line {
                DisplayLine::Text(text) => {
                    result.push(Line::from(vec![
                        Span::styled("│ ", border_style),
                        Span::styled(format!("  │ {}", text), output_style),
                    ]));
                }
                DisplayLine::Omitted(n) => {
                    result.push(Line::from(vec![
                        Span::styled("│ ", border_style),
                        Span::styled(format!("  ┄ {} lines omitted ┄", n), omit_style),
                    ]));
                }
            }
        }

        result
    }

    /// Render a compact stats footer line.
    pub fn render_stats<'a>(&self, border_style: Style) -> Line<'a> {
        let stats_style = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);

        let follow_indicator = if self.auto_follow { "↓follow" } else { "scroll" };
        let truncated = if self.omitted > 0 {
            format!(" ({} omitted)", self.omitted)
        } else {
            String::new()
        };

        Line::from(vec![
            Span::styled("│ ", border_style),
            Span::styled(
                format!(
                    "  {} lines · {} · {}{}",
                    self.total_lines,
                    format_bytes(self.total_bytes),
                    follow_indicator,
                    truncated,
                ),
                stats_style,
            ),
        ])
    }

    /// Get a logical display line by index.
    fn get_display_line(&self, index: usize) -> DisplayLine {
        let head_len = self.head.len();

        if index < head_len {
            DisplayLine::Text(self.head[index].clone())
        } else if self.omitted > 0 && index == head_len {
            DisplayLine::Omitted(self.omitted)
        } else {
            let tail_index = if self.omitted > 0 {
                index - head_len - 1
            } else {
                index - head_len
            };
            if tail_index < self.tail.len() {
                DisplayLine::Text(self.tail[tail_index].clone())
            } else {
                // Shouldn't happen, but be safe.
                DisplayLine::Text(String::new())
            }
        }
    }
}

impl Default for StreamingOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal enum for get_display_line.
enum DisplayLine {
    Text(String),
    Omitted(usize),
}

/// Manages streaming output buffers for all active tool executions.
#[derive(Debug)]
pub struct StreamingOutputManager {
    /// Per-call_id output buffers.
    outputs: HashMap<String, StreamingOutput>,
    /// Default config for new buffers.
    config: StreamingConfig,
}

impl StreamingOutputManager {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            config: StreamingConfig::default(),
        }
    }

    /// Get or create a streaming output for a call_id.
    fn get_or_create(&mut self, call_id: &str) -> &mut StreamingOutput {
        self.outputs
            .entry(call_id.to_string())
            .or_insert_with(|| StreamingOutput::with_config(self.config.clone()))
    }

    /// Add a line of output for a tool.
    pub fn add_line(&mut self, call_id: &str, line: &str) {
        self.get_or_create(call_id).push_line(line);
    }

    /// Add multi-line text for a tool.
    pub fn add_text(&mut self, call_id: &str, text: &str) {
        self.get_or_create(call_id).push_text(text);
    }

    /// Get a streaming output buffer (immutable).
    pub fn get(&self, call_id: &str) -> Option<&StreamingOutput> {
        self.outputs.get(call_id)
    }

    /// Get a streaming output buffer (mutable, for scroll control).
    pub fn get_mut(&mut self, call_id: &str) -> Option<&mut StreamingOutput> {
        self.outputs.get_mut(call_id)
    }

    /// Remove a completed tool's buffer.
    pub fn remove(&mut self, call_id: &str) {
        self.outputs.remove(call_id);
    }

    /// Check if any output exists for a call_id.
    pub fn has(&self, call_id: &str) -> bool {
        self.outputs.contains_key(call_id)
    }

    /// Get the call_id of the currently focused tool output, if any.
    pub fn focused_call_id(&self) -> Option<&str> {
        self.outputs.iter().find(|(_, out)| out.focused).map(|(id, _)| id.as_str())
    }

    /// Focus a specific tool's output.
    pub fn focus(&mut self, call_id: &str) {
        // Unfocus all others.
        for out in self.outputs.values_mut() {
            out.focused = false;
        }
        if let Some(out) = self.outputs.get_mut(call_id) {
            out.focused = true;
        }
    }

    /// Unfocus all tool outputs.
    pub fn unfocus_all(&mut self) {
        for out in self.outputs.values_mut() {
            out.focused = false;
        }
    }

    /// Number of active tool output buffers.
    pub fn active_count(&self) -> usize {
        self.outputs.len()
    }
}

impl Default for StreamingOutputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a byte count as human-readable.
fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_line_basic() {
        let mut out = StreamingOutput::new();
        out.push_line("hello");
        out.push_line("world");
        assert_eq!(out.total_lines(), 2);
        assert_eq!(out.display_line_count(), 2);
        assert_eq!(out.omitted(), 0);
    }

    #[test]
    fn push_text_splits_lines() {
        let mut out = StreamingOutput::new();
        out.push_text("line 1\nline 2\nline 3");
        assert_eq!(out.total_lines(), 3);
        assert_eq!(out.display_line_count(), 3);
    }

    #[test]
    fn head_tail_truncation() {
        let config = StreamingConfig {
            max_lines: 10,
            head_lines: 3,
            tail_lines: 3,
            visible_lines: 16,
        };
        let mut out = StreamingOutput::with_config(config);

        for i in 0..20 {
            out.push_line(&format!("line {}", i));
        }

        assert_eq!(out.total_lines(), 20);
        // Head: 3 lines + omission marker + tail: 3 lines = 7 display lines
        assert_eq!(out.head.len(), 3);
        assert_eq!(out.tail.len(), 3);
        assert_eq!(out.omitted(), 14); // 20 - 3 head - 3 tail = 14 omitted
        assert_eq!(out.display_line_count(), 7); // 3 + 1 + 3

        // Verify head content.
        assert_eq!(out.head[0], "line 0");
        assert_eq!(out.head[1], "line 1");
        assert_eq!(out.head[2], "line 2");

        // Verify tail content (last 3 lines).
        assert_eq!(out.tail[0], "line 17");
        assert_eq!(out.tail[1], "line 18");
        assert_eq!(out.tail[2], "line 19");
    }

    #[test]
    fn display_line_ordering() {
        let config = StreamingConfig {
            max_lines: 10,
            head_lines: 2,
            tail_lines: 2,
            visible_lines: 16,
        };
        let mut out = StreamingOutput::with_config(config);

        for i in 0..10 {
            out.push_line(&format!("L{}", i));
        }

        // Display lines: L0, L1, [6 omitted], L8, L9
        assert_eq!(out.display_line_count(), 5);
        assert!(matches!(out.get_display_line(0), DisplayLine::Text(s) if s == "L0"));
        assert!(matches!(out.get_display_line(1), DisplayLine::Text(s) if s == "L1"));
        assert!(matches!(out.get_display_line(2), DisplayLine::Omitted(6)));
        assert!(matches!(out.get_display_line(3), DisplayLine::Text(s) if s == "L8"));
        assert!(matches!(out.get_display_line(4), DisplayLine::Text(s) if s == "L9"));
    }

    #[test]
    fn scroll_up_down() {
        let config = StreamingConfig {
            max_lines: 2000,
            head_lines: 200,
            tail_lines: 200,
            visible_lines: 5,
        };
        let mut out = StreamingOutput::with_config(config);

        for i in 0..30 {
            out.push_line(&format!("line {}", i));
        }

        // Auto-follow should place us at the bottom.
        assert!(out.auto_follow());

        // Scroll up disables auto-follow.
        out.scroll_up(5);
        assert!(!out.auto_follow());

        // Scroll to bottom re-enables.
        out.scroll_to_bottom();
        assert!(out.auto_follow());
    }

    #[test]
    fn scroll_to_top() {
        let mut out = StreamingOutput::new();
        for i in 0..50 {
            out.push_line(&format!("line {}", i));
        }

        out.scroll_to_top();
        assert!(!out.auto_follow());
        assert_eq!(out.scroll_offset, 0);
    }

    #[test]
    fn toggle_auto_follow() {
        let mut out = StreamingOutput::new();
        assert!(out.auto_follow());

        out.toggle_auto_follow();
        assert!(!out.auto_follow());

        out.toggle_auto_follow();
        assert!(out.auto_follow());
    }

    #[test]
    fn render_lines_basic() {
        let mut out = StreamingOutput::new();
        out.push_line("hello");
        out.push_line("world");

        let border = Style::default().fg(Color::DarkGray);
        let lines = out.render_lines(10, border);
        assert_eq!(lines.len(), 2);

        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("hello"));
    }

    #[test]
    fn render_lines_with_truncation() {
        let config = StreamingConfig {
            max_lines: 10,
            head_lines: 2,
            tail_lines: 2,
            visible_lines: 16,
        };
        let mut out = StreamingOutput::with_config(config);

        for i in 0..10 {
            out.push_line(&format!("L{}", i));
        }

        let border = Style::default().fg(Color::DarkGray);
        // Show all display lines (5 total: 2 head + omit + 2 tail).
        let lines = out.render_lines(10, border);
        assert_eq!(lines.len(), 5);

        // Check omission marker.
        let omit_text: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(omit_text.contains("6 lines omitted"));
    }

    #[test]
    fn render_stats_footer() {
        let mut out = StreamingOutput::new();
        out.push_text("hello\nworld");

        let border = Style::default().fg(Color::DarkGray);
        let stats = out.render_stats(border);
        let text: String = stats.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("2 lines"));
        assert!(text.contains("follow"));
    }

    #[test]
    fn manager_add_and_get() {
        let mut mgr = StreamingOutputManager::new();
        mgr.add_line("call-1", "hello");
        mgr.add_line("call-1", "world");

        assert!(mgr.has("call-1"));
        assert!(!mgr.has("call-2"));

        let out = mgr.get("call-1").unwrap();
        assert_eq!(out.total_lines(), 2);
    }

    #[test]
    fn manager_remove() {
        let mut mgr = StreamingOutputManager::new();
        mgr.add_line("call-1", "hello");
        assert!(mgr.has("call-1"));

        mgr.remove("call-1");
        assert!(!mgr.has("call-1"));
    }

    #[test]
    fn manager_focus() {
        let mut mgr = StreamingOutputManager::new();
        mgr.add_line("call-1", "a");
        mgr.add_line("call-2", "b");

        mgr.focus("call-1");
        assert_eq!(mgr.focused_call_id(), Some("call-1"));

        mgr.focus("call-2");
        assert_eq!(mgr.focused_call_id(), Some("call-2"));
        assert!(!mgr.get("call-1").unwrap().focused);
    }

    #[test]
    fn manager_unfocus_all() {
        let mut mgr = StreamingOutputManager::new();
        mgr.add_line("call-1", "a");
        mgr.focus("call-1");
        assert!(mgr.focused_call_id().is_some());

        mgr.unfocus_all();
        assert!(mgr.focused_call_id().is_none());
    }

    #[test]
    fn format_bytes_display() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(2 * 1024 * 1024 + 512 * 1024), "2.5 MB");
    }

    #[test]
    fn empty_buffer_render() {
        let mut out = StreamingOutput::new();
        let border = Style::default().fg(Color::DarkGray);
        let lines = out.render_lines(10, border);
        assert!(lines.is_empty());
    }

    #[test]
    fn scroll_down_clamps() {
        let mut out = StreamingOutput::new();
        for i in 0..5 {
            out.push_line(&format!("line {}", i));
        }

        out.scroll_to_top();
        // With visible_height=3 and 5 lines, max offset = 2
        out.scroll_down(100, 3);
        // Should clamp and re-enable auto-follow.
        assert!(out.auto_follow());
    }

    #[test]
    fn bytes_tracking() {
        let mut out = StreamingOutput::new();
        out.push_line("hello"); // 5 chars + 1 newline = 6
        out.push_line("world"); // 5 + 1 = 6
        assert_eq!(out.total_bytes(), 12);
    }

    #[test]
    fn no_omission_marker_when_not_truncated() {
        let mut out = StreamingOutput::new();
        for i in 0..5 {
            out.push_line(&format!("line {}", i));
        }
        assert_eq!(out.omitted(), 0);
        assert_eq!(out.display_line_count(), 5); // No +1 for marker.
    }
}
