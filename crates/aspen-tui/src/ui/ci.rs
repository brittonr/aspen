//! CI pipeline view rendering.

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
use ratatui::widgets::Wrap;

use super::common::ci_status_color;
use super::common::format_timestamp_ms;
use crate::app::App;
use crate::types::CiPipelineDetail;

fn is_selected_row(index: usize, selected: u32) -> bool {
    u32::try_from(index).unwrap_or(u32::MAX) == selected
}

/// Draw CI pipeline view with run list and optional details.
pub(super) fn draw_ci_view(frame: &mut Frame, app: &App, area: Rect) {
    if app.ci_state.log_stream.is_visible {
        let main_chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);

        draw_ci_log_header(frame, app, main_chunks[0]);
        draw_ci_log_viewer(frame, app, main_chunks[1]);
    } else {
        let main_chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).split(area);

        draw_ci_summary(frame, app, main_chunks[0]);

        if app.ci_state.show_details {
            let content_chunks =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(main_chunks[1]);

            draw_ci_run_list(frame, app, content_chunks[0]);
            draw_ci_run_details(frame, app, content_chunks[1]);
        } else {
            draw_ci_run_list(frame, app, main_chunks[1]);
        }
    }
}

/// Draw CI summary bar with status counts.
fn draw_ci_summary(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let runs = &app.ci_state.runs;

    let pending = runs.iter().filter(|r| r.status == "pending").count();
    let running = runs.iter().filter(|r| r.status == "running").count();
    let success = runs.iter().filter(|r| r.status == "success").count();
    let failed = runs.iter().filter(|r| r.status == "failed").count();

    let stats_text = vec![
        Line::from(vec![
            Span::styled("Pipelines: ", Style::new().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} total", runs.len()), Style::new().fg(Color::White)),
            Span::raw(" | "),
            Span::styled(format!("{} pending", pending), Style::new().fg(Color::Yellow)),
            Span::raw(" | "),
            Span::styled(format!("{} running", running), Style::new().fg(Color::Blue)),
            Span::raw(" | "),
            Span::styled(format!("{} success", success), Style::new().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(format!("{} failed", failed), Style::new().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled("Filter: ", Style::new().add_modifier(Modifier::BOLD)),
            Span::raw(app.ci_state.status_filter.as_str()),
            if let Some(ref repo) = app.ci_state.repo_filter {
                Span::raw(format!(" | Repo: {}", repo))
            } else {
                Span::raw("")
            },
        ]),
        Line::from(vec![
            Span::styled("Keys: ", Style::new().fg(Color::DarkGray)),
            Span::raw("j/k=navigate | s=status | d=details | l=logs | x=cancel | t=trigger | r=refresh"),
        ]),
    ];

    let paragraph = Paragraph::new(stats_text).block(Block::new().borders(Borders::ALL).title(" CI Pipelines "));

    frame.render_widget(paragraph, area);
}

/// Draw the CI run list.
fn draw_ci_run_list(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let runs = &app.ci_state.runs;

    let items: Vec<ListItem> = runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let status_color = ci_status_color(&run.status);

            let style = if is_selected_row(i, app.ci_state.selected_run) {
                Style::new().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::new()
            };

            let created_str = format_timestamp_ms(run.created_at_ms);

            ListItem::new(Line::from(vec![
                Span::styled(&run.run_id[..run.run_id.len().min(8)], style),
                Span::raw(" "),
                Span::styled(&run.ref_name, Style::new().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(&run.status, Style::new().fg(status_color)),
                Span::raw(" "),
                Span::styled(created_str, Style::new().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let title = format!(" Pipeline Runs ({}) ", runs.len());

    let list = List::new(items).block(Block::new().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}

fn ci_run_detail_lines<'a>(detail: &'a CiPipelineDetail) -> Vec<Line<'a>> {
    debug_assert!(!detail.run_id.is_empty());
    debug_assert!(!detail.status.is_empty());
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let error_bold = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
    let error = Style::new().fg(Color::Red);
    let mut lines = vec![
        Line::from(vec![Span::styled("Run ID: ", bold), Span::raw(&detail.run_id)]),
        Line::from(vec![Span::styled("Repo: ", bold), Span::raw(&detail.repo_id)]),
        Line::from(vec![
            Span::styled("Ref: ", bold),
            Span::styled(&detail.ref_name, Style::new().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Commit: ", bold),
            Span::raw(&detail.commit_hash[..detail.commit_hash.len().min(12)]),
        ]),
        Line::from(vec![
            Span::styled("Status: ", bold),
            Span::styled(&detail.status, Style::new().fg(ci_status_color(&detail.status))),
        ]),
        Line::from(vec![
            Span::styled("Created: ", bold),
            Span::raw(format_timestamp_ms(detail.created_at_ms)),
        ]),
    ];

    if let Some(completed) = detail.completed_at_ms {
        lines.push(Line::from(vec![
            Span::styled("Completed: ", bold),
            Span::raw(format_timestamp_ms(completed)),
        ]));
    }
    if !detail.stages.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("Stages:", bold)]));
        for stage in &detail.stages {
            let status_color = ci_status_color(&stage.status);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(&stage.name, bold),
                Span::raw(": "),
                Span::styled(&stage.status, Style::new().fg(status_color)),
            ]));
            for job in &stage.jobs {
                let job_color = ci_status_color(&job.status);
                lines.push(Line::from(vec![
                    Span::raw("    - "),
                    Span::raw(&job.name),
                    Span::raw(": "),
                    Span::styled(&job.status, Style::new().fg(job_color)),
                ]));
            }
        }
    }
    if let Some(ref err) = detail.error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled("Error: ", error_bold), Span::styled(err, error)]));
    }
    lines
}

