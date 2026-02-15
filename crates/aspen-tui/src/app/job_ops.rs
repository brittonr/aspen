//! Job and worker operations.

use tracing::warn;

use super::state::App;
use crate::types::MAX_DISPLAYED_JOBS;

impl App {
    /// Refresh the jobs list from the cluster.
    pub(crate) async fn refresh_jobs(&mut self) {
        let status_filter = self.jobs_state.status_filter.to_rpc_filter();
        let limit = Some(MAX_DISPLAYED_JOBS as u32);

        match self.client.list_jobs(status_filter, limit).await {
            Ok(mut jobs) => {
                // Apply priority filter client-side (API may not support it)
                if let Some(priority) = self.jobs_state.priority_filter.to_priority() {
                    jobs.retain(|j| j.priority == priority);
                }
                self.jobs_state.jobs = jobs;
                // Reset selection if out of bounds
                if self.jobs_state.selected_job >= self.jobs_state.jobs.len() && !self.jobs_state.jobs.is_empty() {
                    self.jobs_state.selected_job = self.jobs_state.jobs.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to list jobs");
                self.set_status(&format!("Failed to list jobs: {}", e));
            }
        }

        // Also refresh queue stats
        match self.client.get_queue_stats().await {
            Ok(stats) => {
                self.jobs_state.queue_stats = stats;
            }
            Err(e) => {
                warn!(error = %e, "failed to get queue stats");
            }
        }
    }

    /// Refresh the workers list from the cluster.
    pub(crate) async fn refresh_workers(&mut self) {
        match self.client.get_worker_status().await {
            Ok(pool_info) => {
                self.workers_state.pool_info = pool_info;
                // Reset selection if out of bounds
                if self.workers_state.selected_worker >= self.workers_state.pool_info.workers.len()
                    && !self.workers_state.pool_info.workers.is_empty()
                {
                    self.workers_state.selected_worker = self.workers_state.pool_info.workers.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to get worker status");
                self.set_status(&format!("Failed to get worker status: {}", e));
            }
        }
    }

    /// Cancel the currently selected job.
    pub(crate) async fn cancel_selected_job(&mut self) {
        if self.jobs_state.jobs.is_empty() {
            self.set_status("No job selected");
            return;
        }

        let Some(job) = self.jobs_state.jobs.get(self.jobs_state.selected_job) else {
            self.set_status("No job selected");
            return;
        };

        let job_id = job.job_id.clone();

        match self.client.cancel_job(&job_id, Some("Cancelled from TUI".to_string())).await {
            Ok(()) => {
                self.set_status(&format!("Job {} cancelled", job_id));
                // Refresh to show updated status
                self.refresh_jobs().await;
            }
            Err(e) => {
                self.set_status(&format!("Failed to cancel job: {}", e));
            }
        }
    }
}
