//! Common UI utilities and shared helper functions.

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
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

/// Draw input popup for command entry.
pub(super) fn draw_input_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());

    frame.render_widget(Clear, area);

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(" Enter command (get <key> | set <key> <value>) "));

    frame.render_widget(input, area);

    frame.set_cursor_position((area.x + app.input_buffer.len() as u16 + 1, area.y + 1));
}

/// Draw SQL input popup for query editing.
pub(super) fn draw_sql_input_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 40, frame.area());

    frame.render_widget(Clear, area);

    let title = format!(
        " Edit SQL Query (Enter to save, Esc to cancel, Up/Down for history [{}/{}]) ",
        if app.sql_state.history_browsing {
            app.sql_state.history_index + 1
        } else {
            app.sql_state.history.len() as u32
        },
        app.sql_state.history.len()
    );

    let input = Paragraph::new(app.sql_state.query_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(input, area);

    let cursor_x = area.x + (app.sql_state.query_buffer.len() as u16 % (area.width - 2)) + 1;
    let cursor_y = area.y + (app.sql_state.query_buffer.len() as u16 / (area.width - 2)) + 1;
    frame.set_cursor_position((cursor_x, cursor_y.min(area.y + area.height - 2)));
}

/// Create a centered rectangle.
pub(super) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    let duration = Duration::from_millis(ts_ms);
    let datetime = UNIX_EPOCH + duration;

    if let Ok(elapsed) = std::time::SystemTime::now().duration_since(datetime) {
        let secs = elapsed.as_secs();
        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    } else {
        "just now".to_string()
    }
}
