//! UI rendering for the Aspen TUI.
//!
//! Implements the View layer of The Elm Architecture (TEA).

use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Table;
use ratatui::widgets::Tabs;
use ratatui::widgets::Wrap;

use crate::app::ActiveView;
use crate::app::App;
use crate::app::InputMode;
use crate::types::NodeStatus;
use crate::types::SqlConsistency;

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

    // Draw SQL input popup if in SQL editing mode
    if app.input_mode == InputMode::SqlEditing {
        draw_sql_input_popup(frame, app);
    }
}

/// Draw the header with tab navigation.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = [
        ActiveView::Cluster,
        ActiveView::Metrics,
        ActiveView::KeyValue,
        ActiveView::Vaults,
        ActiveView::Sql,
        ActiveView::Logs,
        ActiveView::Jobs,
        ActiveView::Workers,
        ActiveView::Help,
    ]
    .iter()
    .map(|v| {
        let style = if *v == app.active_view {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        Line::from(Span::styled(format!(" {} ", v.name()), style))
    })
    .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Aspen Cluster Manager "))
        .select(match app.active_view {
            ActiveView::Cluster => 0,
            ActiveView::Metrics => 1,
            ActiveView::KeyValue => 2,
            ActiveView::Vaults => 3,
            ActiveView::Sql => 4,
            ActiveView::Logs => 5,
            ActiveView::Jobs => 6,
            ActiveView::Workers => 7,
            ActiveView::Help => 8,
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
        ActiveView::Sql => draw_sql_view(frame, app, area),
        ActiveView::Logs => draw_logs_view(frame, app, area),
        ActiveView::Jobs => draw_jobs_view(frame, app, area),
        ActiveView::Workers => draw_workers_view(frame, app, area),
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
                Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
                Span::raw(format!("Node {}{}", node.node_id, leader_marker)),
            ]);

            let style = if i == app.selected_node {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
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
                Span::styled(node.node_id.to_string(), Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::raw("Status: "), Span::raw(status_str)]),
            Line::from(vec![
                Span::raw("Role: "),
                Span::raw(if node.is_leader { "Leader" } else { "Follower" }),
            ]),
            Line::from(vec![Span::raw("Address: "), Span::raw(&node.addr)]),
        ];

        if let Some(term) = node.current_term {
            lines.push(Line::from(vec![Span::raw("Term: "), Span::raw(term.to_string())]));
        }

        if let Some(index) = node.last_applied_index {
            lines.push(Line::from(vec![Span::raw("Last Applied: "), Span::raw(index.to_string())]));
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
        .block(Block::default().borders(Borders::ALL).title(" Node Details "))
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
                Span::styled(metrics.node_count.to_string(), Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("Leader: "),
                Span::styled(
                    metrics.leader.map(|id| id.to_string()).unwrap_or_else(|| "None".to_string()),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![Span::raw("Term: "), Span::raw(metrics.term.to_string())]),
            Line::from(vec![
                Span::raw("Last Applied: "),
                Span::raw(metrics.last_applied_index.map(|i| i.to_string()).unwrap_or_else(|| "N/A".to_string())),
            ]),
        ];

        let paragraph = Paragraph::new(summary)
            .block(Block::default().borders(Borders::ALL).title(" Cluster Summary "))
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
                node.current_term.map(|t| t.to_string()).unwrap_or_else(|| "-".to_string()),
                node.last_applied_index.map(|i| i.to_string()).unwrap_or_else(|| "-".to_string()),
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
    .block(Block::default().borders(Borders::ALL).title(" Node Metrics "));

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
        .block(Block::default().borders(Borders::ALL).title(" Key-Value Operations "))
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
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = Line::from(vec![
                Span::styled(format!(" {} ", vault.name), style),
                Span::styled(format!("({} keys)", vault.key_count), Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Vaults (Enter to browse, r to refresh) "))
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
                Span::styled(&vault.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Keys: "),
                Span::styled(vault.key_count.to_string(), Style::default().fg(Color::Cyan)),
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
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = Line::from(vec![Span::styled(format!(" {} ", kv.key), style)]);
            ListItem::new(content)
        })
        .collect();

    let title = format!(" {} ({} keys) - Backspace to go back ", vault_name, app.vault_keys.len());
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
                Span::styled(&kv.key, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
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
        .block(Block::default().borders(Borders::ALL).title(" Key Details "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw SQL query view.
fn draw_sql_view(frame: &mut Frame, app: &App, area: Rect) {
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

/// Draw SQL results table.
fn draw_sql_results(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(result) = &app.sql_state.last_result {
        if !result.success {
            // Display error
            let error_msg = result.error.as_deref().unwrap_or("Unknown error");
            let paragraph = Paragraph::new(error_msg)
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title(" Error "));
            frame.render_widget(paragraph, area);
            return;
        }

        if result.rows.is_empty() {
            let paragraph =
                Paragraph::new("No results").block(Block::default().borders(Borders::ALL).title(" Results "));
            frame.render_widget(paragraph, area);
            return;
        }

        // Build header row
        let header_cells: Vec<&str> = result.columns.iter().map(|c| c.as_str()).collect();
        let header = Row::new(header_cells)
            .style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))
            .bottom_margin(1);

        // Build data rows with selection highlight
        let rows: Vec<Row> = result
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let cells: Vec<String> = row
                    .iter()
                    .map(|cell| {
                        // Truncate long values
                        if cell.len() > 40 {
                            format!("{}...", &cell[..37])
                        } else {
                            cell.clone()
                        }
                    })
                    .collect();

                let style = if i == app.sql_state.selected_row {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(cells).style(style)
            })
            .collect();

        // Calculate column widths
        let widths: Vec<Constraint> =
            result.column_widths.iter().map(|&w| Constraint::Length(w.min(40) as u16)).collect();

        let title = format!(
            " Results ({} rows, {} columns) [j/k navigate, h/l scroll] ",
            result.rows.len(),
            result.columns.len()
        );

        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(title))
            .column_spacing(1)
            .highlight_style(Style::default().bg(Color::DarkGray));

        frame.render_widget(table, area);
    } else {
        // No results yet
        let paragraph = Paragraph::new("Execute a query to see results")
            .block(Block::default().borders(Borders::ALL).title(" Results "));
        frame.render_widget(paragraph, area);
    }
}

/// Draw SQL info bar with execution stats.
fn draw_sql_info_bar(frame: &mut Frame, app: &App, area: Rect) {
    let info = if let Some(result) = &app.sql_state.last_result {
        if result.success {
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

/// Draw SQL input popup for query editing.
fn draw_sql_input_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 40, frame.area());

    // Clear the area
    frame.render_widget(Clear, area);

    let title = format!(
        " Edit SQL Query (Enter to save, Esc to cancel, Up/Down for history [{}/{}]) ",
        if app.sql_state.history_browsing {
            app.sql_state.history_index + 1
        } else {
            app.sql_state.history.len()
        },
        app.sql_state.history.len()
    );

    let input = Paragraph::new(app.sql_state.query_buffer.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(input, area);

    // Set cursor position
    let cursor_x = area.x + (app.sql_state.query_buffer.len() as u16 % (area.width - 2)) + 1;
    let cursor_y = area.y + (app.sql_state.query_buffer.len() as u16 / (area.width - 2)) + 1;
    frame.set_cursor_position((cursor_x, cursor_y.min(area.y + area.height - 2)));
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

/// Draw jobs view with queue statistics and job list.
fn draw_jobs_view(frame: &mut Frame, app: &App, area: Rect) {
    // Layout: stats bar at top, job list below (optionally with details panel)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // Draw queue stats bar
    draw_queue_stats(frame, app, main_chunks[0]);

    // Split content area for list and optional details
    if app.jobs_state.show_details {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_chunks[1]);

        draw_job_list(frame, app, content_chunks[0]);
        draw_job_details(frame, app, content_chunks[1]);
    } else {
        draw_job_list(frame, app, main_chunks[1]);
    }
}

/// Draw queue statistics bar.
fn draw_queue_stats(frame: &mut Frame, app: &App, area: Rect) {
    let stats = &app.jobs_state.queue_stats;

    let stats_text = vec![
        Line::from(vec![
            Span::styled("Queue: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} pending", stats.pending_count), Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("{} scheduled", stats.scheduled_count), Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled(format!("{} running", stats.running_count), Style::default().fg(Color::Blue)),
            Span::raw(" | "),
            Span::styled(format!("{} completed", stats.completed_count), Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(format!("{} failed", stats.failed_count), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled("Filters: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "Status: {} | Priority: {}",
                app.jobs_state.status_filter.as_str(),
                app.jobs_state.priority_filter.as_str()
            )),
        ]),
        Line::from(vec![
            Span::styled("Keys: ", Style::default().fg(Color::DarkGray)),
            Span::raw("s=status filter | p=priority filter | d=details | x=cancel | r=refresh"),
        ]),
    ];

    let paragraph =
        Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(" Queue Statistics "));

    frame.render_widget(paragraph, area);
}

/// Draw the job list.
fn draw_job_list(frame: &mut Frame, app: &App, area: Rect) {
    let jobs = &app.jobs_state.jobs;

    let items: Vec<ListItem> = jobs
        .iter()
        .enumerate()
        .map(|(i, job)| {
            let status_color = match job.status.as_str() {
                "pending" => Color::Yellow,
                "scheduled" => Color::Cyan,
                "running" => Color::Blue,
                "completed" => Color::Green,
                "failed" => Color::Red,
                "cancelled" => Color::DarkGray,
                _ => Color::White,
            };

            let priority_symbol = match job.priority {
                0 => "L",
                1 => "N",
                2 => "H",
                3 => "!",
                _ => "?",
            };

            let style = if i == app.jobs_state.selected_job {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let progress_str = if job.status == "running" {
                format!(" {}%", job.progress)
            } else {
                String::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", priority_symbol), Style::default().fg(Color::Cyan)),
                Span::styled(&job.job_id[..job.job_id.len().min(8)], style),
                Span::raw(" "),
                Span::styled(&job.job_type, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(&job.status, Style::default().fg(status_color)),
                Span::styled(progress_str, Style::default().fg(Color::Blue)),
            ]))
        })
        .collect();

    let title = format!(" Jobs ({}) ", jobs.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

/// Draw job details panel.
fn draw_job_details(frame: &mut Frame, app: &App, area: Rect) {
    let details_text = if let Some(job) = app.jobs_state.jobs.get(app.jobs_state.selected_job) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Job ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&job.job_id),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&job.job_type),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&job.status),
            ]),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(job.priority_str()),
            ]),
            Line::from(vec![
                Span::styled("Progress: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}%", job.progress)),
            ]),
            Line::from(vec![
                Span::styled("Attempts: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", job.attempts)),
            ]),
            Line::from(vec![
                Span::styled("Submitted: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&job.submitted_at),
            ]),
        ];

        if let Some(ref started) = job.started_at {
            lines.push(Line::from(vec![
                Span::styled("Started: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(started),
            ]));
        }

        if let Some(ref completed) = job.completed_at {
            lines.push(Line::from(vec![
                Span::styled("Completed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(completed),
            ]));
        }

        if let Some(ref worker_id) = job.worker_id {
            lines.push(Line::from(vec![
                Span::styled("Worker: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(worker_id),
            ]));
        }

        if !job.tags.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(job.tags.join(", ")),
            ]));
        }

        if let Some(ref msg) = job.progress_message {
            lines.push(Line::from(vec![
                Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(msg),
            ]));
        }

        if let Some(ref err) = job.error_message {
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }

        lines
    } else {
        vec![Line::from("No job selected")]
    };

    let paragraph = Paragraph::new(details_text)
        .block(Block::default().borders(Borders::ALL).title(" Job Details "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw workers view with pool summary and worker list.
fn draw_workers_view(frame: &mut Frame, app: &App, area: Rect) {
    // Layout: stats bar at top, worker list below (optionally with details panel)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    // Draw pool summary
    draw_worker_pool_summary(frame, app, main_chunks[0]);

    // Split content area for list and optional details
    if app.workers_state.show_details {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_chunks[1]);

        draw_worker_list(frame, app, content_chunks[0]);
        draw_worker_details(frame, app, content_chunks[1]);
    } else {
        draw_worker_list(frame, app, main_chunks[1]);
    }
}

/// Draw worker pool summary.
fn draw_worker_pool_summary(frame: &mut Frame, app: &App, area: Rect) {
    let pool = &app.workers_state.pool_info;

    let stats_text = vec![
        Line::from(vec![
            Span::styled("Workers: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} total", pool.total_workers), Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled(format!("{} idle", pool.idle_workers), Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(format!("{} busy", pool.busy_workers), Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("{} offline", pool.offline_workers), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled("Capacity: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} / {} used", pool.used_capacity, pool.total_capacity)),
        ]),
        Line::from(vec![
            Span::styled("Keys: ", Style::default().fg(Color::DarkGray)),
            Span::raw("j/k=navigate | d=details | r=refresh"),
        ]),
    ];

    let paragraph = Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(" Worker Pool "));

    frame.render_widget(paragraph, area);
}

/// Draw the worker list.
fn draw_worker_list(frame: &mut Frame, app: &App, area: Rect) {
    let workers = &app.workers_state.pool_info.workers;

    let items: Vec<ListItem> = workers
        .iter()
        .enumerate()
        .map(|(i, worker)| {
            let status_color = match worker.status.as_str() {
                "idle" => Color::Green,
                "busy" => Color::Yellow,
                "offline" => Color::Red,
                _ => Color::White,
            };

            let style = if i == app.workers_state.selected_worker {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(&worker.worker_id[..worker.worker_id.len().min(12)], style),
                Span::raw(" "),
                Span::styled(&worker.status, Style::default().fg(status_color)),
                Span::raw(format!(" ({}/{})", worker.active_jobs, worker.capacity)),
                Span::raw(format!(" [{}]", worker.capabilities.join(","))),
            ]))
        })
        .collect();

    let title = format!(" Workers ({}) ", workers.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

/// Draw worker details panel.
fn draw_worker_details(frame: &mut Frame, app: &App, area: Rect) {
    let details_text = if let Some(worker) = app.workers_state.pool_info.workers.get(app.workers_state.selected_worker)
    {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Worker ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&worker.worker_id),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&worker.status),
            ]),
            Line::from(vec![
                Span::styled("Capacity: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{} (using {})", worker.capacity, worker.active_jobs)),
            ]),
            Line::from(vec![
                Span::styled("Capabilities: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(worker.capabilities.join(", ")),
            ]),
            Line::from(vec![
                Span::styled("Last Heartbeat: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&worker.last_heartbeat),
            ]),
            Line::from(vec![
                Span::styled("Total Processed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", worker.total_processed)),
            ]),
            Line::from(vec![
                Span::styled("Total Failed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{}", worker.total_failed),
                    Style::default().fg(if worker.total_failed > 0 {
                        Color::Red
                    } else {
                        Color::White
                    }),
                ),
            ]),
        ];

        if !worker.active_job_ids.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "Active Jobs:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for job_id in &worker.active_job_ids {
                lines.push(Line::from(vec![Span::raw("  - "), Span::raw(job_id)]));
            }
        }

        lines
    } else {
        vec![Line::from("No worker selected")]
    };

    let paragraph = Paragraph::new(details_text)
        .block(Block::default().borders(Borders::ALL).title(" Worker Details "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw help view.
fn draw_help_view(frame: &mut Frame, area: Rect) {
    let help_text = build_help_content();

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title(" Help "))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn build_help_content() -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.extend(build_header_section());
    lines.extend(build_navigation_section());
    lines.extend(build_cluster_section());
    lines.extend(build_keyvalue_section());
    lines.extend(build_vaults_section());
    lines.extend(build_sql_section());
    lines.extend(build_logs_section());
    lines.extend(build_jobs_section());
    lines.extend(build_workers_section());

    lines
}

fn build_header_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Aspen TUI - Cluster Management",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ]
}

