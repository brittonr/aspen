//! Cluster and metrics view rendering.

use std::collections::VecDeque;

use ratatui::Frame;
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
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Sparkline;
use ratatui::widgets::Table;
use ratatui::widgets::Wrap;

use crate::app::App;
use crate::types::NodeStatus;

fn bordered_block<'a, T>(title: T) -> Block<'a>
where T: Into<Line<'a>> {
    Block::new().borders(Borders::ALL).title(title)
}

fn bold_style() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

fn selected_row_style() -> Style {
    Style::new().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
}

fn status_icon(status: NodeStatus) -> &'static str {
    match status {
        NodeStatus::Healthy => "●",
        NodeStatus::Degraded => "◐",
        NodeStatus::Unhealthy => "○",
        NodeStatus::Unknown => "?",
    }
}

fn status_color(status: NodeStatus) -> Color {
    match status {
        NodeStatus::Healthy => Color::Green,
        NodeStatus::Degraded => Color::Yellow,
        NodeStatus::Unhealthy => Color::Red,
        NodeStatus::Unknown => Color::Gray,
    }
}

fn status_label(status: NodeStatus) -> &'static str {
    match status {
        NodeStatus::Healthy => "Healthy",
        NodeStatus::Degraded => "Degraded",
        NodeStatus::Unhealthy => "Unhealthy",
        NodeStatus::Unknown => "Unknown",
    }
}

fn role_label(is_leader: bool) -> &'static str {
    if is_leader { "Leader" } else { "Follower" }
}

struct EmptyPanel<'a> {
    title: &'a str,
    message: &'a str,
}

fn render_empty_panel(frame: &mut Frame, area: Rect, panel: EmptyPanel<'_>) {
    let paragraph = Paragraph::new(panel.message)
        .block(bordered_block(panel.title))
        .style(Style::new().fg(Color::DarkGray));
    frame.render_widget(paragraph, area);
}

fn render_latency_sparkline(frame: &mut Frame, area: Rect, title: &'static str, data: &VecDeque<u64>) {
    if data.is_empty() {
        render_empty_panel(frame, area, EmptyPanel {
            title,
            message: "No metric data",
        });
        return;
    }

    let sparkline_data: Vec<u64> = data.iter().copied().collect();
    debug_assert!(!sparkline_data.is_empty());
    debug_assert!(sparkline_data.len() == data.len());
    let sparkline = Sparkline::default()
        .block(bordered_block(title))
        .data(&sparkline_data)
        .style(Style::new().fg(Color::Cyan));
    frame.render_widget(sparkline, area);
}

/// Draw cluster overview with node list.
pub(super) fn draw_cluster_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    debug_assert!(chunks.len() == 2);

    draw_node_list(frame, app, chunks[0]);
    draw_node_details(frame, app, chunks[1]);
}

/// Draw the list of nodes.
fn draw_node_list(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let items: Vec<ListItem> = app
        .nodes
        .values()
        .enumerate()
        .map(|(index, node)| {
            let leader_marker = if node.is_leader { " [L]" } else { "" };
            let content = Line::from(vec![
                Span::styled(format!("{} ", status_icon(node.status)), Style::new().fg(status_color(node.status))),
                Span::raw(format!("Node {}{}", node.node_id, leader_marker)),
            ]);

            let item_style = if index == app.selected_node {
                selected_row_style()
            } else {
                Style::new()
            };

            ListItem::new(content).style(item_style)
        })
        .collect();

    debug_assert!(items.len() == app.nodes.len());
    let title = format!(" Nodes ({}) ", app.nodes.len());
    let list = List::new(items).block(bordered_block(title));

    frame.render_widget(list, area);
}

/// Draw details for the selected node.
fn draw_node_details(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let content = if let Some(node) = app.selected_node_info() {
        let mut lines = vec![
            Line::from(vec![
                Span::raw("Node ID: "),
                Span::styled(node.node_id.to_string(), bold_style()),
            ]),
            Line::from(vec![Span::raw("Status: "), Span::raw(status_label(node.status))]),
            Line::from(vec![Span::raw("Role: "), Span::raw(role_label(node.is_leader))]),
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
            let minutes = (uptime % 3600) / 60;
            let seconds = uptime % 60;
            lines.push(Line::from(vec![
                Span::raw("Uptime: "),
                Span::raw(format!("{:02}:{:02}:{:02}", hours, minutes, seconds)),
            ]));
        }

        lines
    } else {
        vec![Line::from("No node selected")]
    };

    debug_assert!(!content.is_empty());
    let paragraph = Paragraph::new(content).block(bordered_block(" Node Details ")).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw metrics view with cluster-wide stats.
pub(super) fn draw_metrics_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Length(6), Constraint::Min(0)]).split(area);
    debug_assert!(chunks.len() == 3);

    draw_cluster_summary(frame, app, chunks[0]);
    draw_latency_sparklines(frame, app, chunks[1]);

    let bottom = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[2]);
    debug_assert!(bottom.len() == 2);

    draw_connection_health(frame, app, bottom[0]);
    draw_active_alerts(frame, app, bottom[1]);
}

