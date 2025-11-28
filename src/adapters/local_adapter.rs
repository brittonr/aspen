//! Local process execution backend for development and testing

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

use crate::domain::types::Job;
use crate::worker_trait::WorkResult;
use crate::common::timestamp::current_timestamp_or_zero;

use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};
use super::output_capture::{OutputCapture, CapturedOutput};
use super::process_executor::{ProcessExecutor, ProcessConfig};
use super::execution_tracker::ExecutionTracker;

/// Configuration for local process adapter
#[derive(Debug, Clone)]
pub struct LocalProcessConfig {
    /// Maximum concurrent processes
    pub max_concurrent: usize,
    /// Default timeout for process execution
    pub default_timeout: Duration,
    /// Working directory for processes
    pub work_dir: std::path::PathBuf,
    /// Whether to capture stdout/stderr
    pub capture_output: bool,
}

impl Default for LocalProcessConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            default_timeout: super::DEFAULT_EXECUTION_TIMEOUT,
            work_dir: std::env::temp_dir().join("blixard-local"),
            capture_output: true,
        }
    }
}

/// Local process execution backend
pub struct LocalProcessAdapter {
    config: LocalProcessConfig,
    tracker: ExecutionTracker,
    executor: ProcessExecutor,
}

impl LocalProcessAdapter {
    /// Create a new local process adapter
    pub fn new(config: LocalProcessConfig) -> Self {
        Self {
            config,
            tracker: ExecutionTracker::new(),
            executor: ProcessExecutor::new(),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(LocalProcessConfig::default())
    }

    /// Execute a job as a local process
    async fn execute_process(&self, handle: ExecutionHandle, job: Job, exec_config: ExecutionConfig) {
        let start_time = std::time::Instant::now();

        // Extract command from job payload
        let (command, args) = self.extract_command(&job);

        debug!(
            "Local adapter: executing command '{}' with args {:?}",
            command, args
        );

        // Create output capture
        let output_capture = OutputCapture::new();

        // Create process config
        let process_config = ProcessConfig {
            work_dir: self.config.work_dir.clone(),
            capture_output: self.config.capture_output,
            environment: exec_config.environment.clone(),
        };

        // Spawn the process
        let pid = match self.executor.spawn_process(&command, &args, &process_config, &output_capture).await {
            Ok(pid) => pid,
            Err(e) => {
                self.tracker.mark_failed(&handle.id, format!("Failed to spawn process: {}", e)).await;
                return;
            }
        };

        // Update execution state with PID
        self.tracker.update_pid(&handle.id, pid).await;

        // Wait for process with timeout
        let timeout = exec_config.timeout.unwrap_or(self.config.default_timeout);
        let result = self.executor.wait_with_timeout(pid, timeout).await;

        let execution_time = start_time.elapsed();
        let output = output_capture.get_output().await;

        // Process the result
        match result {
            Ok(process_result) if process_result.timed_out => {
                self.tracker.mark_failed(&handle.id, format!("Process timed out after {:?}", timeout)).await;
            }
            Ok(process_result) if process_result.success => {
                let work_result = self.create_success_result(&output, execution_time);
                self.tracker.mark_completed(&handle.id, work_result).await;
            }
            Ok(process_result) => {
                let error_msg = if output.has_stderr() {
                    output.stderr_string()
                } else {
                    format!("Process exited with code {}", process_result.exit_code)
                };
                self.tracker.mark_failed(&handle.id, error_msg).await;
            }
            Err(e) => {
                self.tracker.mark_failed(&handle.id, format!("Process failed: {}", e)).await;
            }
        }
    }

    /// Extract command and arguments from job payload
    ///
    /// Expects job payload to contain either:
    /// - A "command" string with just the executable name
    /// - A "command" object with "program" and "args" fields
    /// - Legacy: A "command" string that will be parsed (deprecated)
    fn extract_command(&self, job: &Job) -> (String, Vec<String>) {
        // Try structured command format first (recommended)
        if let Some(cmd_value) = job.payload.get("command") {
            // Check for structured command object
            if let Some(cmd_obj) = cmd_value.as_object() {
                if let (Some(program), args) = (
                    cmd_obj.get("program").and_then(|p| p.as_str()),
                    cmd_obj.get("args")
                ) {
                    let args_vec = if let Some(args_array) = args.and_then(|a| a.as_array()) {
                        args_array.iter()
                            .filter_map(|a| a.as_str().map(String::from))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    return (program.to_string(), args_vec);
                }
            }

            // Check for simple command string (executable only, no args)
            if let Some(cmd_str) = cmd_value.as_str() {
                // Validate that the command doesn't contain shell metacharacters
                // This prevents injection attacks
                if Self::is_safe_command(cmd_str) {
                    // For backward compatibility, still support simple whitespace splitting
                    // but only if the command passes safety validation
                    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                    if !parts.is_empty() {
                        let command = parts[0].to_string();
                        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                        return (command, args);
                    }
                }
            }
        }

        // Default: echo the job ID (safe fallback)
        ("echo".to_string(), vec![format!("Processing job {}", job.id)])
    }

    /// Validate that a command string doesn't contain dangerous shell metacharacters
    fn is_safe_command(cmd: &str) -> bool {
        // Reject commands with shell metacharacters that could enable injection
        const DANGEROUS_CHARS: &[char] = &[
            ';', '&', '|', '>', '<', '`', '$', '(', ')', '{', '}',
            '[', ']', '!', '\\', '\n', '\r', '*', '?', '~'
        ];

        // Also reject if command starts with . or / (path traversal)
        if cmd.starts_with('.') || cmd.starts_with('/') {
            return false;
        }

        // Check for dangerous characters
        !cmd.chars().any(|c| DANGEROUS_CHARS.contains(&c))
    }

    /// Create success work result from captured output
    fn create_success_result(&self, output: &CapturedOutput, execution_time: Duration) -> WorkResult {
        let output_json = if output.has_stdout() {
            serde_json::json!({
                "stdout": output.stdout,
                "stderr": output.stderr,
                "exit_code": 0,
                "execution_time_ms": execution_time.as_millis()
            })
        } else {
            serde_json::json!({
                "exit_code": 0,
                "execution_time_ms": execution_time.as_millis()
            })
        };

        WorkResult {
            success: true,
            output: Some(output_json),
            error: None,
        }
    }
}

#[async_trait]
impl ExecutionBackend for LocalProcessAdapter {
    fn backend_type(&self) -> &str {
        "local"
    }

    async fn submit_job(&self, job: Job, config: ExecutionConfig) -> Result<ExecutionHandle> {
        info!("Local adapter: submitting job {}", job.id);

        // Check concurrent limit
        let running_count = self.tracker.count_running().await;
        if running_count >= self.config.max_concurrent {
            return Err(anyhow::anyhow!("Maximum concurrent processes reached"));
        }

        // Create execution handle
        let handle = ExecutionHandle::local_process(0, job.id.clone());

        // Create execution state
        self.tracker.create_execution(handle.clone()).await;

        // Spawn background task to execute the process
        let self_clone = Arc::new(Self {
            config: self.config.clone(),
            tracker: ExecutionTracker::new(),
            executor: ProcessExecutor::new(),
        });
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            self_clone.execute_process(handle_clone, job, config).await;
        });

        info!("Local adapter: job submitted as {}", handle.id);
        Ok(handle)
    }