fn build_navigation_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  Tab / Shift+Tab  Switch between views"),
        Line::from(
            "  1-8              Jump to view (1=Cluster, 2=Metrics, 3=KV, 4=Vaults, 5=SQL, 6=Logs, 7=Jobs, 8=Workers)",
        ),
        Line::from("  ?                Show this help"),
        Line::from("  q / Esc          Quit"),
        Line::from(""),
    ]
}

fn build_cluster_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Cluster View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k or Up/Down   Navigate node list"),
        Line::from("  r                Refresh cluster state"),
        Line::from("  i                Initialize cluster"),
        Line::from(""),
    ]
}

fn build_keyvalue_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Key-Value View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  Enter            Enter command mode"),
        Line::from("  Esc              Exit command mode"),
        Line::from(""),
    ]
}

fn build_vaults_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Vaults View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k or Up/Down   Navigate vault/key list"),
        Line::from("  Enter            Browse vault contents"),
        Line::from("  Backspace/Esc    Go back to vault list"),
        Line::from("  r                Refresh vaults"),
        Line::from(""),
    ]
}

fn build_sql_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "SQL View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  Enter            Edit query"),
        Line::from("  e                Execute query"),
        Line::from("  c                Toggle consistency (linearizable/stale)"),
        Line::from("  j/k              Navigate result rows"),
        Line::from("  h/l              Scroll columns"),
        Line::from("  Up/Down          Navigate query history (in edit mode)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "SQL KV Table Columns",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  key, value, version, create_revision, mod_revision,"),
        Line::from("  expires_at_ms, lease_id"),
        Line::from("  Example: SELECT key, value FROM kv WHERE key LIKE 'vault:%'"),
        Line::from(""),
    ]
}

