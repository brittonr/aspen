//! Command pattern for TUI key handling.
//!
//! Separates key-to-command mapping from command execution for better
//! testability and maintainability. Tiger Style: keeps functions small
//! by splitting input interpretation from action execution.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

use crate::app::ActiveView;
use crate::app::InputMode;

/// Commands that can be executed in the TUI.
///
/// Each command represents a discrete action that modifies application state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Global commands
    Quit,
    Refresh,

    // View navigation
    NextView,
    PrevView,
    SwitchToView(ActiveView),
    ShowHelp,

    // List navigation
    NavigateUp,
    NavigateDown,

    // Input mode transitions
    EnterEditMode,
    EnterSqlEditMode,
    ExitEditMode,
    ExitSqlEditMode,

    // Cluster operations
    InitCluster,
    ConnectHttp,
    ConnectTicket,

    // Key-value operations
    ExecuteKvOperation,

    // Vault operations
    EnterVault,
    ExitVault,

    // SQL operations
    ExecuteSqlQuery,
    ToggleSqlConsistency,
    SqlScrollLeft,
    SqlScrollRight,

    // Log operations
    LogScrollUp,
    LogScrollDown,

    // Jobs operations
    CycleJobStatusFilter,
    CycleJobPriorityFilter,
    ToggleJobDetails,
    CancelSelectedJob,

    // Workers operations
    ToggleWorkerDetails,

    // CI operations
    CycleCiStatusFilter,
    ToggleCiDetails,
    CancelSelectedCiRun,
    TriggerCiPipeline,

    // Input handling (editing modes)
    InputChar(char),
    InputBackspace,
    InputTab,
    InputEnter,
    HistoryPrev,
    HistoryNext,
}

/// Map a key event to a command based on current view and input mode.
///
/// Returns `None` if the key doesn't map to any command.
pub fn key_to_command(key: KeyEvent, view: ActiveView, mode: InputMode) -> Option<Command> {
    match mode {
        InputMode::Normal => normal_mode_command(key, view),
        InputMode::Editing => editing_mode_command(key),
        InputMode::SqlEditing => sql_editing_mode_command(key),
    }
}

/// Map keys in normal navigation mode.
fn normal_mode_command(key: KeyEvent, view: ActiveView) -> Option<Command> {
    match key.code {
        // Quit commands
        KeyCode::Char('q') => Some(Command::Quit),
        KeyCode::Esc => {
            // In vault view with active vault, Esc goes back instead of quit
            if view == ActiveView::Vaults {
                Some(Command::ExitVault)
            } else {
                Some(Command::Quit)
            }
        }

        // View navigation
        KeyCode::Tab => Some(Command::NextView),
        KeyCode::BackTab => Some(Command::PrevView),
        KeyCode::Char('1') => Some(Command::SwitchToView(ActiveView::Cluster)),
        KeyCode::Char('2') => Some(Command::SwitchToView(ActiveView::Metrics)),
        KeyCode::Char('3') => Some(Command::SwitchToView(ActiveView::KeyValue)),
        KeyCode::Char('4') => Some(Command::SwitchToView(ActiveView::Vaults)),
        KeyCode::Char('5') => Some(Command::SwitchToView(ActiveView::Sql)),
        KeyCode::Char('6') => Some(Command::SwitchToView(ActiveView::Logs)),
        KeyCode::Char('7') => Some(Command::SwitchToView(ActiveView::Jobs)),
        KeyCode::Char('8') => Some(Command::SwitchToView(ActiveView::Workers)),
        KeyCode::Char('9') => Some(Command::SwitchToView(ActiveView::Ci)),
        KeyCode::Char('?') => Some(Command::ShowHelp),

        // List navigation
        KeyCode::Up | KeyCode::Char('k') => Some(Command::NavigateUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Command::NavigateDown),

        // Refresh
        KeyCode::Char('r') => Some(Command::Refresh),

        // Cluster operations
        KeyCode::Char('i') => Some(Command::InitCluster),

        // Connection commands (context-sensitive)
        KeyCode::Char('c') => {
            if view == ActiveView::Sql {
                Some(Command::ToggleSqlConsistency)
            } else {
                Some(Command::ConnectHttp)
            }
        }
        KeyCode::Char('t') => {
            if view == ActiveView::Ci {
                Some(Command::TriggerCiPipeline)
            } else {
                Some(Command::ConnectTicket)
            }
        }

        // View-specific commands
        KeyCode::Char('h') if view == ActiveView::Sql => Some(Command::SqlScrollLeft),
        KeyCode::Char('l') if view == ActiveView::Sql => Some(Command::SqlScrollRight),
        KeyCode::Char('e') if view == ActiveView::Sql => Some(Command::ExecuteSqlQuery),

        // Enter key (context-sensitive)
        KeyCode::Enter => match view {
            ActiveView::Sql => Some(Command::EnterSqlEditMode),
            ActiveView::KeyValue => Some(Command::EnterEditMode),
            ActiveView::Vaults => Some(Command::EnterVault),
            _ => None,
        },

        // Backspace in vault view
        KeyCode::Backspace if view == ActiveView::Vaults => Some(Command::ExitVault),

        // Log scroll
        KeyCode::PageUp if view == ActiveView::Logs => Some(Command::LogScrollUp),
        KeyCode::PageDown if view == ActiveView::Logs => Some(Command::LogScrollDown),

        // Jobs view commands
        KeyCode::Char('s') if view == ActiveView::Jobs => Some(Command::CycleJobStatusFilter),
        KeyCode::Char('p') if view == ActiveView::Jobs => Some(Command::CycleJobPriorityFilter),
        KeyCode::Char('d') if view == ActiveView::Jobs => Some(Command::ToggleJobDetails),
        KeyCode::Char('x') if view == ActiveView::Jobs => Some(Command::CancelSelectedJob),

        // Workers view commands
        KeyCode::Char('d') if view == ActiveView::Workers => Some(Command::ToggleWorkerDetails),

        // CI view commands
        KeyCode::Char('s') if view == ActiveView::Ci => Some(Command::CycleCiStatusFilter),
        KeyCode::Char('d') if view == ActiveView::Ci => Some(Command::ToggleCiDetails),
        KeyCode::Char('x') if view == ActiveView::Ci => Some(Command::CancelSelectedCiRun),

        _ => None,
    }
}

