//! Jobs and workers view rendering.

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

use crate::app::App;

/// Draw jobs view with queue statistics and job list.
pub(super) fn draw_jobs_view(frame: &mut Frame, app: &App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    draw_queue_stats(frame, app, main_chunks[0]);

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

            let style = if i as u32 == app.jobs_state.selected_job {
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
    let details_text = if let Some(job) = app.jobs_state.jobs.get(app.jobs_state.selected_job as usize) {
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
pub(super) fn draw_workers_view(frame: &mut Frame, app: &App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    draw_worker_pool_summary(frame, app, main_chunks[0]);

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
            Span::raw(format!("{} / {} used", pool.used_capacity_jobs, pool.total_capacity_jobs)),
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

            let style = if i as u32 == app.workers_state.selected_worker {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(&worker.worker_id[..worker.worker_id.len().min(12)], style),
                Span::raw(" "),
                Span::styled(&worker.status, Style::default().fg(status_color)),
                Span::raw(format!(" ({}/{})", worker.active_jobs, worker.capacity_jobs)),
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
    let details_text =
        if let Some(worker) = app.workers_state.pool_info.workers.get(app.workers_state.selected_worker as usize) {
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
                    Span::raw(format!("{} (using {})", worker.capacity_jobs, worker.active_jobs)),
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
