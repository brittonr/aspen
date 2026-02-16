//! View navigation and list selection.

use super::state::App;
use super::types::ActiveView;

impl App {
    /// Switch to the next view in cycle.
    pub(crate) async fn switch_to_next_view(&mut self) {
        let prev_view = self.active_view;
        self.active_view = self.active_view.next();
        self.on_view_change(prev_view).await;
    }

    /// Switch to the previous view in cycle.
    pub(crate) async fn switch_to_prev_view(&mut self) {
        let prev_view = self.active_view;
        self.active_view = self.active_view.prev();
        self.on_view_change(prev_view).await;
    }

    /// Switch to a specific view.
    pub(crate) async fn switch_to_view(&mut self, view: ActiveView) {
        let prev_view = self.active_view;
        self.active_view = view;
        self.on_view_change(prev_view).await;
    }

    /// Handle view change side effects (auto-refresh).
    async fn on_view_change(&mut self, prev_view: ActiveView) {
        match self.active_view {
            ActiveView::Vaults if prev_view != ActiveView::Vaults => self.refresh_vaults().await,
            ActiveView::Jobs if prev_view != ActiveView::Jobs => self.refresh_jobs().await,
            ActiveView::Workers if prev_view != ActiveView::Workers => self.refresh_workers().await,
            ActiveView::Ci if prev_view != ActiveView::Ci => self.refresh_ci_runs().await,
            _ => {}
        }
    }

    /// Navigate up in current list.
    pub(crate) fn navigate_up(&mut self) {
        match self.active_view {
            ActiveView::Vaults => {
                if self.active_vault.is_some() {
                    if self.selected_vault_key > 0 {
                        self.selected_vault_key -= 1;
                    }
                } else if self.selected_vault > 0 {
                    self.selected_vault -= 1;
                }
            }
            ActiveView::Sql => {
                if self.sql_state.selected_row > 0 {
                    self.sql_state.selected_row -= 1;
                }
            }
            ActiveView::Jobs => {
                if self.jobs_state.selected_job > 0 {
                    self.jobs_state.selected_job -= 1;
                }
            }
            ActiveView::Workers => {
                if self.workers_state.selected_worker > 0 {
                    self.workers_state.selected_worker -= 1;
                }
            }
            ActiveView::Ci => {
                if self.ci_state.selected_run > 0 {
                    self.ci_state.selected_run -= 1;
                }
            }
            _ => {
                if self.selected_node > 0 {
                    self.selected_node -= 1;
                }
            }
        }
    }

    /// Navigate down in current list.
    pub(crate) fn navigate_down(&mut self) {
        match self.active_view {
            ActiveView::Vaults => {
                if self.active_vault.is_some() {
                    let max = self.vault_keys.len().saturating_sub(1);
                    if self.selected_vault_key < max {
                        self.selected_vault_key += 1;
                    }
                } else {
                    let max = self.vaults.len().saturating_sub(1);
                    if self.selected_vault < max {
                        self.selected_vault += 1;
                    }
                }
            }
            ActiveView::Sql => {
                if let Some(result) = &self.sql_state.last_result {
                    let max = (result.rows.len().saturating_sub(1)) as u32;
                    if self.sql_state.selected_row < max {
                        self.sql_state.selected_row += 1;
                    }
                }
            }
            ActiveView::Jobs => {
                let max = (self.jobs_state.jobs.len().saturating_sub(1)) as u32;
                if self.jobs_state.selected_job < max {
                    self.jobs_state.selected_job += 1;
                }
            }
            ActiveView::Workers => {
                let max = (self.workers_state.pool_info.workers.len().saturating_sub(1)) as u32;
                if self.workers_state.selected_worker < max {
                    self.workers_state.selected_worker += 1;
                }
            }
            ActiveView::Ci => {
                let max = (self.ci_state.runs.len().saturating_sub(1)) as u32;
                if self.ci_state.selected_run < max {
                    self.ci_state.selected_run += 1;
                }
            }
            _ => {
                let max = self.nodes.len().saturating_sub(1);
                if self.selected_node < max {
                    self.selected_node += 1;
                }
            }
        }
    }
}
