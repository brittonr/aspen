//! CI pipeline operations and log viewer.

use aspen_ci_core::log_writer::CiLogChunk;
use aspen_client::watch::WatchEvent;
use aspen_client::watch::WatchSession;
use aspen_core::CI_LOG_COMPLETE_MARKER;
use aspen_core::CI_LOG_KV_PREFIX;
use tokio::sync::mpsc;
use tracing::debug;
use tracing::warn;

use super::state::App;
use crate::types::CiLogLine;
use crate::types::MAX_DISPLAYED_CI_RUNS;

impl App {
    /// Refresh the CI pipeline runs list from the cluster.
    pub(crate) async fn refresh_ci_runs(&mut self) {
        let status_filter = self.ci_state.status_filter.to_rpc_filter();
        let limit = Some(MAX_DISPLAYED_CI_RUNS as u32);

        match self.client.ci_list_runs(self.ci_state.repo_filter.clone(), status_filter, limit).await {
            Ok(runs) => {
                self.ci_state.runs = runs;
                let runs_len = self.ci_state.runs.len() as u32;
                if self.ci_state.selected_run >= runs_len && !self.ci_state.runs.is_empty() {
                    self.ci_state.selected_run = runs_len.saturating_sub(1);
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

        let Some(run) = self.ci_state.runs.get(self.ci_state.selected_run as usize) else {
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

        let Some(run) = self.ci_state.runs.get(self.ci_state.selected_run as usize) else {
            self.set_status("No CI run selected");
            return;
        };

        let run_id = run.run_id.clone();

        match self.client.ci_cancel_run(&run_id, Some("Cancelled from TUI".to_string())).await {
            Ok(()) => {
                self.set_status(&format!("CI run {} cancelled", run_id));
                self.refresh_ci_runs().await;
            }
            Err(e) => {
                self.set_status(&format!("Failed to cancel CI run: {}", e));
            }
        }
    }

    /// Open the CI log viewer for the selected job.
    pub(crate) async fn open_ci_log_viewer(&mut self) {
        if !self.ci_state.show_details {
            self.set_status("Press 'd' to show details first, then 'l' to view logs");
            return;
        }

        let Some(run) = self.ci_state.runs.get(self.ci_state.selected_run as usize) else {
            self.set_status("No CI run selected");
            return;
        };

        let Some(details) = &self.ci_state.selected_detail else {
            self.set_status("No CI run details available");
            return;
        };

        // Find the selected job from the stages
        let selected_job_idx = self.ci_state.selected_job_index.unwrap_or(0);
        let mut job_idx: u32 = 0;
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

        // Reset log viewer state
        self.ci_state.log_stream.run_id = Some(run_id.clone());
        self.ci_state.log_stream.job_id = Some(job_id.clone());
        self.ci_state.log_stream.is_streaming = true;
        self.ci_state.log_stream.error = None;
        self.ci_state.log_stream.is_visible = true;
        self.ci_log_output = rat_streaming::StreamingOutput::new();

        self.set_status("Loading logs...");

        // Fetch logs from the server
        match self.client.ci_get_job_logs(&run_id, &job_id, 0, None).await {
            Ok(result) => {
                if !result.was_found {
                    self.ci_state.log_stream.error = Some("No logs found for this job".to_string());
                    self.set_status("No logs found for this job");
                    return;
                }

                // Push log chunks into StreamingOutput
                for chunk in &result.chunks {
                    self.ci_log_output.push_text(&chunk.content);
                }

                self.ci_state.log_stream.last_chunk_index = result.last_index;
                self.ci_state.log_stream.is_streaming = !result.is_complete;

                let line_count = self.ci_log_output.total_lines();
                if result.is_complete {
                    self.set_status(&format!("Loaded {} log lines (complete)", line_count));
                } else {
                    self.start_ci_log_watch(&run_id, &job_id, result.last_index);
                    self.set_status(&format!("Loaded {} log lines (streaming...)", line_count));
                }
            }
            Err(e) => {
                self.ci_state.log_stream.error = Some(format!("Failed to fetch logs: {}", e));
                self.set_status(&format!("Failed to fetch logs: {}", e));
            }
        }
    }

    /// Spawn a background task to stream CI log lines via WatchSession.
    fn start_ci_log_watch(&mut self, run_id: &str, job_id: &str, last_chunk_index: u32) {
        let Some(watch_info) = self.client.watch_connection_info() else {
            debug!("watch not available: client does not support watch connections");
            return;
        };

        let watch_prefix = ci_log_watch_prefix(run_id, job_id);

        let (tx, rx) = mpsc::channel::<CiLogLine>(1000);
        self.ci_log_watch_rx = Some(rx);

        tokio::spawn(async move {
            ci_log_watch_task(
                watch_info.endpoint,
                watch_info.peer_addr,
                watch_info.cluster_id,
                watch_prefix,
                last_chunk_index,
                tx,
            )
            .await;
        });
    }

    /// Drain the CI log watch channel and push lines into StreamingOutput.
    pub(crate) fn drain_ci_log_watch(&mut self) {
        let Some(rx) = self.ci_log_watch_rx.as_mut() else {
            return;
        };

        // Drain up to 500 lines per tick to avoid blocking the UI.
        for _ in 0..500 {
            match rx.try_recv() {
                Ok(line) => {
                    self.ci_log_output.push_line(&line.content);
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.ci_log_watch_rx = None;
                    self.ci_state.log_stream.is_streaming = false;
                    break;
                }
            }
        }
    }

    /// Close the CI log viewer.
    pub(crate) fn close_ci_log_viewer(&mut self) {
        self.ci_log_watch_rx = None;
        self.ci_state.log_stream.is_visible = false;
        self.ci_state.log_stream.is_streaming = false;
        self.ci_state.log_stream.run_id = None;
        self.ci_state.log_stream.job_id = None;
        self.ci_log_output = rat_streaming::StreamingOutput::new();
        self.set_status("Log viewer closed");
    }
}

/// Background task that streams CI log lines via WatchSession.
async fn ci_log_watch_task(
    endpoint: iroh::Endpoint,
    peer_addr: iroh::EndpointAddr,
    cluster_id: String,
    watch_prefix: String,
    last_chunk_index: u32,
    tx: mpsc::Sender<CiLogLine>,
) {
    let session = match WatchSession::connect(&endpoint, peer_addr, &cluster_id).await {
        Ok(s) => s,
        Err(e) => {
            debug!(error = %e, "CI log watch connection failed");
            return;
        }
    };

    let mut subscription = match session.subscribe(watch_prefix.as_bytes().to_vec(), 0).await {
        Ok(s) => s,
        Err(e) => {
            debug!(error = %e, "CI log watch subscription failed");
            return;
        }
    };

    let mut last_seen = last_chunk_index;

    while let Some(event) = subscription.next().await {
        match event {
            WatchEvent::Set { key, value, .. } | WatchEvent::SetWithTTL { key, value, .. } => {
                let key_str = String::from_utf8_lossy(&key);

                if ci_log_is_completion_marker_key(&key_str) {
                    return;
                }

                let chunk: CiLogChunk = match serde_json::from_slice(&value) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if chunk.index < last_seen {
                    continue;
                }
                last_seen = chunk.index + 1;

                for line in chunk.content.lines() {
                    let (stream, content) = if let Some(rest) = line.strip_prefix("[stdout] ") {
                        ("stdout".to_string(), rest.to_string())
                    } else if let Some(rest) = line.strip_prefix("[stderr] ") {
                        ("stderr".to_string(), rest.to_string())
                    } else if let Some(rest) = line.strip_prefix("[build] ") {
                        ("build".to_string(), rest.to_string())
                    } else {
                        ("stdout".to_string(), line.to_string())
                    };

                    if tx
                        .send(CiLogLine {
                            content,
                            stream,
                            timestamp_ms: chunk.timestamp_ms,
                        })
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
            WatchEvent::EndOfStream { reason } => {
                debug!(reason = %reason, "CI log watch stream ended");
                return;
            }
            _ => {}
        }
    }
}

fn ci_log_watch_prefix(run_id: &str, job_id: &str) -> String {
    format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:")
}

fn ci_log_is_completion_marker_key(key: &str) -> bool {
    key.ends_with(CI_LOG_COMPLETE_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_log_watch_uses_shared_completion_marker_contract() {
        let prefix = ci_log_watch_prefix("run-1", "job-1");
        assert_eq!(prefix, format!("{}run-1:job-1:", CI_LOG_KV_PREFIX));

        let completion_key = format!("{prefix}{CI_LOG_COMPLETE_MARKER}");
        assert!(ci_log_is_completion_marker_key(&completion_key));
        assert!(!ci_log_is_completion_marker_key(&format!("{prefix}0000000001")));
    }
}
