//! Cluster and metrics view rendering.

use ratatui::Frame;
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
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Table;
use ratatui::widgets::Wrap;

use crate::app::App;
use crate::types::NodeStatus;

/// Draw cluster overview with node list.
pub(super) fn draw_cluster_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_node_list(frame, app, chunks[0]);
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
pub(super) fn draw_metrics_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    draw_cluster_summary(frame, app, chunks[0]);
    draw_node_metrics_table(frame, app, chunks[1]);
}

/// Draw cluster summary section of metrics view.
fn draw_cluster_summary(frame: &mut Frame, app: &App, area: Rect) {
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

        frame.render_widget(paragraph, area);
    }
}

/// Draw node metrics table section.
fn draw_node_metrics_table(frame: &mut Frame, app: &App, area: Rect) {
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

    let table = Table::new(rows, [
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Length(10),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Node Metrics "));

    frame.render_widget(table, area);
}
