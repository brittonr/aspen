//! CI pipeline view rendering.

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
use ratatui::widgets::Wrap;

use super::common::ci_status_color;
use super::common::format_timestamp_ms;
use crate::app::App;

/// Draw CI pipeline view with run list and optional details.
pub(super) fn draw_ci_view(frame: &mut Frame, app: &App, area: Rect) {
    if app.ci_state.log_stream.is_visible {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        draw_ci_log_header(frame, app, main_chunks[0]);
        draw_ci_log_viewer(frame, app, main_chunks[1]);
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(area);

        draw_ci_summary(frame, app, main_chunks[0]);

        if app.ci_state.show_details {
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);

            draw_ci_run_list(frame, app, content_chunks[0]);
            draw_ci_run_details(frame, app, content_chunks[1]);
        } else {
            draw_ci_run_list(frame, app, main_chunks[1]);
        }
    }
}

/// Draw CI summary bar with status counts.
fn draw_ci_summary(frame: &mut Frame, app: &App, area: Rect) {
    let runs = &app.ci_state.runs;

    let pending = runs.iter().filter(|r| r.status == "pending").count();
    let running = runs.iter().filter(|r| r.status == "running").count();
    let success = runs.iter().filter(|r| r.status == "success").count();
    let failed = runs.iter().filter(|r| r.status == "failed").count();

    let stats_text = vec![
        Line::from(vec![
            Span::styled("Pipelines: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} total", runs.len()), Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled(format!("{} pending", pending), Style::default().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("{} running", running), Style::default().fg(Color::Blue)),
            Span::raw(" | "),
            Span::styled(format!("{} success", success), Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(format!("{} failed", failed), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled("Filter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.ci_state.status_filter.as_str()),
            if let Some(ref repo) = app.ci_state.repo_filter {
                Span::raw(format!(" | Repo: {}", repo))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("Keys: ", Style::default().fg(Color::DarkGray)),
            Span::raw("j/k=navigate | s=status | d=details | l=logs | x=cancel | t=trigger | r=refresh"),
        ]),
    ];

    let paragraph = Paragraph::new(stats_text).block(Block::default().borders(Borders::ALL).title(" CI Pipelines "));

    frame.render_widget(paragraph, area);
}

/// Draw the CI run list.
fn draw_ci_run_list(frame: &mut Frame, app: &App, area: Rect) {
    let runs = &app.ci_state.runs;

    let items: Vec<ListItem> = runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let status_color = ci_status_color(&run.status);

            let style = if i as u32 == app.ci_state.selected_run {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let created_str = format_timestamp_ms(run.created_at_ms);

            ListItem::new(Line::from(vec![
                Span::styled(&run.run_id[..run.run_id.len().min(8)], style),
                Span::raw(" "),
                Span::styled(&run.ref_name, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(&run.status, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(created_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let title = format!(" Pipeline Runs ({}) ", runs.len());

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}

/// Draw CI run details panel.
fn draw_ci_run_details(frame: &mut Frame, app: &App, area: Rect) {
    let details_text = if let Some(ref detail) = app.ci_state.selected_detail {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Run ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&detail.run_id),
            ]),
            Line::from(vec![
                Span::styled("Repo: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&detail.repo_id),
            ]),
            Line::from(vec![
                Span::styled("Ref: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&detail.ref_name, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Commit: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&detail.commit_hash[..detail.commit_hash.len().min(12)]),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&detail.status, Style::default().fg(ci_status_color(&detail.status))),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format_timestamp_ms(detail.created_at_ms)),
            ]),
        ];

        if let Some(completed) = detail.completed_at_ms {
            lines.push(Line::from(vec![
                Span::styled("Completed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format_timestamp_ms(completed)),
            ]));
        }

        if !detail.stages.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled("Stages:", Style::default().add_modifier(Modifier::BOLD))]));

            for stage in &detail.stages {
                let status_color = ci_status_color(&stage.status);
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(&stage.name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(": "),
                    Span::styled(&stage.status, Style::default().fg(status_color)),
                ]));

                for job in &stage.jobs {
                    let job_color = ci_status_color(&job.status);
                    lines.push(Line::from(vec![
                        Span::raw("    - "),
                        Span::raw(&job.name),
                        Span::raw(": "),
                        Span::styled(&job.status, Style::default().fg(job_color)),
                    ]));
                }
            }
        }

        if let Some(ref err) = detail.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }

        lines
    } else {
        vec![Line::from("Select a run and press 'd' to view details")]
    };

    let paragraph = Paragraph::new(details_text)
        .block(Block::default().borders(Borders::ALL).title(" Run Details "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw header for CI log viewer.
fn draw_ci_log_header(frame: &mut Frame, app: &App, area: Rect) {
    let log_state = &app.ci_state.log_stream;

    let status_indicator = if log_state.is_streaming {
        Span::styled(" LIVE ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" COMPLETE ", Style::default().fg(Color::Black).bg(Color::DarkGray))
    };

    let job_info = if let (Some(run_id), Some(job_id)) = (&log_state.run_id, &log_state.job_id) {
        format!("Run: {} | Job: {}", &run_id[..run_id.len().min(8)], job_id)
    } else {
        "No job selected".to_string()
    };

    let auto_scroll = if log_state.auto_scroll {
        Span::styled("AUTO", Style::default().fg(Color::Green))
    } else {
        Span::styled("MANUAL", Style::default().fg(Color::Yellow))
    };

    let header_text = vec![Line::from(vec![
        status_indicator,
        Span::raw(" "),
        Span::raw(job_info),
        Span::raw(" | Scroll: "),
        auto_scroll,
        Span::raw(" | "),
        Span::styled("Keys: ", Style::default().fg(Color::DarkGray)),
        Span::raw("j/k=scroll f=follow G=end g=start q=close"),
    ])];

    let paragraph = Paragraph::new(header_text).block(Block::default().borders(Borders::ALL).title(" CI Job Logs "));

    frame.render_widget(paragraph, area);
}

/// Draw CI job log viewer panel.
fn draw_ci_log_viewer(frame: &mut Frame, app: &App, area: Rect) {
    let log_state = &app.ci_state.log_stream;

    if let Some(ref error) = log_state.error {
        let error_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(error, Style::default().fg(Color::Red)),
            ]),
            Line::from(""),
            Line::from("Press 'q' to close this view"),
        ];

        let paragraph = Paragraph::new(error_text)
            .block(Block::default().borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
        return;
    }

    let visible_height = area.height.saturating_sub(2) as usize;

    let total_lines = log_state.lines.len();
    let start_line = if log_state.auto_scroll {
        total_lines.saturating_sub(visible_height)
    } else {
        (log_state.scroll_position as usize).saturating_sub(visible_height / 2)
    };

    let end_line = (start_line + visible_height).min(total_lines);

    let log_lines: Vec<Line> = log_state.lines[start_line..end_line]
        .iter()
        .map(|line| {
            let style = match line.stream.as_str() {
                "stderr" => Style::default().fg(Color::Red),
                "stdout" => Style::default().fg(Color::White),
                "build" => Style::default().fg(Color::Cyan),
                _ => Style::default().fg(Color::DarkGray),
            };
            Line::from(Span::styled(&line.content, style))
        })
        .collect();

    let display_lines = if log_lines.is_empty() {
        if log_state.is_streaming {
            vec![
                Line::from(""),
                Line::from(Span::styled("Waiting for logs...", Style::default().fg(Color::DarkGray))),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled("No logs available", Style::default().fg(Color::DarkGray))),
            ]
        }
    } else {
        log_lines
    };

    let scroll_info = if total_lines > 0 {
        format!(" Lines {}-{} of {} ", start_line + 1, end_line, total_lines)
    } else {
        " Empty ".to_string()
    };

    let paragraph = Paragraph::new(display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(scroll_info, Style::default().fg(Color::DarkGray))),
    );

    frame.render_widget(paragraph, area);
}
