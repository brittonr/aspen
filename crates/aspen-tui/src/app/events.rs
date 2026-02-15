//! Event handling and command execution.

use std::time::Duration;

use color_eyre::Result;
use tracing::warn;

use super::state::App;
use super::state::MAX_INPUT_SIZE;
use super::types::ActiveView;
use super::types::InputMode;
use crate::commands::Command;
use crate::commands::key_to_command;
use crate::event::Event;

impl App {
    /// Handle incoming event.
    ///
    /// Tiger Style: Centralized event dispatch.
    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Tick => self.on_tick().await,
            Event::Key(key_event) => self.on_key(key_event).await,
            Event::Mouse(_) => {}     // Ignore mouse events for now
            Event::Resize(_, _) => {} // Terminal handles resize
        }
        Ok(())
    }

    /// Handle tick event (periodic refresh).
    async fn on_tick(&mut self) {
        // Auto-refresh every tick if not already refreshing
        if !self.refreshing {
            // Wrap refresh in a timeout to prevent UI freezing
            // If refresh takes too long, skip it and try again next tick
            let refresh_timeout = Duration::from_secs(3);
            match tokio::time::timeout(refresh_timeout, self.refresh_cluster_state()).await {
                Ok(_) => {
                    // Refresh completed successfully
                }
                Err(_) => {
                    // Refresh timed out, mark as not refreshing so we try again
                    self.refreshing = false;
                    self.set_status("Network timeout - retrying...");
                    warn!("Refresh timed out after {:?}", refresh_timeout);
                }
            }
        }

        // Clear old status messages (after 5 seconds)
        if let Some((_, timestamp)) = &self.status_message
            && timestamp.elapsed() > Duration::from_secs(5)
        {
            self.status_message = None;
        }
    }

    /// Handle key event.
    ///
    /// Uses the command pattern to separate key interpretation from execution.
    /// Tiger Style: Keeps this function small by delegating to `execute_command`.
    async fn on_key(&mut self, key: crossterm::event::KeyEvent) {
        if let Some(cmd) = key_to_command(key, self.active_view, self.input_mode) {
            self.execute_command(cmd).await;
        }
    }

    /// Execute a command.
    ///
    /// Tiger Style: Single point of command execution for testability.
    pub(crate) async fn execute_command(&mut self, cmd: Command) {
        match cmd {
            // Global commands
            Command::Quit => self.should_quit = true,
            Command::Refresh => self.refresh_cluster_state().await,

            // View navigation
            Command::NextView => self.switch_to_next_view().await,
            Command::PrevView => self.switch_to_prev_view().await,
            Command::SwitchToView(view) => self.switch_to_view(view).await,
            Command::ShowHelp => self.active_view = ActiveView::Help,

            // List navigation
            Command::NavigateUp => self.navigate_up(),
            Command::NavigateDown => self.navigate_down(),

            // Input mode transitions
            Command::EnterEditMode => self.input_mode = InputMode::Editing,
            Command::EnterSqlEditMode => {
                self.input_mode = InputMode::SqlEditing;
                self.set_status("Editing SQL query (Enter to save, Esc to cancel, Up/Down for history)");
            }
            Command::ExitEditMode => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            Command::ExitSqlEditMode => {
                self.input_mode = InputMode::Normal;
                self.set_status("Edit cancelled");
            }

            // Cluster operations
            Command::InitCluster => self.init_cluster().await,
            Command::ConnectHttp => {
                self.input_mode = InputMode::Editing;
                self.input_buffer = "http://127.0.0.1:21001".to_string();
                self.set_status("Enter HTTP node address(es) separated by spaces, then press Enter");
            }
            Command::ConnectTicket => {
                self.input_mode = InputMode::Editing;
                self.input_buffer.clear();
                self.set_status("Paste Iroh cluster ticket, then press Enter");
            }

            // Key-value operations
            Command::ExecuteKvOperation => {
                // Handled in InputEnter based on context
            }

            // Vault operations
            Command::EnterVault => self.enter_vault().await,
            Command::ExitVault => self.exit_vault(),

            // SQL operations
            Command::ExecuteSqlQuery => self.execute_sql_query().await,
            Command::ToggleSqlConsistency => {
                self.sql_state.consistency = self.sql_state.consistency.toggle();
                self.set_status(&format!("Consistency: {}", self.sql_state.consistency.as_str()));
            }
            Command::SqlScrollLeft => {
                if self.sql_state.result_scroll_col > 0 {
                    self.sql_state.result_scroll_col -= 1;
                }
            }
            Command::SqlScrollRight => {
                if let Some(result) = &self.sql_state.last_result {
                    let max = result.columns.len().saturating_sub(1);
                    if self.sql_state.result_scroll_col < max {
                        self.sql_state.result_scroll_col += 1;
                    }
                }
            }

            // Log operations
            Command::LogScrollUp => self.log_scroll = self.log_scroll.saturating_sub(10),
            Command::LogScrollDown => self.log_scroll = self.log_scroll.saturating_add(10),

            // Jobs operations
            Command::CycleJobStatusFilter => {
                self.jobs_state.status_filter = self.jobs_state.status_filter.next();
                self.set_status(&format!("Status filter: {}", self.jobs_state.status_filter.as_str()));
                self.refresh_jobs().await;
            }
            Command::CycleJobPriorityFilter => {
                self.jobs_state.priority_filter = self.jobs_state.priority_filter.next();
                self.set_status(&format!("Priority filter: {}", self.jobs_state.priority_filter.as_str()));
                self.refresh_jobs().await;
            }
            Command::ToggleJobDetails => self.jobs_state.show_details = !self.jobs_state.show_details,
            Command::CancelSelectedJob => self.cancel_selected_job().await,

            // Workers operations
            Command::ToggleWorkerDetails => self.workers_state.show_details = !self.workers_state.show_details,

            // CI operations
            Command::CycleCiStatusFilter => {
                self.ci_state.status_filter = self.ci_state.status_filter.next();
                self.set_status(&format!("Status filter: {}", self.ci_state.status_filter.as_str()));
                self.refresh_ci_runs().await;
            }
            Command::ToggleCiDetails => {
                self.ci_state.show_details = !self.ci_state.show_details;
                if self.ci_state.show_details {
                    self.refresh_selected_ci_run().await;
                }
            }
            Command::CancelSelectedCiRun => self.cancel_selected_ci_run().await,
            Command::TriggerCiPipeline => {
                self.input_mode = InputMode::Editing;
                self.input_buffer.clear();
                self.set_status("Enter: <repo_id> <ref_name> [commit_hash]");
            }

            // CI log viewer operations
            Command::CiOpenLogViewer => self.open_ci_log_viewer().await,
            Command::CiCloseLogViewer => {
                if self.ci_state.log_stream.is_visible {
                    self.close_ci_log_viewer();
                } else {
                    self.should_quit = true;
                }
            }
            Command::CiLogScrollUp => {
                if self.ci_state.log_stream.scroll_position > 0 {
                    self.ci_state.log_stream.scroll_position =
                        self.ci_state.log_stream.scroll_position.saturating_sub(10);
                    self.ci_state.log_stream.auto_scroll = false;
                }
            }
            Command::CiLogScrollDown => {
                let max_scroll = self.ci_state.log_stream.lines.len().saturating_sub(1);
                if self.ci_state.log_stream.scroll_position < max_scroll {
                    self.ci_state.log_stream.scroll_position =
                        (self.ci_state.log_stream.scroll_position + 10).min(max_scroll);
                }
            }
            Command::CiLogScrollToEnd => {
                self.ci_state.log_stream.scroll_position = self.ci_state.log_stream.lines.len().saturating_sub(1);
                self.ci_state.log_stream.auto_scroll = true;
            }
            Command::CiLogScrollToStart => {
                self.ci_state.log_stream.scroll_position = 0;
                self.ci_state.log_stream.auto_scroll = false;
            }
            Command::CiLogToggleFollow => {
                self.ci_state.log_stream.auto_scroll = !self.ci_state.log_stream.auto_scroll;
                if self.ci_state.log_stream.auto_scroll {
                    self.ci_state.log_stream.scroll_position = self.ci_state.log_stream.lines.len().saturating_sub(1);
                    self.set_status("Follow mode enabled");
                } else {
                    self.set_status("Follow mode disabled");
                }
            }

            // Input handling
            Command::InputChar(c) => self.handle_input_char(c),
            Command::InputBackspace => self.handle_input_backspace(),
            Command::InputTab => std::mem::swap(&mut self.key_buffer, &mut self.input_buffer),
            Command::InputEnter => self.handle_input_enter().await,
            Command::HistoryPrev => self.sql_state.history_prev(),
            Command::HistoryNext => self.sql_state.history_next(),
        }
    }

    /// Handle character input in editing modes.
    fn handle_input_char(&mut self, c: char) {
        use crate::types::MAX_SQL_QUERY_SIZE;

        match self.input_mode {
            InputMode::Editing => {
                if self.input_buffer.len() < MAX_INPUT_SIZE {
                    self.input_buffer.push(c);
                }
            }
            InputMode::SqlEditing => {
                if self.sql_state.query_buffer.len() < MAX_SQL_QUERY_SIZE {
                    self.sql_state.query_buffer.push(c);
                    self.sql_state.history_browsing = false;
                }
            }
            InputMode::Normal => {}
        }
    }

    /// Handle backspace in editing modes.
    fn handle_input_backspace(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                self.input_buffer.pop();
            }
            InputMode::SqlEditing => {
                self.sql_state.query_buffer.pop();
                self.sql_state.history_browsing = false;
            }
            InputMode::Normal => {}
        }
    }

    /// Handle enter in editing modes.
    async fn handle_input_enter(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                let is_ticket_connect = self
                    .status_message
                    .as_ref()
                    .map(|(msg, _)| msg.contains("Paste Iroh cluster ticket"))
                    .unwrap_or(false);

                if is_ticket_connect {
                    self.connect_iroh_ticket(&self.input_buffer.clone()).await;
                } else {
                    self.execute_kv_operation().await;
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            InputMode::SqlEditing => {
                self.input_mode = InputMode::Normal;
                self.set_status("Query saved (press 'e' to execute)");
            }
            InputMode::Normal => {}
        }
    }
}
