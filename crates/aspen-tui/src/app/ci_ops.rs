//! CI pipeline operations and log viewer.

use tracing::warn;

use super::state::App;
use crate::types::MAX_DISPLAYED_CI_RUNS;

impl App {
    /// Refresh the CI pipeline runs list from the cluster.
    pub(crate) async fn refresh_ci_runs(&mut self) {
        let status_filter = self.ci_state.status_filter.to_rpc_filter();
        let limit = Some(MAX_DISPLAYED_CI_RUNS as u32);

        match self.client.ci_list_runs(self.ci_state.repo_filter.clone(), status_filter, limit).await {
            Ok(runs) => {
                self.ci_state.runs = runs;
                // Reset selection if out of bounds
                if self.ci_state.selected_run >= self.ci_state.runs.len() && !self.ci_state.runs.is_empty() {
                    self.ci_state.selected_run = self.ci_state.runs.len() - 1;
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to list CI runs");
                self.set_status(&format!("Failed to list CI runs: {}", e));
            }
        }
    }

    /// Refresh the selected CI run's details.
    pub(crate) async fn refresh_selected_ci_run(&mut self) {
        if self.ci_state.runs.is_empty() {
            return;
        }

        let Some(run) = self.ci_state.runs.get(self.ci_state.selected_run) else {
            return;
        };

        let run_id = run.run_id.clone();

        match self.client.ci_get_status(&run_id).await {
            Ok(detail) => {
                self.ci_state.selected_detail = Some(detail);
            }
            Err(e) => {
                warn!(error = %e, run_id = %run_id, "failed to get CI run details");
                self.set_status(&format!("Failed to get CI run details: {}", e));
            }
        }
    }

    /// Cancel the currently selected CI pipeline run.
    pub(crate) async fn cancel_selected_ci_run(&mut self) {
        if self.ci_state.runs.is_empty() {
            self.set_status("No CI run selected");
            return;
        }

        let Some(run) = self.ci_state.runs.get(self.ci_state.selected_run) else {
            self.set_status("No CI run selected");
            return;
        };

        let run_id = run.run_id.clone();

        match self.client.ci_cancel_run(&run_id, Some("Cancelled from TUI".to_string())).await {
            Ok(()) => {
                self.set_status(&format!("CI run {} cancelled", run_id));
                // Refresh to show updated status
                self.refresh_ci_runs().await;
            }
            Err(e) => {
                self.set_status(&format!("Failed to cancel CI run: {}", e));
            }
        }
    }

    /// Open the CI log viewer for the selected job.
    ///
    /// Fetches logs for the currently selected job in the CI run details view.
    pub(crate) async fn open_ci_log_viewer(&mut self) {
        // Need details view to select a job
        if !self.ci_state.show_details {
            self.set_status("Press 'd' to show details first, then 'l' to view logs");
            return;
        }

        let Some(run) = self.ci_state.runs.get(self.ci_state.selected_run) else {
            self.set_status("No CI run selected");
            return;
        };

        let Some(details) = &self.ci_state.selected_detail else {
            self.set_status("No CI run details available");
            return;
        };

        // Find the selected job from the stages
        let selected_job_idx = self.ci_state.selected_job_index.unwrap_or(0);
        let mut job_idx = 0;
        let mut found_job: Option<(String, String)> = None;

        for stage in &details.stages {
            for job in &stage.jobs {
                if job_idx == selected_job_idx {
                    found_job = Some((run.run_id.clone(), job.id.clone()));
                    break;
                }
                job_idx += 1;
            }
            if found_job.is_some() {
                break;
            }
        }

        let Some((run_id, job_id)) = found_job else {
            self.set_status("No job selected - use j/k to navigate jobs");
            return;
        };

        // Clear existing logs and prepare for new stream
        self.ci_state.log_stream.run_id = Some(run_id.clone());
        self.ci_state.log_stream.job_id = Some(job_id.clone());
        self.ci_state.log_stream.lines.clear();
        self.ci_state.log_stream.scroll_position = 0;
        self.ci_state.log_stream.auto_scroll = true;
        self.ci_state.log_stream.is_streaming = true;
        self.ci_state.log_stream.last_chunk_index = 0;
        self.ci_state.log_stream.error = None;
        self.ci_state.log_stream.is_visible = true;

        self.set_status("Loading logs...");

        // Fetch logs from the server
        match self.client.ci_get_job_logs(&run_id, &job_id, 0, None).await {
            Ok(result) => {
                if !result.found {
                    self.ci_state.log_stream.error = Some("No logs found for this job".to_string());
                    self.set_status("No logs found for this job");
                    return;
                }

                // Parse log chunks into lines
                for chunk in result.chunks {
                    self.parse_log_chunk_into_lines(&chunk.content, chunk.timestamp_ms);
                }

                self.ci_state.log_stream.last_chunk_index = result.last_index;
                self.ci_state.log_stream.is_streaming = !result.is_complete;

                if self.ci_state.log_stream.auto_scroll && !self.ci_state.log_stream.lines.is_empty() {
                    self.ci_state.log_stream.scroll_position = self.ci_state.log_stream.lines.len().saturating_sub(1);
                }

                let line_count = self.ci_state.log_stream.lines.len();
                if result.is_complete {
                    self.set_status(&format!("Loaded {} log lines (complete)", line_count));
                } else {
                    self.set_status(&format!("Loaded {} log lines (streaming...)", line_count));
                }
            }
            Err(e) => {
                self.ci_state.log_stream.error = Some(format!("Failed to fetch logs: {}", e));
                self.set_status(&format!("Failed to fetch logs: {}", e));
            }
        }
    }

    /// Parse a log chunk content into individual lines.
    fn parse_log_chunk_into_lines(&mut self, content: &str, timestamp_ms: u64) {
        use crate::types::CiLogLine;

        for line in content.lines() {
            // Parse lines with format: [stream] content
            let (stream, content) = if let Some(rest) = line.strip_prefix("[stdout] ") {
                ("stdout".to_string(), rest.to_string())
            } else if let Some(rest) = line.strip_prefix("[stderr] ") {
                ("stderr".to_string(), rest.to_string())
            } else if let Some(rest) = line.strip_prefix("[build] ") {
                ("build".to_string(), rest.to_string())
            } else {
                // Unknown format - treat as stdout
                ("stdout".to_string(), line.to_string())
            };

            self.ci_state.log_stream.lines.push(CiLogLine {
                content,
                stream,
                timestamp_ms,
            });

            // Enforce bounded buffer
            if self.ci_state.log_stream.lines.len() > aspen_core::MAX_TUI_LOG_LINES {
                self.ci_state.log_stream.lines.remove(0);
            }
        }
    }

    /// Close the CI log viewer.
    pub(crate) fn close_ci_log_viewer(&mut self) {
        self.ci_state.log_stream.is_visible = false;
        self.ci_state.log_stream.is_streaming = false;
        self.ci_state.log_stream.lines.clear();
        self.ci_state.log_stream.run_id = None;
        self.ci_state.log_stream.job_id = None;
        self.set_status("Log viewer closed");
    }
}
