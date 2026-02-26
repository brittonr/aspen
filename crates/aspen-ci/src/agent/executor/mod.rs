//! Command execution engine for the CI agent.
//!
//! Handles spawning processes, streaming output, enforcing timeouts,
//! and process lifecycle management.

mod nix_db;
mod process;
mod termination;
mod validation;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::info;

use crate::agent::error;
use crate::agent::error::Result;
use crate::agent::protocol::ExecutionRequest;
use crate::agent::protocol::ExecutionResult;
use crate::agent::protocol::LogMessage;

/// Handle to a running job, used for cancellation.
pub struct JobHandle {
    /// Cancellation sender.
    cancel_tx: oneshot::Sender<()>,
}

impl JobHandle {
    /// Cancel the running job.
    pub fn cancel(self) {
        let _ = self.cancel_tx.send(());
    }
}

/// Executor that runs commands and streams output.
pub struct Executor {
    /// Currently running jobs, keyed by job ID.
    running_jobs: Arc<Mutex<HashMap<String, JobHandle>>>,

    /// Workspace root path for directory validation.
    /// Working directories must be under this path.
    /// Defaults to `/workspace` for VM environments.
    workspace_root: std::path::PathBuf,
}

impl Executor {
    /// Create a new executor with default `/workspace` root.
    pub fn new() -> Self {
        Self {
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
            workspace_root: std::path::PathBuf::from("/workspace"),
        }
    }

    /// Create a new executor with a custom workspace root.
    ///
    /// This is useful for local execution where the workspace
    /// is not mounted at `/workspace`.
    pub fn with_workspace_root(workspace_root: std::path::PathBuf) -> Self {
        Self {
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
            workspace_root,
        }
    }

    /// Execute a command and stream output via the provided channel.
    ///
    /// Returns when the command completes or is cancelled.
    pub async fn execute(
        &self,
        request: ExecutionRequest,
        log_tx: mpsc::Sender<LogMessage>,
    ) -> Result<ExecutionResult> {
        let job_id = request.id.clone();
        let start = Instant::now();

        // Validate working directory
        self.validate_working_dir(&request.working_dir)?;

        // Load nix database dump if present and command is nix-related.
        // The host generates this file with `nix-store --dump-db` after prefetching
        // the build closure. We load it here (not at startup) because the dump is
        // written AFTER the VM boots and the job is assigned.
        if nix_db::is_nix_command(&request.command) {
            nix_db::load_nix_db_dump(&self.workspace_root).await;
        }

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Register job handle
        {
            let mut jobs = self.running_jobs.lock().await;
            jobs.insert(job_id.clone(), JobHandle { cancel_tx });
        }

        // Execute with cleanup on drop
        let result = process::execute_inner(&request, log_tx.clone(), cancel_rx).await;

        // Unregister job
        {
            let mut jobs = self.running_jobs.lock().await;
            jobs.remove(&job_id);
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok((exit_code, stdout, stderr)) => Ok(ExecutionResult {
                id: job_id,
                exit_code,
                stdout,
                stderr,
                duration_ms,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                id: job_id,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Cancel a running job by ID.
    pub async fn cancel(&self, job_id: &str) -> Result<()> {
        let handle = {
            let mut jobs = self.running_jobs.lock().await;
            jobs.remove(job_id)
        };

        match handle {
            Some(handle) => {
                handle.cancel();
                info!(job_id = %job_id, "job cancelled");
                Ok(())
            }
            None => error::JobNotFoundSnafu { id: job_id }.fail(),
        }
    }

    /// Check if a job is running.
    pub async fn is_running(&self, job_id: &str) -> bool {
        let jobs = self.running_jobs.lock().await;
        jobs.contains_key(job_id)
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