    async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus> {
        self.tracker
            .get_status(&handle.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))
    }

    async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()> {
        let state = self.tracker
            .get_state(&handle.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))?;

        if matches!(state.status, ExecutionStatus::Running) {
            // Kill the process
            self.executor.kill_process(state.pid).await?;
            self.tracker.mark_cancelled(&handle.id).await;
            info!("Local adapter: cancelled execution {}", handle.id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Execution is not running"))
        }
    }

    async fn get_metadata(&self, handle: &ExecutionHandle) -> Result<ExecutionMetadata> {
        let state = self.tracker
            .get_state(&handle.id)
            .await
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))?;

        let mut custom = HashMap::new();
        custom.insert("pid".to_string(), serde_json::json!(state.pid));
        custom.insert(
            "work_dir".to_string(),
            serde_json::json!(self.config.work_dir.to_string_lossy()),
        );

        Ok(ExecutionMetadata {
            execution_id: handle.id.clone(),
            backend_type: "local".to_string(),
            started_at: state.started_at,
            completed_at: state.completed_at,
            node_id: Some("localhost".to_string()),
            custom,
        })
    }

    async fn wait_for_completion(
        &self,
        handle: &ExecutionHandle,
        timeout: Option<Duration>,
    ) -> Result<ExecutionStatus> {
        let start = std::time::Instant::now();
        let timeout = timeout.unwrap_or(self.config.default_timeout);

        loop {
            let status = self.get_status(handle).await?;
            match status {
                ExecutionStatus::Completed(_)
                | ExecutionStatus::Failed(_)
                | ExecutionStatus::Cancelled => {
                    return Ok(status);
                }
                _ => {
                    if start.elapsed() >= timeout {
                        return Err(anyhow::anyhow!("Timeout waiting for completion"));
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn health_check(&self) -> Result<BackendHealth> {
        let running_count = self.tracker.count_running().await;

        Ok(BackendHealth {
            healthy: true,
            status_message: format!(
                "Local process adapter healthy: {} processes running",
                running_count
            ),
            resource_info: Some(self.get_resource_info().await?),
            last_check: Some(current_timestamp_or_zero() as u64),
            details: HashMap::from([
                ("type".to_string(), "local".to_string()),
                ("work_dir".to_string(), self.config.work_dir.to_string_lossy().to_string()),
                ("max_concurrent".to_string(), self.config.max_concurrent.to_string()),
            ]),
        })
    }

    async fn get_resource_info(&self) -> Result<ResourceInfo> {
        let running_count = self.tracker.count_running().await;

        let total_cpus = num_cpus::get() as f32;
        let total_memory = 16384; // Default 16GB

        Ok(ResourceInfo {
            total_cpu_cores: total_cpus,
            available_cpu_cores: total_cpus - (running_count as f32 * 0.1),
            total_memory_mb: total_memory,
            available_memory_mb: total_memory - (running_count as u32 * 50),
            total_disk_mb: 100000,
            available_disk_mb: 80000,
            running_executions: running_count,
            max_executions: self.config.max_concurrent,
        })
    }

    async fn can_handle(&self, _job: &Job, _config: &ExecutionConfig) -> Result<bool> {
        let running_count = self.tracker.count_running().await;
        Ok(running_count < self.config.max_concurrent)
    }

    async fn list_executions(&self) -> Result<Vec<ExecutionHandle>> {
        Ok(self.tracker.list_handles().await)
    }

    async fn cleanup_executions(&self, older_than: Duration) -> Result<usize> {
        let cleaned = self.tracker.cleanup_old(older_than).await;
        info!("Local adapter: cleaned up {} old executions", cleaned);
        Ok(cleaned)
    }

    async fn initialize(&self) -> Result<()> {
        info!("Initializing local process adapter");

        // Create work directory if it doesn't exist
        if !self.config.work_dir.exists() {
            tokio::fs::create_dir_all(&self.config.work_dir).await?;
        }

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down local process adapter");

        // Kill all running processes
        let count = self.executor.kill_all().await;
        info!("Killed {} running processes", count);

        Ok(())
    }
}