/// Draw CI run details panel.
fn draw_ci_run_details(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let details_text = if let Some(ref detail) = app.ci_state.selected_detail {
        ci_run_detail_lines(detail)
    } else {
        vec![Line::from("Select a run and press 'd' to view details")]
    };

    let paragraph = Paragraph::new(details_text)
        .block(Block::new().borders(Borders::ALL).title(" Run Details "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw header for CI log viewer.
fn draw_ci_log_header(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let log_state = &app.ci_state.log_stream;

    let status_indicator = if log_state.is_streaming {
        Span::styled(" LIVE ", Style::new().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" COMPLETE ", Style::new().fg(Color::Black).bg(Color::DarkGray))
    };

    let job_info = if let (Some(run_id), Some(job_id)) = (&log_state.run_id, &log_state.job_id) {
        format!("Run: {} | Job: {}", &run_id[..run_id.len().min(8)], job_id)
    } else {
        "No job selected".to_string()
    };

    let auto_scroll = if app.ci_log_output.auto_follow() {
        Span::styled("AUTO", Style::new().fg(Color::Green))
    } else {
        Span::styled("MANUAL", Style::new().fg(Color::Yellow))
    };

    let header_text = vec![Line::from(vec![
        status_indicator,
        Span::raw(" "),
        Span::raw(job_info),
        Span::raw(" | Scroll: "),
        auto_scroll,
        Span::raw(" | "),
        Span::styled("Keys: ", Style::new().fg(Color::DarkGray)),
        Span::raw("j/k=scroll f=follow G=end g=start q=close"),
    ])];

    let paragraph = Paragraph::new(header_text).block(Block::new().borders(Borders::ALL).title(" CI Job Logs "));

    frame.render_widget(paragraph, area);
}

/// Draw CI job log viewer panel using rat-streaming StreamingOutput.
fn draw_ci_log_viewer(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let log_state = &app.ci_state.log_stream;

    if let Some(ref error) = log_state.error {
        let error_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Error: ", Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(error, Style::new().fg(Color::Red)),
            ]),
            Line::from(""),
            Line::from("Press 'q' to close this view"),
        ];

        let paragraph = Paragraph::new(error_text)
            .block(Block::new().borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
        return;
    }

    let border_style = Style::new().fg(Color::DarkGray);

    // Use StreamingOutput to get rendered lines — need mutable access
    // Since we only have &App, render using Paragraph from the output's stats
    let total_lines = app.ci_log_output.total_lines();

    if total_lines == 0 {
        let msg = if log_state.is_streaming {
            "Waiting for logs..."
        } else {
            "No logs available"
        };
        let paragraph = Paragraph::new(Span::styled(msg, Style::new().fg(Color::DarkGray)))
            .block(Block::new().borders(Borders::ALL).title(" Logs "));
        frame.render_widget(paragraph, area);
        return;
    }

    let stats_line = app.ci_log_output.render_stats(border_style);

    let paragraph = Paragraph::new(vec![stats_line])
        .block(Block::new().borders(Borders::ALL).title(format!(" {} lines ", total_lines)));

    frame.render_widget(paragraph, area);
}
