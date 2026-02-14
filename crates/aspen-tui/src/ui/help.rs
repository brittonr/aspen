//! Help view rendering.

use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

/// Draw help view.
pub(super) fn draw_help_view(frame: &mut Frame, area: Rect) {
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
    lines.extend(build_ci_section());

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
            "  1-9              Jump to view (1=Cluster, 2=Metrics, 3=KV, 4=Vaults, 5=SQL, 6=Logs, 7=Jobs, 8=Workers, 9=CI)",
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

fn build_ci_section() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![Span::styled(
            "CI View",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )]),
        Line::from("  j/k              Navigate pipeline run list"),
        Line::from("  s                Cycle status filter (All/Pending/Running/Success/Failed/Cancelled)"),
        Line::from("  d                Toggle details panel"),
        Line::from("  x                Cancel selected pipeline run"),
        Line::from("  t                Trigger new pipeline"),
        Line::from("  r                Refresh pipeline runs"),
        Line::from(""),
    ]
}