/// Draw cluster summary section of metrics view.
fn draw_cluster_summary(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(metrics) = &app.cluster_metrics {
        let summary = vec![
            Line::from(vec![
                Span::raw("Total Nodes: "),
                Span::styled(metrics.node_count.to_string(), bold_style()),
            ]),
            Line::from(vec![
                Span::raw("Leader: "),
                Span::styled(
                    metrics.leader.map(|id| id.to_string()).unwrap_or_else(|| "None".to_string()),
                    Style::new().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![Span::raw("Term: "), Span::raw(metrics.term.to_string())]),
            Line::from(vec![
                Span::raw("Last Applied: "),
                Span::raw(
                    metrics.last_applied_index.map(|index| index.to_string()).unwrap_or_else(|| "N/A".to_string()),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(summary).block(bordered_block(" Cluster Summary ")).wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}

/// Draw node metrics table section.
fn draw_node_metrics_table(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let header = Row::new(vec!["Node", "Status", "Role", "Term", "Applied"]).style(bold_style()).bottom_margin(1);

    let rows: Vec<Row> = app
        .nodes
        .values()
        .map(|node| {
            Row::new(vec![
                node.node_id.to_string(),
                status_label(node.status).to_string(),
                role_label(node.is_leader).to_string(),
                node.current_term.map(|term| term.to_string()).unwrap_or_else(|| "-".to_string()),
                node.last_applied_index.map(|index| index.to_string()).unwrap_or_else(|| "-".to_string()),
            ])
        })
        .collect();

    debug_assert!(rows.len() == app.nodes.len());
    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(10),
    ])
    .header(header)
    .block(bordered_block(" Node Metrics "));

    frame.render_widget(table, area);
}

/// Draw latency sparklines for Read, Write, and Raft commit.
fn draw_latency_sparklines(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);
    debug_assert!(columns.len() == 3);
    debug_assert!(area.width > 0);

    render_latency_sparkline(frame, columns[0], " Read Latency ", &app.read_latency_sparkline);
    render_latency_sparkline(frame, columns[1], " Write Latency ", &app.write_latency_sparkline);
    render_latency_sparkline(frame, columns[2], " Raft Commit ", &app.raft_latency_sparkline);
}

/// Draw connection health panel.
fn draw_connection_health(frame: &mut Frame, app: &App, area: Rect) {
    match &app.network_metrics {
        Some(metrics) => {
            let lines = vec![
                Line::from(vec![
                    Span::styled(format!("  {} healthy", metrics.healthy_connections), Style::new().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled(format!("{} degraded", metrics.degraded_connections), Style::new().fg(Color::Yellow)),
                    Span::raw("  "),
                    Span::styled(format!("{} failed", metrics.failed_connections), Style::new().fg(Color::Red)),
                ]),
                Line::from(vec![
                    Span::raw(format!("  Streams: {}  ", metrics.total_active_streams)),
                    Span::raw(format!("Raft: {}  Bulk: {}", metrics.raft_streams_opened, metrics.bulk_streams_opened)),
                ]),
                Line::from(format!(
                    "  ReadIndex retries: {} (ok: {})",
                    metrics.read_index_retry_count, metrics.read_index_retry_success_count
                )),
            ];

            let paragraph = Paragraph::new(lines).block(bordered_block(" Connection Health "));
            frame.render_widget(paragraph, area);
        }
        None => render_empty_panel(frame, area, EmptyPanel {
            title: " Connection Health ",
            message: "No connections",
        }),
    }
}

/// Draw active alerts panel.
fn draw_active_alerts(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    if app.active_alerts.is_empty() {
        render_empty_panel(frame, area, EmptyPanel {
            title: " Alerts ",
            message: "No active alerts",
        });
        return;
    }

    let items: Vec<ListItem> = app
        .active_alerts
        .iter()
        .map(|alert| {
            let severity_color = match alert.rule.severity {
                aspen_client_api::AlertSeverity::Critical => Color::Red,
                aspen_client_api::AlertSeverity::Warning => Color::Yellow,
                aspen_client_api::AlertSeverity::Info => Color::Cyan,
            };
            let status_code = match alert.state.as_ref().map(|state| &state.status) {
                Some(aspen_client_api::AlertStatus::Pending) => "PEND",
                Some(aspen_client_api::AlertStatus::Firing) => "FIRE",
                _ => "--",
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", status_code), Style::new().fg(severity_color)),
                Span::raw(&alert.rule.name),
            ]))
        })
        .collect();

    debug_assert!(!items.is_empty());
    let title = format!(" Alerts ({}) ", app.active_alerts.len());
    let list = List::new(items).block(bordered_block(title));
    frame.render_widget(list, area);
}
