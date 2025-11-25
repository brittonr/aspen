//! Local process execution backend for development and testing

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::domain::types::Job;
use crate::worker_trait::WorkResult;

use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};

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
            default_timeout: Duration::from_secs(300),
            work_dir: std::env::temp_dir().join("blixard-local"),
            capture_output: true,
        }
    }
}

/// State of a local process execution
#[derive(Debug, Clone)]
struct LocalProcessState {
    handle: ExecutionHandle,
    job: Job,
    pid: u32,
    status: ExecutionStatus,
    started_at: u64,
    completed_at: Option<u64>,
    output: Vec<String>,
    errors: Vec<String>,
}

/// Local process execution backend
pub struct LocalProcessAdapter {
    config: LocalProcessConfig,
    executions: Arc<RwLock<HashMap<String, LocalProcessState>>>,
    processes: Arc<RwLock<HashMap<u32, Child>>>,
}

impl LocalProcessAdapter {
    /// Create a new local process adapter
    pub fn new(config: LocalProcessConfig) -> Self {
        Self {
            config,
            executions: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(LocalProcessConfig::default())
    }

    /// Execute a job as a local process
    async fn execute_process(&self, handle: ExecutionHandle, job: Job, config: ExecutionConfig) {
        let start_time = std::time::Instant::now();

        // Extract command from job payload
        let (command, args) = self.extract_command(&job);

        debug!(
            "Local adapter: executing command '{}' with args {:?}",
            command, args
        );

        // Create command
        let mut cmd = Command::new(&command);
        cmd.args(&args);

        // Set working directory
        if !self.config.work_dir.exists() {
            if let Err(e) = tokio::fs::create_dir_all(&self.config.work_dir).await {
                self.mark_failed(&handle, format!("Failed to create work directory: {}", e))
                    .await;
                return;
            }
        }
        cmd.current_dir(&self.config.work_dir);

        // Set environment variables from config
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Configure output capture
        if self.config.capture_output {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        // Spawn the process
        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                self.mark_failed(&handle, format!("Failed to spawn process: {}", e))
                    .await;
                return;
            }
        };

        let pid = child.id().unwrap_or(0);

        // Store the child process
        let mut processes = self.processes.write().await;
        processes.insert(pid, child);
        drop(processes);

        // Update execution state with PID
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            state.pid = pid;
        }
        drop(executions);

        // Get the child back
        let mut processes = self.processes.write().await;
        let mut child = processes.remove(&pid).unwrap();
        drop(processes);

        // Capture output if configured
        let output_lines = Arc::new(RwLock::new(Vec::new()));
        let error_lines = Arc::new(RwLock::new(Vec::new()));

        if self.config.capture_output {
            // Spawn tasks to capture stdout and stderr
            if let Some(stdout) = child.stdout.take() {
                let output_clone = output_lines.clone();
                tokio::spawn(async move {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let mut output = output_clone.write().await;
                        output.push(line);
                    }
                });
            }

