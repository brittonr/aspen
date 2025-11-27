//! Process executor for local execution
//!
//! Handles spawning and managing child processes

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::warn;

use super::output_capture::OutputCapture;

/// Configuration for process execution
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub work_dir: std::path::PathBuf,
    pub capture_output: bool,
    pub environment: HashMap<String, String>,
}

/// Result of process execution
pub struct ProcessResult {
    pub exit_code: i32,
    pub success: bool,
    pub timed_out: bool,
}

/// Manages process spawning and lifecycle
pub struct ProcessExecutor {
    processes: Arc<RwLock<HashMap<u32, Child>>>,
}

impl ProcessExecutor {
    /// Create a new process executor
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Spawn a process and return its PID
    pub async fn spawn_process(
        &self,
        command: &str,
        args: &[String],
        config: &ProcessConfig,
        output_capture: &OutputCapture,
    ) -> Result<u32> {
        // Create command
        let mut cmd = Command::new(command);
        cmd.args(args);

        // Create work directory if needed
        if !config.work_dir.exists() {
            tokio::fs::create_dir_all(&config.work_dir).await?;
        }
        cmd.current_dir(&config.work_dir);

        // Set environment variables
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Configure output capture
        if config.capture_output {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        // Spawn the process
        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);

        // Capture output if configured
        if config.capture_output {
            if let Some(stdout) = child.stdout.take() {
                output_capture.capture_stdout(stdout);
            }

            if let Some(stderr) = child.stderr.take() {
                output_capture.capture_stderr(stderr);
            }
        }

        // Store process handle
        let mut processes = self.processes.write().await;
        processes.insert(pid, child);

        Ok(pid)
    }

    /// Wait for a process to complete with timeout
    pub async fn wait_with_timeout(
        &self,
        pid: u32,
        timeout: Duration,
    ) -> Result<ProcessResult> {
        let mut processes = self.processes.write().await;
        let child = processes
            .remove(&pid)
            .ok_or_else(|| anyhow::anyhow!("Process not found: {}", pid))?;

        drop(processes); // Release lock before waiting

        // Wait for process with timeout
        let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => Ok(ProcessResult {
                exit_code: output.status.code().unwrap_or(-1),
                success: output.status.success(),
                timed_out: false,
            }),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                // Timeout - try to kill the process (but we already removed it)
                Ok(ProcessResult {
                    exit_code: -1,
                    success: false,
                    timed_out: true,
                })
            }
        }
    }

    /// Kill a specific process
    pub async fn kill_process(&self, pid: u32) -> Result<()> {
        let mut processes = self.processes.write().await;
        if let Some(mut child) = processes.remove(&pid) {
            if let Err(e) = child.kill().await {
                warn!("Failed to kill process {}: {}", pid, e);
            }
        }
        Ok(())
    }

    /// Kill all running processes
    pub async fn kill_all(&self) -> usize {
        let mut processes = self.processes.write().await;
        let count = processes.len();

        for (pid, mut child) in processes.drain() {
            if let Err(e) = child.kill().await {
                warn!("Failed to kill process {}: {}", pid, e);
            }
        }

        count
    }
}

impl Default for ProcessExecutor {
    fn default() -> Self {
        Self::new()
    }
}