/// Map keys in text editing mode.
fn editing_mode_command(key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Esc => Some(Command::ExitEditMode),
        KeyCode::Enter => Some(Command::InputEnter),
        KeyCode::Backspace => Some(Command::InputBackspace),
        KeyCode::Tab => Some(Command::InputTab),
        KeyCode::Char(c) => Some(Command::InputChar(c)),
        _ => None,
    }
}

/// Map keys in SQL editing mode.
fn sql_editing_mode_command(key: KeyEvent) -> Option<Command> {
    match key.code {
        KeyCode::Esc => Some(Command::ExitSqlEditMode),
        KeyCode::Enter => Some(Command::InputEnter),
        KeyCode::Backspace => Some(Command::InputBackspace),
        KeyCode::Up => Some(Command::HistoryPrev),
        KeyCode::Down => Some(Command::HistoryNext),
        KeyCode::Char(c) => Some(Command::InputChar(c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEventKind;
    use crossterm::event::KeyEventState;
    use crossterm::event::KeyModifiers;

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_quit_command() {
        assert_eq!(
            key_to_command(key(KeyCode::Char('q')), ActiveView::Cluster, InputMode::Normal),
            Some(Command::Quit)
        );
    }

    #[test]
    fn test_view_navigation() {
        assert_eq!(key_to_command(key(KeyCode::Tab), ActiveView::Cluster, InputMode::Normal), Some(Command::NextView));
        assert_eq!(
            key_to_command(key(KeyCode::BackTab), ActiveView::Cluster, InputMode::Normal),
            Some(Command::PrevView)
        );
        assert_eq!(
            key_to_command(key(KeyCode::Char('1')), ActiveView::Metrics, InputMode::Normal),
            Some(Command::SwitchToView(ActiveView::Cluster))
        );
    }

    #[test]
    fn test_esc_context_sensitive() {
        // In vault view, Esc exits vault
        assert_eq!(key_to_command(key(KeyCode::Esc), ActiveView::Vaults, InputMode::Normal), Some(Command::ExitVault));
        // In other views, Esc quits
        assert_eq!(key_to_command(key(KeyCode::Esc), ActiveView::Cluster, InputMode::Normal), Some(Command::Quit));
    }

    #[test]
    fn test_c_key_context_sensitive() {
        // In SQL view, 'c' toggles consistency
        assert_eq!(
            key_to_command(key(KeyCode::Char('c')), ActiveView::Sql, InputMode::Normal),
            Some(Command::ToggleSqlConsistency)
        );
        // In other views, 'c' connects HTTP
        assert_eq!(
            key_to_command(key(KeyCode::Char('c')), ActiveView::Cluster, InputMode::Normal),
            Some(Command::ConnectHttp)
        );
    }

    #[test]
    fn test_editing_mode() {
        assert_eq!(
            key_to_command(key(KeyCode::Esc), ActiveView::KeyValue, InputMode::Editing),
            Some(Command::ExitEditMode)
        );
        assert_eq!(
            key_to_command(key(KeyCode::Char('a')), ActiveView::KeyValue, InputMode::Editing),
            Some(Command::InputChar('a'))
        );
    }

    #[test]
    fn test_sql_editing_mode() {
        assert_eq!(
            key_to_command(key(KeyCode::Up), ActiveView::Sql, InputMode::SqlEditing),
            Some(Command::HistoryPrev)
        );
        assert_eq!(
            key_to_command(key(KeyCode::Down), ActiveView::Sql, InputMode::SqlEditing),
            Some(Command::HistoryNext)
        );
    }
}
