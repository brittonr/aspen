//! Key-value and vaults view rendering.

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

/// Draw key-value operations view.
pub(super) fn draw_kv_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    draw_kv_instructions(frame, chunks[0]);
    draw_kv_results(frame, app, chunks[1]);
}

/// Draw KV instructions panel.
fn draw_kv_instructions(frame: &mut Frame, area: Rect) {
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

    frame.render_widget(paragraph, area);
}

/// Draw KV results panel.
fn draw_kv_results(frame: &mut Frame, app: &App, area: Rect) {
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

    frame.render_widget(results, area);
}

/// Draw vaults browser view.
pub(super) fn draw_vaults_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    if let Some(vault_name) = &app.active_vault {
        draw_vault_keys_list(frame, app, chunks[0], vault_name);
        draw_vault_key_detail(frame, app, chunks[1]);
    } else {
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
