//! UI rendering for the Aspen TUI.
//!
//! Implements the View layer of The Elm Architecture (TEA).

mod ci;
mod cluster;
mod common;
mod help;
mod jobs;
mod kv;
mod logs;
mod sql;

use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Tabs;

use crate::app::ActiveView;
use crate::app::App;
use crate::app::InputMode;

fn active_view_tab_index(active_view: &ActiveView) -> usize {
    match active_view {
        ActiveView::Cluster => 0,
        ActiveView::Metrics => 1,
        ActiveView::KeyValue => 2,
        ActiveView::Vaults => 3,
        ActiveView::Sql => 4,
        ActiveView::Logs => 5,
        ActiveView::Jobs => 6,
        ActiveView::Workers => 7,
        ActiveView::Ci => 8,
        ActiveView::Help => 9,
    }
}

fn tab_title(view: &ActiveView, active_view: &ActiveView) -> Line<'static> {
    let style = if view == active_view {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::White)
    };

    Line::from(Span::styled(format!(" {} ", view.name()), style))
}

fn status_message(app: &App) -> String {
    if let Some(notif) = &app.notification {
        notif.message.clone()
    } else if app.refreshing {
        "Refreshing...".to_string()
    } else if let Some(last_refresh) = app.last_refresh {
        format!("Last refresh: {:.1}s ago", last_refresh.elapsed().as_secs_f64())
    } else {
        "Press 'r' to refresh".to_string()
    }
}

fn mode_label(input_mode: InputMode) -> &'static str {
    match input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDITING",
        InputMode::SqlEditing => "SQL EDIT",
    }
}

fn mode_background(input_mode: InputMode) -> Color {
    match input_mode {
        InputMode::Normal => Color::Green,
        InputMode::Editing => Color::Yellow,
        InputMode::SqlEditing => Color::Cyan,
    }
}

/// Main draw function.
///
/// Tiger Style: Single entry point for all rendering.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header with tabs
        Constraint::Min(0),    // Main content
        Constraint::Length(3), // Status bar
    ])
    .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);

    if app.input_mode == InputMode::Editing {
        common::draw_input_popup(frame, app);
    }

    if app.input_mode == InputMode::SqlEditing {
        common::draw_sql_input_popup(frame, app);
    }
}

/// Draw the header with tab navigation.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let selected_tab = active_view_tab_index(&app.active_view);
    debug_assert!(selected_tab < 10);

    let titles: Vec<Line> = [
        ActiveView::Cluster,
        ActiveView::Metrics,
        ActiveView::KeyValue,
        ActiveView::Vaults,
        ActiveView::Sql,
        ActiveView::Logs,
        ActiveView::Jobs,
        ActiveView::Workers,
        ActiveView::Ci,
        ActiveView::Help,
    ]
    .iter()
    .map(|view| tab_title(view, &app.active_view))
    .collect();

    let tabs = Tabs::new(titles)
        .block(Block::new().borders(Borders::ALL).title(" Aspen Cluster Manager "))
        .select(selected_tab)
        .style(Style::new().fg(Color::White))
        .highlight_style(Style::new().fg(Color::Yellow));

    frame.render_widget(tabs, area);
}

/// Draw main content area based on active view.
fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_view {
        ActiveView::Cluster => cluster::draw_cluster_view(frame, app, area),
        ActiveView::Metrics => cluster::draw_metrics_view(frame, app, area),
        ActiveView::KeyValue => kv::draw_kv_view(frame, app, area),
        ActiveView::Vaults => kv::draw_vaults_view(frame, app, area),
        ActiveView::Sql => sql::draw_sql_view(frame, app, area),
        ActiveView::Logs => logs::draw_logs_view(frame, app, area),
        ActiveView::Jobs => jobs::draw_jobs_view(frame, app, area),
        ActiveView::Workers => jobs::draw_workers_view(frame, app, area),
        ActiveView::Ci => ci::draw_ci_view(frame, app, area),
        ActiveView::Help => help::draw_help_view(frame, area),
    }
}

/// Draw status bar at bottom.
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let status = status_message(app);
    debug_assert!(!status.is_empty());
    let mode = mode_label(app.input_mode);
    debug_assert!(!mode.is_empty());

    let chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(12)]).split(area);

    let status_widget = Paragraph::new(status)
        .style(Style::new().fg(Color::White))
        .block(Block::new().borders(Borders::ALL));

    let mode_widget = Paragraph::new(mode)
        .style(Style::new().fg(Color::Black).bg(mode_background(app.input_mode)))
        .alignment(Alignment::Center)
        .block(Block::new().borders(Borders::ALL));

    frame.render_widget(status_widget, chunks[0]);
    frame.render_widget(mode_widget, chunks[1]);
}
