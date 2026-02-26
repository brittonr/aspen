//! Logs view rendering.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;

use crate::app::App;

/// Draw logs view with tui-logger integration.
pub(super) fn draw_logs_view(frame: &mut Frame, app: &App, area: Rect) {
    let tui_smart_widget = tui_logger::TuiLoggerSmartWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_info(Style::default().fg(Color::Green))
        .style_debug(Style::default().fg(Color::Cyan))
        .style_trace(Style::default().fg(Color::Gray))
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(tui_logger::TuiLoggerLevelOutput::Abbreviated))
        .output_target(true)
        .output_file(false)
        .output_line(false)
        .title_log(" Logs ")
        .title_target(" Targets ");

    frame.render_widget(tui_smart_widget, area);

    // Suppress unused variable warning
    let _ = app.log_scroll;
}
