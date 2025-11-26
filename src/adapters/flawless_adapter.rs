//! Flawless WASM execution backend adapter

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::domain::types::Job;
use crate::worker_flawless::FlawlessWorker;
use crate::worker_trait::WorkerBackend;

use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};

/// Adapter that wraps the existing FlawlessWorker to implement ExecutionBackend
pub struct FlawlessAdapter {
    worker: Arc<FlawlessWorker>,
    executions: Arc<RwLock<HashMap<String, FlawlessExecutionState>>>,
    execution_counter: Arc<RwLock<u64>>,
    max_concurrent: usize,
}

#[derive(Debug, Clone)]
struct FlawlessExecutionState {
    handle: ExecutionHandle,
    job: Job,
    status: ExecutionStatus,
    started_at: u64,
    completed_at: Option<u64>,
}

impl FlawlessAdapter {
    /// Create a new Flawless adapter
    pub async fn new(flawless_url: &str, max_concurrent: usize) -> Result<Self> {
        let worker = Arc::new(FlawlessWorker::new(flawless_url).await?);

        Ok(Self {
            worker,
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_counter: Arc::new(RwLock::new(0)),
            max_concurrent,
        })
    }

    /// Create a Flawless adapter with default settings
    pub async fn default() -> Result<Self> {
        Self::new("http://localhost:27288", 10).await
    }

    /// Generate a unique execution ID
    async fn generate_id(&self) -> String {
        let mut counter = self.execution_counter.write().await;
        *counter += 1;
        format!("flawless-exec-{}", *counter)
    }

    /// Execute job in background
    async fn execute_job_async(&self, handle: ExecutionHandle, job: Job) {
        // Execute the job using the FlawlessWorker
        let result = self.worker.execute(job.clone()).await;

        // Update execution state based on result
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            let completed_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_secs();

            match result {
                Ok(work_result) => {
                    if work_result.success {
                        state.status = ExecutionStatus::Completed(work_result);
                    } else {
                        state.status = ExecutionStatus::Failed(
                            work_result.error.unwrap_or_else(|| "Unknown error".to_string())
                        );
                    }
                }
                Err(e) => {
                    state.status = ExecutionStatus::Failed(e.to_string());
                }
            }
            state.completed_at = Some(completed_at);
        }
    }
}

#[async_trait]
impl ExecutionBackend for FlawlessAdapter {
    fn backend_type(&self) -> &str {
        "flawless"
    }

    async fn submit_job(&self, job: Job, _config: ExecutionConfig) -> Result<ExecutionHandle> {
        info!("Flawless adapter: submitting job {}", job.id);

        // Check concurrent execution limit
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        if running_count >= self.max_concurrent {
            return Err(anyhow::anyhow!("Maximum concurrent executions reached"));
        }
        drop(executions);

        // Generate execution handle
        let exec_id = self.generate_id().await;
        let handle = ExecutionHandle::flawless(exec_id.clone(), job.id.clone());

        // Create execution record
        let state = FlawlessExecutionState {
            handle: handle.clone(),
            job: job.clone(),
            status: ExecutionStatus::Running,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_secs(),
            completed_at: None,
        };

        // Store execution state
        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), state);
        drop(executions);

        // Spawn background task to execute the job
        let self_clone = Arc::new(Self {
            worker: self.worker.clone(),
            executions: self.executions.clone(),
            execution_counter: self.execution_counter.clone(),
            max_concurrent: self.max_concurrent,
        });
        let handle_clone = handle.clone();
        let job_clone = job.clone();
        tokio::spawn(async move {
            self_clone.execute_job_async(handle_clone, job_clone).await;
        });

        info!("Flawless adapter: job {} submitted as {}", job.id, handle.id);
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
                // Note: FlawlessWorker doesn't provide cancellation support
                // We can only mark it as cancelled in our state
                warn!("Flawless adapter: cancellation requested but Flawless doesn't support cancellation");
                state.status = ExecutionStatus::Cancelled;
                state.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("System time is before UNIX epoch")
                        .as_secs(),
                );

                info!("Flawless adapter: marked execution {} as cancelled", handle.id);
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
        if let Some(url) = state.job.payload.get("url") {
            custom.insert(
                "url".to_string(),
                url.clone(),
            );
        }

        Ok(ExecutionMetadata {
            execution_id: handle.id.clone(),
            backend_type: "flawless".to_string(),
            started_at: state.started_at,
            completed_at: state.completed_at,
            node_id: Some("flawless-runtime".to_string()),
            custom,
        })
    }

    async fn wait_for_completion(
        &self,
        handle: &ExecutionHandle,
        timeout: Option<Duration>,
    ) -> Result<ExecutionStatus> {
        let start = std::time::Instant::now();
        let timeout = timeout.unwrap_or(Duration::from_secs(300)); // 5 min default

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
                    tokio::time::sleep(Duration::from_millis(500)).await;
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
        let total_count = executions.len();

        Ok(BackendHealth {
            healthy: true,
            status_message: format!(
                "Flawless adapter healthy: {} executions running, {} total",
                running_count, total_count
            ),
            resource_info: Some(self.get_resource_info().await?),
            last_check: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("System time is before UNIX epoch")
                    .as_secs(),
            ),
            details: HashMap::from([
                ("type".to_string(), "flawless".to_string()),
                ("runtime".to_string(), "wasm".to_string()),
                ("max_concurrent".to_string(), self.max_concurrent.to_string()),
            ]),
        })
    }

    async fn get_resource_info(&self) -> Result<ResourceInfo> {
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        // Flawless doesn't expose detailed resource info, so we provide estimates
        Ok(ResourceInfo {
            total_cpu_cores: 8.0, // Estimate based on typical WASM runtime
            available_cpu_cores: 8.0 - (running_count as f32 * 0.5), // Assume 0.5 cores per workflow
            total_memory_mb: 4096, // 4 GB for WASM runtime
            available_memory_mb: 4096 - (running_count as u32 * 128), // Assume 128MB per workflow
            total_disk_mb: 10000, // 10 GB
            available_disk_mb: 8000, // 8 GB available
            running_executions: running_count,
            max_executions: self.max_concurrent,
        })
    }

    async fn can_handle(&self, job: &Job, _config: &ExecutionConfig) -> Result<bool> {
        // Check if this is a Flawless-compatible job
        // Flawless jobs should have 'id' and 'url' in payload
        let has_required_fields = job.payload.get("id").is_some()
            && job.payload.get("url").is_some();

        if !has_required_fields {
            return Ok(false);
        }

        // Check concurrent limit
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();

        Ok(running_count < self.max_concurrent)
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
            .expect("System time is before UNIX epoch")
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

        info!("Flawless adapter: cleaned up {} old executions", cleaned);
        Ok(cleaned)
    }

    async fn initialize(&self) -> Result<()> {
        info!("Initializing Flawless adapter");
        // FlawlessWorker is already initialized in new()
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Flawless adapter");

        // Wait for running executions to complete (with timeout)
        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();

        loop {
            let executions = self.executions.read().await;
            let running_count = executions
                .values()
                .filter(|state| matches!(state.status, ExecutionStatus::Running))
                .count();
            drop(executions);

            if running_count == 0 {
                break;
            }

            if start.elapsed() >= timeout {
                warn!("Timeout waiting for Flawless executions to complete");
                break;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}