fn build_logs_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Logs View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  PageUp/PageDown  Scroll logs"),
        Line::from(""),
    ]
}

fn build_jobs_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Jobs View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k              Navigate job list"),
        Line::from("  s                Cycle status filter (All/Pending/Scheduled/Running/Completed/Failed/Cancelled)"),
        Line::from("  p                Cycle priority filter (All/Low/Normal/High/Critical)"),
        Line::from("  d                Toggle details panel"),
        Line::from("  x                Cancel selected job"),
        Line::from("  r                Refresh job list"),
        Line::from(""),
    ]
}

fn build_workers_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "Workers View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k              Navigate worker list"),
        Line::from("  d                Toggle details panel"),
        Line::from("  r                Refresh worker status"),
        Line::from(""),
    ]
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
        InputMode::SqlEditing => "SQL EDIT",
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(12)])
        .split(area);

    let status_widget = Paragraph::new(status)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));

    let mode_bg = match app.input_mode {
        InputMode::Normal => Color::Green,
        InputMode::Editing => Color::Yellow,
        InputMode::SqlEditing => Color::Cyan,
    };

    let mode_widget = Paragraph::new(mode)
        .style(Style::default().fg(Color::Black).bg(mode_bg))
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
        .block(Block::default().borders(Borders::ALL).title(" Enter command (get <key> | set <key> <value>) "));

    frame.render_widget(input, area);

    // Set cursor position
    frame.set_cursor_position((area.x + app.input_buffer.len() as u16 + 1, area.y + 1));
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
