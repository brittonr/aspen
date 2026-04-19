//! Key-value and vaults view rendering.

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

use crate::app::App;

fn selected_style(is_selected: bool) -> Style {
    if is_selected {
        return Style::new().bg(Color::DarkGray).add_modifier(Modifier::BOLD);
    }
    Style::new()
}

fn bordered_block(title: impl Into<ratatui::text::Line<'static>>) -> Block<'static> {
    Block::new().borders(Borders::ALL).title(title)
}

/// Draw key-value operations view.
pub(super) fn draw_kv_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(6), Constraint::Min(0)]).split(area);
    debug_assert!(chunks.len() == 2);

    draw_kv_instructions(frame, chunks[0]);
    draw_kv_results(frame, app, chunks[1]);
}

/// Draw KV instructions panel.
fn draw_kv_instructions(frame: &mut Frame, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let instructions = vec![
        Line::from("Press Enter to input commands"),
        Line::from(""),
        Line::from("Commands:"),
        Line::from("  get <key>        - Read a key"),
        Line::from("  set <key> <val>  - Write a key-value pair"),
    ];
    debug_assert!(!instructions.is_empty());

    let paragraph = Paragraph::new(instructions)
        .block(bordered_block(" Key-Value Operations "))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw KV results panel.
fn draw_kv_results(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let result_content = if let Some((key, value)) = &app.last_read_result {
        let value_text = value
            .as_ref()
            .map(|raw_value| String::from_utf8_lossy(raw_value).to_string())
            .unwrap_or_else(|| "(not found)".to_string());
        vec![
            Line::from(vec![
                Span::raw("Key: "),
                Span::styled(key, Style::new().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![Span::raw("Value: "), Span::raw(value_text)]),
        ]
    } else {
        vec![Line::from("No results yet")]
    };
    debug_assert!(!result_content.is_empty());

    let results = Paragraph::new(result_content).block(bordered_block(" Results ")).wrap(Wrap { trim: true });

    frame.render_widget(results, area);
}

/// Draw vaults browser view.
pub(super) fn draw_vaults_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area);
    debug_assert!(chunks.len() == 2);

    if let Some(vault_name) = &app.active_vault {
        draw_vault_keys_list(frame, app, chunks[0], vault_name);
        draw_vault_key_detail(frame, app, chunks[1]);
        return;
    }

    draw_vaults_list(frame, app, chunks[0]);
    draw_vault_info(frame, app, chunks[1]);
}

/// Draw the list of vaults.
fn draw_vaults_list(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let items: Vec<ListItem> = app
        .vaults
        .iter()
        .enumerate()
        .map(|(index, vault)| {
            let vault_style = selected_style(index == app.selected_vault);
            let content = Line::from(vec![
                Span::styled(format!(" {} ", vault.name), vault_style),
                Span::styled(format!("({} keys)", vault.key_count), Style::new().fg(Color::DarkGray)),
            ]);
            ListItem::new(content)
        })
        .collect();
    debug_assert!(items.len() == app.vaults.len());

    let list = List::new(items)
        .block(bordered_block(" Vaults (Enter to browse, r to refresh) "))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, area);
}

/// Draw vault info panel.
fn draw_vault_info(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
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
                Span::styled(&vault.name, Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Keys: "),
                Span::styled(vault.key_count.to_string(), Style::new().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from("Press Enter to browse vault contents."),
            Line::from("Press Backspace/Esc to go back."),
        ]
    } else {
        vec![Line::from("Select a vault")]
    };
    debug_assert!(!content.is_empty());

    let paragraph = Paragraph::new(content).block(bordered_block(" Vault Info ")).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw the list of keys within a vault.
fn draw_vault_keys_list(frame: &mut Frame, app: &App, area: Rect, vault_name: &str) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let items: Vec<ListItem> = app
        .vault_keys
        .iter()
        .enumerate()
        .map(|(index, key_value)| {
            let key_style = selected_style(index == app.selected_vault_key);
            let content = Line::from(vec![Span::styled(format!(" {} ", key_value.key), key_style)]);
            ListItem::new(content)
        })
        .collect();
    debug_assert!(items.len() == app.vault_keys.len());

    let title = format!(" {} ({} keys) - Backspace to go back ", vault_name, app.vault_keys.len());
    debug_assert!(!title.is_empty());
    let list = List::new(items)
        .block(bordered_block(title))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));

    frame.render_widget(list, area);
}

/// Draw vault key detail panel.
fn draw_vault_key_detail(frame: &mut Frame, app: &App, area: Rect) {
    debug_assert!(area.width > 0);
    debug_assert!(area.height > 0);
    let content = if app.vault_keys.is_empty() {
        vec![Line::from("No keys in this vault.")]
    } else if let Some(key_value) = app.vault_keys.get(app.selected_vault_key) {
        vec![
            Line::from(vec![
                Span::raw("Key: "),
                Span::styled(&key_value.key, Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("Value:", Style::new().add_modifier(Modifier::UNDERLINED))]),
            Line::from(""),
            Line::from(key_value.value.clone()),
        ]
    } else {
        vec![Line::from("Select a key")]
    };
    debug_assert!(!content.is_empty());

    let paragraph = Paragraph::new(content).block(bordered_block(" Key Details ")).wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
