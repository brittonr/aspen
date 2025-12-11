//! UI rendering for the Aspen TUI.
//!
//! Implements the View layer of The Elm Architecture (TEA).

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap},
};

use crate::app::{ActiveView, App, InputMode};
use crate::client::NodeStatus;

/// Main draw function.
///
/// Tiger Style: Single entry point for all rendering.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);

    // Draw input popup if in editing mode
    if app.input_mode == InputMode::Editing {
        draw_input_popup(frame, app);
    }
}

/// Draw the header with tab navigation.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = [
        ActiveView::Cluster,
        ActiveView::Metrics,
        ActiveView::KeyValue,
        ActiveView::Vaults,
        ActiveView::Logs,
        ActiveView::Help,
    ]
    .iter()
    .map(|v| {
        let style = if *v == app.active_view {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        Line::from(Span::styled(format!(" {} ", v.name()), style))
    })
    .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Aspen Cluster Manager "),
        )
        .select(match app.active_view {
            ActiveView::Cluster => 0,
            ActiveView::Metrics => 1,
            ActiveView::KeyValue => 2,
            ActiveView::Vaults => 3,
            ActiveView::Logs => 4,
            ActiveView::Help => 5,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow));

    frame.render_widget(tabs, area);
}

/// Draw main content area based on active view.
fn draw_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_view {
        ActiveView::Cluster => draw_cluster_view(frame, app, area),
        ActiveView::Metrics => draw_metrics_view(frame, app, area),
        ActiveView::KeyValue => draw_kv_view(frame, app, area),
        ActiveView::Vaults => draw_vaults_view(frame, app, area),
        ActiveView::Logs => draw_logs_view(frame, app, area),
        ActiveView::Help => draw_help_view(frame, area),
    }
}

/// Draw cluster overview with node list.
fn draw_cluster_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Node list
    draw_node_list(frame, app, chunks[0]);

    // Right: Selected node details
    draw_node_details(frame, app, chunks[1]);
}

