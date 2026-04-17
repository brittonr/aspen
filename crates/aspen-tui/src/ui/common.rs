//! Common UI utilities and shared helper functions.

use std::num::NonZeroU16;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::app::App;

struct PopupSizePercent {
    width_percent: u16,
    height_percent: u16,
}

fn saturating_u16_from_usize(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn popup_margin_percent(content_percent: u16) -> u16 {
    debug_assert!(content_percent <= 100);
    100_u16.saturating_sub(content_percent) / 2
}

#[allow(
    unknown_lints,
    ambient_clock,
    reason = "TUI relative timestamps need current wall clock at render time"
)]
fn current_wall_time() -> SystemTime {
    SystemTime::now()
}

fn elapsed_since_timestamp(timestamp: SystemTime, current_time: SystemTime) -> Option<Duration> {
    current_time.duration_since(timestamp).ok()
}

fn format_elapsed(elapsed: Duration) -> String {
    debug_assert!(elapsed >= Duration::ZERO);
    debug_assert!(elapsed.as_secs() <= u64::MAX / 2);
    let elapsed_secs = elapsed.as_secs();

    match elapsed_secs {
        secs if secs < 60 => format!("{}s ago", secs),
        secs if secs < 3600 => format!("{}m ago", secs / 60),
        secs if secs < 86_400 => format!("{}h ago", secs / 3600),
        secs => format!("{}d ago", secs / 86_400),
    }
}

/// Draw input popup for command entry.
pub(super) fn draw_input_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(
        PopupSizePercent {
            width_percent: 60,
            height_percent: 20,
        },
        frame.area(),
    );
    debug_assert!(area.width > 2);
    debug_assert!(area.height > 2);

    frame.render_widget(Clear, area);

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::new().fg(Color::Yellow))
        .block(Block::new().borders(Borders::ALL).title(" Enter command (get <key> | set <key> <value>) "));

    frame.render_widget(input, area);

    let cursor_column_index = saturating_u16_from_usize(app.input_buffer.len());
    let max_cursor_column_index = area.width.saturating_sub(2);
    let cursor_x = area.x.saturating_add(1).saturating_add(cursor_column_index.min(max_cursor_column_index));
    let cursor_y = area.y.saturating_add(1);
    frame.set_cursor_position((cursor_x, cursor_y));
}

/// Draw SQL input popup for query editing.
pub(super) fn draw_sql_input_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(
        PopupSizePercent {
            width_percent: 80,
            height_percent: 40,
        },
        frame.area(),
    );
    debug_assert!(area.width > 2);
    debug_assert!(area.height > 2);

    frame.render_widget(Clear, area);

    let (history_position, history_len) = app.sql_state.history.position();
    let title = format!(
        " Edit SQL Query (Enter to save, Esc to cancel, Up/Down for history [{}/{}]) ",
        history_position, history_len
    );

    let input = Paragraph::new(app.sql_state.query_buffer.as_str())
        .style(Style::new().fg(Color::Yellow))
        .block(Block::new().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(input, area);

    let Some(inner_width_columns) = NonZeroU16::new(area.width.saturating_sub(2)) else {
        frame.set_cursor_position((area.x.saturating_add(1), area.y.saturating_add(1)));
        return;
    };
    let inner_width_columns = inner_width_columns.get();
    let query_len_cols = saturating_u16_from_usize(app.sql_state.query_buffer.len());
    let cursor_column = query_len_cols.checked_rem(inner_width_columns).unwrap_or(0);
    let cursor_row = query_len_cols.checked_div(inner_width_columns).unwrap_or(0);
    let cursor_x = area.x.saturating_add(1).saturating_add(cursor_column);
    let max_cursor_y = area.y.saturating_add(area.height.saturating_sub(2));
    let cursor_y = area.y.saturating_add(1).saturating_add(cursor_row).min(max_cursor_y);
    frame.set_cursor_position((cursor_x, cursor_y));
}

/// Create a centered rectangle.
fn centered_rect(size_percent: PopupSizePercent, area: Rect) -> Rect {
    debug_assert!(size_percent.width_percent <= 100);
    debug_assert!(size_percent.height_percent <= 100);
    let vertical_margin_percent = popup_margin_percent(size_percent.height_percent);
    let vertical_layout = Layout::vertical([
        Constraint::Percentage(vertical_margin_percent),
        Constraint::Percentage(size_percent.height_percent),
        Constraint::Percentage(vertical_margin_percent),
    ])
    .split(area);

    let horizontal_margin_percent = popup_margin_percent(size_percent.width_percent);
    Layout::horizontal([
        Constraint::Percentage(horizontal_margin_percent),
        Constraint::Percentage(size_percent.width_percent),
        Constraint::Percentage(horizontal_margin_percent),
    ])
    .split(vertical_layout[1])[1]
}

/// Get color for CI status.
pub(super) fn ci_status_color(status: &str) -> Color {
    match status {
        "initializing" | "checking_out" => Color::Cyan,
        "pending" => Color::Yellow,
        "running" => Color::Blue,
        "success" => Color::Green,
        "failed" | "checkout_failed" => Color::Red,
        "cancelled" => Color::DarkGray,
        _ => Color::White,
    }
}

/// Format a timestamp in milliseconds to a human-readable string.
pub(super) fn format_timestamp_ms(ts_ms: u64) -> String {
    debug_assert!(ts_ms <= u64::MAX / 2);
    debug_assert!(UNIX_EPOCH.checked_add(Duration::from_millis(ts_ms)).is_some());
    let timestamp_age = Duration::from_millis(ts_ms);
    let timestamp = UNIX_EPOCH + timestamp_age;
    let current_time = current_wall_time();

    elapsed_since_timestamp(timestamp, current_time)
        .map(format_elapsed)
        .unwrap_or_else(|| "just now".to_string())
}
