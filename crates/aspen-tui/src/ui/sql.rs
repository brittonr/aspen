//! SQL query view rendering.

use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::app::App;
use crate::types::SqlConsistency;

/// Draw SQL query view.
pub(super) fn draw_sql_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Query editor
            Constraint::Min(0),    // Results table
            Constraint::Length(1), // Info bar
        ])
        .split(area);

    draw_sql_editor(frame, app, chunks[0]);
    draw_sql_results(frame, app, chunks[1]);
    draw_sql_info_bar(frame, app, chunks[2]);
}

/// Draw SQL query editor.
fn draw_sql_editor(frame: &mut Frame, app: &App, area: Rect) {
    let consistency_indicator = match app.sql_state.consistency {
        SqlConsistency::Linearizable => "[L]",
        SqlConsistency::Stale => "[S]",
    };

    let title = format!(" SQL Query {} (Enter to edit, 'e' to execute, 'c' to toggle) ", consistency_indicator);

    let query_display = if app.sql_state.query_buffer.is_empty() {
        "-- Example: SELECT key, value FROM kv WHERE key LIKE 'config:%' LIMIT 100".to_string()
    } else {
        app.sql_state.query_buffer.clone()
    };

    let paragraph = Paragraph::new(query_display)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Draw SQL results table using rat-table DataTable widget.
fn draw_sql_results(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(result) = &app.sql_state.last_result {
        if !result.is_success {
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");
            let paragraph = Paragraph::new(error_msg)
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title(" Error "));
            frame.render_widget(paragraph, area);
            return;
        }
    }

    if app.sql_table.is_empty() {
        let msg = if app.sql_state.last_result.is_some() {
            "No results"
        } else {
            "Execute a query to see results"
        };
        let paragraph = Paragraph::new(msg).block(Block::default().borders(Borders::ALL).title(" Results "));
        frame.render_widget(paragraph, area);
        return;
    }

    let info = app.sql_table.info();
    let title =
        format!(" Results ({} rows, {} columns) [j/k navigate, h/l scroll] ", info.row_count, info.column_count);
    let block = Block::default().borders(Borders::ALL).title(title);

    let style = rat_table::DataTableStyle {
        header_style: Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow),
        selected_style: Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
        normal_style: Style::default(),
        truncation_suffix: "...".to_string(),
        column_spacing: 1,
    };

    app.sql_table.render(frame, area, Some(block), &style);
}

/// Draw SQL info bar with execution stats.
fn draw_sql_info_bar(frame: &mut Frame, app: &App, area: Rect) {
    let info = if let Some(result) = &app.sql_state.last_result {
        if result.is_success {
            let truncated = if result.is_truncated { "Yes" } else { "No" };
            format!(
                " Rows: {} | Time: {}ms | Truncated: {} | Consistency: {} ",
                result.row_count,
                result.execution_time_ms,
                truncated,
                app.sql_state.consistency.as_str()
            )
        } else {
            format!(" Query failed | Consistency: {} ", app.sql_state.consistency.as_str())
        }
    } else {
        format!(" No query executed | Consistency: {} ", app.sql_state.consistency.as_str())
    };

    let paragraph = Paragraph::new(info).style(Style::default().fg(Color::Cyan)).alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