            if let Some(stderr) = child.stderr.take() {
                let error_clone = error_lines.clone();
                tokio::spawn(async move {
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let mut errors = error_clone.write().await;
                        errors.push(line);
                    }
                });
            }
        }

        // Wait for process with timeout
        let timeout = config.timeout.unwrap_or(self.config.default_timeout);
        let result = tokio::time::timeout(timeout, child.wait()).await;

        let execution_time = start_time.elapsed();

        // Process the result
        match result {
            Ok(Ok(exit_status)) => {
                let output = output_lines.read().await.clone();
                let errors = error_lines.read().await.clone();

                if exit_status.success() {
                    let output_json = if !output.is_empty() {
                        serde_json::json!({
                            "stdout": output,
                            "stderr": errors,
                            "exit_code": 0,
                            "execution_time_ms": execution_time.as_millis()
                        })
                    } else {
                        serde_json::json!({
                            "exit_code": 0,
                            "execution_time_ms": execution_time.as_millis()
                        })
                    };

                    self.mark_completed(
                        &handle,
                        WorkResult {
                            success: true,
                            output: Some(output_json),
                            error: None,
                        },
                    )
                    .await;
                } else {
                    let exit_code = exit_status.code().unwrap_or(-1);
                    let error_msg = if !errors.is_empty() {
                        errors.join("\n")
                    } else {
                        format!("Process exited with code {}", exit_code)
                    };

                    self.mark_failed(&handle, error_msg).await;
                }
            }
            Ok(Err(e)) => {
                self.mark_failed(&handle, format!("Process failed: {}", e))
                    .await;
            }
            Err(_) => {
                // Timeout - try to kill the process
                if let Err(e) = child.kill().await {
                    warn!("Failed to kill timed-out process: {}", e);
                }
                self.mark_failed(&handle, format!("Process timed out after {:?}", timeout))
                    .await;
            }
        }

        // Clean up process handle
        let mut processes = self.processes.write().await;
        processes.remove(&pid);
    }

    /// Extract command and arguments from job payload
    fn extract_command(&self, job: &Job) -> (String, Vec<String>) {
        // Try to get command from payload
        if let Some(cmd_value) = job.payload.get("command") {
            if let Some(cmd_str) = cmd_value.as_str() {
                // Split command into program and args
                let parts: Vec<&str> = cmd_str.split_whitespace().collect();
                if !parts.is_empty() {
                    let command = parts[0].to_string();
                    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                    return (command, args);
                }
            }
        }

        // Default: echo the job ID
        ("echo".to_string(), vec![format!("Processing job {}", job.id)])
    }

    /// Mark an execution as completed
    async fn mark_completed(&self, handle: &ExecutionHandle, result: WorkResult) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            state.status = ExecutionStatus::Completed(result);
            state.completed_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
        }
    }

    /// Mark an execution as failed
    async fn mark_failed(&self, handle: &ExecutionHandle, error: String) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            state.status = ExecutionStatus::Failed(error);
            state.completed_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
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
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        if running_count >= self.config.max_concurrent {
            return Err(anyhow::anyhow!("Maximum concurrent processes reached"));
        }
        drop(executions);

        // Create execution handle
        let handle = ExecutionHandle::local_process(0, job.id.clone()); // PID will be updated

        // Create execution state
        let state = LocalProcessState {
            handle: handle.clone(),
            job: job.clone(),
            pid: 0, // Will be updated when process starts
            status: ExecutionStatus::Running,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            completed_at: None,
            output: Vec::new(),
            errors: Vec::new(),
        };

        // Store execution state
        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), state);
        drop(executions);

        // Spawn background task to execute the process
        let self_clone = Arc::new(Self {
            config: self.config.clone(),
            executions: self.executions.clone(),
            processes: self.processes.clone(),
        });
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            self_clone.execute_process(handle_clone, job, config).await;
        });

        info!("Local adapter: job submitted as {}", handle.id);
        Ok(handle)
    }

    async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus> {
        let executions = self.executions.read().await;
        executions
            .get(&handle.id)
            .map(|state| state.status.clone())
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))
    }

    async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            if matches!(state.status, ExecutionStatus::Running) {
                // Kill the process
                let mut processes = self.processes.write().await;
                if let Some(mut child) = processes.remove(&state.pid) {
                    if let Err(e) = child.kill().await {
                        warn!("Failed to kill process {}: {}", state.pid, e);
                    }
                }
                drop(processes);

                state.status = ExecutionStatus::Cancelled;
                state.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );

                info!("Local adapter: cancelled execution {}", handle.id);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Execution is not running"))
            }
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", handle.id))
        }
    }

    async fn get_metadata(&self, handle: &ExecutionHandle) -> Result<ExecutionMetadata> {
        let executions = self.executions.read().await;
        let state = executions
            .get(&handle.id)
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))?;

        let mut custom = HashMap::new();
        custom.insert(
            "pid".to_string(),
            serde_json::json!(state.pid),
        );
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
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        Ok(BackendHealth {
            healthy: true,
            status_message: format!(
                "Local process adapter healthy: {} processes running",
                running_count
            ),
            resource_info: Some(self.get_resource_info().await?),
            last_check: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            details: HashMap::from([
                ("type".to_string(), "local".to_string()),
                ("work_dir".to_string(), self.config.work_dir.to_string_lossy().to_string()),
                ("max_concurrent".to_string(), self.config.max_concurrent.to_string()),
            ]),
        })
    }

    async fn get_resource_info(&self) -> Result<ResourceInfo> {
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        // Get system info (simplified)
        let total_cpus = num_cpus::get() as f32;
        let total_memory = 16384; // Default 16GB, could use sys-info crate for real value

        Ok(ResourceInfo {
            total_cpu_cores: total_cpus,
            available_cpu_cores: total_cpus - (running_count as f32 * 0.1), // Assume 0.1 core per process
            total_memory_mb: total_memory,
            available_memory_mb: total_memory - (running_count as u32 * 50), // Assume 50MB per process
            total_disk_mb: 100000, // 100GB
            available_disk_mb: 80000, // 80GB available
            running_executions: running_count,
            max_executions: self.config.max_concurrent,
        })
    }

    async fn can_handle(&self, _job: &Job, _config: &ExecutionConfig) -> Result<bool> {
        // Check concurrent limit
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        Ok(running_count < self.config.max_concurrent)
    }

    async fn list_executions(&self) -> Result<Vec<ExecutionHandle>> {
        let executions = self.executions.read().await;
        Ok(executions
            .values()
            .map(|state| state.handle.clone())
            .collect())
    }

    async fn cleanup_executions(&self, older_than: Duration) -> Result<usize> {
        let mut executions = self.executions.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff = now - older_than.as_secs();
        let mut cleaned = 0;

        executions.retain(|_, state| {
            if let Some(completed_at) = state.completed_at {
                if completed_at < cutoff {
                    cleaned += 1;
                    return false;
                }
            }
            true
        });

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
        let mut processes = self.processes.write().await;
        for (pid, mut child) in processes.drain() {
            info!("Killing process {}", pid);
            if let Err(e) = child.kill().await {
                error!("Failed to kill process {}: {}", pid, e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_adapter_creation() {
        let adapter = LocalProcessAdapter::default();
        assert_eq!(adapter.backend_type(), "local");

        let health = adapter.health_check().await.unwrap();
        assert!(health.healthy);
    }

    #[tokio::test]
    async fn test_local_adapter_echo() {
        let adapter = LocalProcessAdapter::default();
        adapter.initialize().await.unwrap();

        let job = crate::domain::types::Job {
            id: "test-echo".to_string(),
            payload: serde_json::json!({
                "command": "echo hello world"
            }),
            ..Default::default()
        };

        let config = ExecutionConfig::default();
        let handle = adapter.submit_job(job, config).await.unwrap();

        let status = adapter
            .wait_for_completion(&handle, Some(Duration::from_secs(5)))
            .await
            .unwrap();

        assert!(matches!(status, ExecutionStatus::Completed(_)));
    }
}