/// Draw the list of nodes.
fn draw_node_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .nodes
        .values()
        .enumerate()
        .map(|(i, node)| {
            let status_icon = match node.status {
                NodeStatus::Healthy => "●",
                NodeStatus::Degraded => "◐",
                NodeStatus::Unhealthy => "○",
                NodeStatus::Unknown => "?",
            };

            let status_color = match node.status {
                NodeStatus::Healthy => Color::Green,
                NodeStatus::Degraded => Color::Yellow,
                NodeStatus::Unhealthy => Color::Red,
                NodeStatus::Unknown => Color::Gray,
            };

            let leader_marker = if node.is_leader { " [L]" } else { "" };

            let content = Line::from(vec![
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::raw(format!("Node {}{}", node.node_id, leader_marker)),
            ]);

            let style = if i == app.selected_node {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = format!(" Nodes ({}) ", app.nodes.len());
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}

/// Draw details for the selected node.
fn draw_node_details(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(node) = app.selected_node_info() {
        let status_str = match node.status {
            NodeStatus::Healthy => "Healthy",
            NodeStatus::Degraded => "Degraded",
            NodeStatus::Unhealthy => "Unhealthy",
            NodeStatus::Unknown => "Unknown",
        };

        let mut lines = vec![
            Line::from(vec![
                Span::raw("Node ID: "),
                Span::styled(
                    node.node_id.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![Span::raw("Status: "), Span::raw(status_str)]),
            Line::from(vec![
                Span::raw("Role: "),
                Span::raw(if node.is_leader { "Leader" } else { "Follower" }),
            ]),
            Line::from(vec![Span::raw("Address: "), Span::raw(&node.http_addr)]),
        ];

        if let Some(term) = node.current_term {
            lines.push(Line::from(vec![
                Span::raw("Term: "),
                Span::raw(term.to_string()),
            ]));
        }

        if let Some(index) = node.last_applied_index {
            lines.push(Line::from(vec![
                Span::raw("Last Applied: "),
                Span::raw(index.to_string()),
            ]));
        }

        if let Some(uptime) = node.uptime_secs {
            let hours = uptime / 3600;
            let mins = (uptime % 3600) / 60;
            let secs = uptime % 60;
            lines.push(Line::from(vec![
                Span::raw("Uptime: "),
                Span::raw(format!("{:02}:{:02}:{:02}", hours, mins, secs)),
            ]));
        }

        lines
    } else {
        vec![Line::from("No node selected")]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Node Details "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw metrics view with cluster-wide stats.
fn draw_metrics_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    // Cluster summary
    if let Some(metrics) = &app.cluster_metrics {
        let summary = vec![
            Line::from(vec![
                Span::raw("Total Nodes: "),
                Span::styled(
                    metrics.node_count.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Leader: "),
                Span::styled(
                    metrics
                        .leader
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::raw("Term: "),
                Span::raw(metrics.term.to_string()),
            ]),
            Line::from(vec![
                Span::raw("Last Applied: "),
                Span::raw(
                    metrics
                        .last_applied_index
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(summary)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Cluster Summary "),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, chunks[0]);
    }

    // Node metrics table
    let header = Row::new(vec!["Node", "Status", "Role", "Term", "Applied"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .nodes
        .values()
        .map(|node| {
            let status = match node.status {
                NodeStatus::Healthy => ("Healthy", Color::Green),
                NodeStatus::Degraded => ("Degraded", Color::Yellow),
                NodeStatus::Unhealthy => ("Unhealthy", Color::Red),
                NodeStatus::Unknown => ("Unknown", Color::Gray),
            };

            Row::new(vec![
                node.node_id.to_string(),
                status.0.to_string(),
                if node.is_leader {
                    "Leader".to_string()
                } else {
                    "Follower".to_string()
                },
                node.current_term
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                node.last_applied_index
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Node Metrics "),
    );

    frame.render_widget(table, chunks[1]);
}

/// Draw key-value operations view.
fn draw_kv_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Instructions
    let instructions = vec![
        Line::from("Press Enter to input commands"),
        Line::from(""),
        Line::from("Commands:"),
        Line::from("  get <key>        - Read a key"),
        Line::from("  set <key> <val>  - Write a key-value pair"),
    ];

    let paragraph = Paragraph::new(instructions)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Key-Value Operations "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, chunks[0]);

    // Results area
    let result_content = if let Some((key, value)) = &app.last_read_result {
        let val_str = value
            .as_ref()
            .map(|v| String::from_utf8_lossy(v).to_string())
            .unwrap_or_else(|| "(not found)".to_string());
        vec![
            Line::from(vec![
                Span::raw("Key: "),
                Span::styled(key, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::raw("Value: "), Span::raw(val_str)]),
        ]
    } else {
        vec![Line::from("No results yet")]
    };

    let results = Paragraph::new(result_content)
        .block(Block::default().borders(Borders::ALL).title(" Results "))
        .wrap(Wrap { trim: true });

    frame.render_widget(results, chunks[1]);
}

/// Draw vaults browser view.
fn draw_vaults_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left panel: vault list or key list depending on mode
    if let Some(vault_name) = &app.active_vault {
        // Viewing vault contents - show keys
        draw_vault_keys_list(frame, app, chunks[0], vault_name);
        draw_vault_key_detail(frame, app, chunks[1]);
    } else {
        // Viewing vault list
        draw_vaults_list(frame, app, chunks[0]);
        draw_vault_info(frame, app, chunks[1]);
    }
}

/// Draw the list of vaults.
fn draw_vaults_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .vaults
        .iter()
        .enumerate()
        .map(|(i, vault)| {
            let style = if i == app.selected_vault {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = Line::from(vec![
                Span::styled(format!(" {} ", vault.name), style),
                Span::styled(
                    format!("({} keys)", vault.key_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Vaults (Enter to browse, r to refresh) "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, area);
}

/// Draw vault info panel.
fn draw_vault_info(frame: &mut Frame, app: &App, area: Rect) {
    let content = if app.vaults.is_empty() {
        vec![
            Line::from("No vaults found."),
            Line::from(""),
            Line::from("Vaults are created when keys are stored with the prefix:"),
            Line::from("  vault:<vault_name>:<key>"),
            Line::from(""),
            Line::from("Press 'r' to refresh."),
        ]
    } else if let Some(vault) = app.vaults.get(app.selected_vault) {
        vec![
            Line::from(vec![
                Span::raw("Vault: "),
                Span::styled(
                    &vault.name,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Keys: "),
                Span::styled(
                    vault.key_count.to_string(),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(""),
            Line::from("Press Enter to browse vault contents."),
            Line::from("Press Backspace/Esc to go back."),
        ]
    } else {
        vec![Line::from("Select a vault")]
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(" Vault Info "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw the list of keys within a vault.
fn draw_vault_keys_list(frame: &mut Frame, app: &App, area: Rect, vault_name: &str) {
    let items: Vec<ListItem> = app
        .vault_keys
        .iter()
        .enumerate()
        .map(|(i, kv)| {
            let style = if i == app.selected_vault_key {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = Line::from(vec![Span::styled(format!(" {} ", kv.key), style)]);
            ListItem::new(content)
        })
        .collect();

    let title = format!(
        " {} ({} keys) - Backspace to go back ",
        vault_name,
        app.vault_keys.len()
    );
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, area);
}

/// Draw vault key detail panel.
fn draw_vault_key_detail(frame: &mut Frame, app: &App, area: Rect) {
    let content = if app.vault_keys.is_empty() {
        vec![Line::from("No keys in this vault.")]
    } else if let Some(kv) = app.vault_keys.get(app.selected_vault_key) {
        vec![
            Line::from(vec![
                Span::raw("Key: "),
                Span::styled(
                    &kv.key,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Value:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )]),
            Line::from(""),
            Line::from(kv.value.clone()),
        ]
    } else {
        vec![Line::from("Select a key")]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Key Details "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw logs view with tui-logger integration.
fn draw_logs_view(frame: &mut Frame, app: &App, area: Rect) {
    // Create tui-logger widget
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

/// Draw help view.
fn draw_help_view(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Aspen TUI - Cluster Management",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  Tab / Shift+Tab  Switch between views"),
        Line::from("  1-5              Jump to view"),
        Line::from("  ?                Show this help"),
        Line::from("  q / Esc          Quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Cluster View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k or Up/Down   Navigate node list"),
        Line::from("  r                Refresh cluster state"),
        Line::from("  i                Initialize cluster"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Key-Value View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  Enter            Enter command mode"),
        Line::from("  Esc              Exit command mode"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Vaults View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k or Up/Down   Navigate vault/key list"),
        Line::from("  Enter            Browse vault contents"),
        Line::from("  Backspace/Esc    Go back to vault list"),
        Line::from("  r                Refresh vaults"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Logs View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  PageUp/PageDown  Scroll logs"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw status bar at bottom.
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = if let Some((msg, _)) = &app.status_message {
        msg.clone()
    } else if app.refreshing {
        "Refreshing...".to_string()
    } else if let Some(last) = app.last_refresh {
        format!("Last refresh: {:.1}s ago", last.elapsed().as_secs_f64())
    } else {
        "Press 'r' to refresh".to_string()
    };

    let mode = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDITING",
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(12)])
        .split(area);

    let status_widget = Paragraph::new(status)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));

    let mode_widget = Paragraph::new(mode)
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(if app.input_mode == InputMode::Editing {
                    Color::Yellow
                } else {
                    Color::Green
                }),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status_widget, chunks[0]);
    frame.render_widget(mode_widget, chunks[1]);
}

/// Draw input popup for command entry.
fn draw_input_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());

    // Clear the area
    frame.render_widget(Clear, area);

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Enter command (get <key> | set <key> <value>) "),
        );

    frame.render_widget(input, area);

    // Set cursor position
    frame.set_cursor(area.x + app.input_buffer.len() as u16 + 1, area.y + 1);
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
