//! Flawless WASM execution backend adapter

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::domain::types::Job;
use crate::worker_flawless::FlawlessWorker;
use crate::worker_trait::WorkerBackend;

use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};
use super::cleanup::{CleanupConfig, CleanupMetrics};

/// Adapter that wraps the existing FlawlessWorker to implement ExecutionBackend
pub struct FlawlessAdapter {
    worker: Arc<FlawlessWorker>,
    executions: Arc<RwLock<HashMap<String, FlawlessExecutionState>>>,
    execution_counter: Arc<RwLock<u64>>,
    max_concurrent: usize,
    cleanup_config: CleanupConfig,
    cleanup_metrics: Arc<RwLock<CleanupMetrics>>,
    cleanup_task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

#[derive(Debug, Clone)]
struct FlawlessExecutionState {
    handle: ExecutionHandle,
    job: Job,
    status: ExecutionStatus,
    started_at: u64,
    completed_at: Option<u64>,
    last_accessed: u64,
}

impl FlawlessAdapter {
    /// Create a new Flawless adapter
    pub async fn new(flawless_url: &str, max_concurrent: usize) -> Result<Self> {
        Self::with_cleanup_config(flawless_url, max_concurrent, CleanupConfig::default()).await
    }

    /// Create a new Flawless adapter with custom cleanup configuration
    pub async fn with_cleanup_config(
        flawless_url: &str,
        max_concurrent: usize,
        cleanup_config: CleanupConfig,
    ) -> Result<Self> {
        let worker = Arc::new(FlawlessWorker::new(flawless_url).await?);

        let adapter = Self {
            worker,
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_counter: Arc::new(RwLock::new(0)),
            max_concurrent,
            cleanup_config: cleanup_config.clone(),
            cleanup_metrics: Arc::new(RwLock::new(CleanupMetrics::new())),
            cleanup_task_handle: Arc::new(RwLock::new(None)),
        };

        // Start background cleanup if enabled
        if cleanup_config.enable_background_cleanup {
            adapter.start_background_cleanup();
        }

        Ok(adapter)
    }

    /// Create a Flawless adapter with default settings
    pub async fn default() -> Result<Self> {
        Self::new("http://localhost:27288", 10).await
    }

    /// Start background cleanup task
    fn start_background_cleanup(&self) {
        let executions = self.executions.clone();
        let config = self.cleanup_config.clone();
        let metrics = self.cleanup_metrics.clone();
        let cleanup_task_handle = self.cleanup_task_handle.clone();

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;

                debug!("Running background cleanup for Flawless adapter");
                let start = std::time::Instant::now();

                let (ttl_count, lru_count) = Self::cleanup_internal(&executions, &config).await;

                let duration = start.elapsed();

                if ttl_count > 0 || lru_count > 0 {
                    info!(
                        "Flawless adapter cleanup: {} TTL expired, {} LRU evicted in {:?}",
                        ttl_count, lru_count, duration
                    );
                }

                // Update metrics
                let mut m = metrics.write().await;
                m.record_cleanup(ttl_count, lru_count, duration);
            }
        });

        // Store task handle
        let handle_clone = cleanup_task_handle.clone();
        tokio::spawn(async move {
            let mut handle = handle_clone.write().await;
            *handle = Some(task);
        });
    }

    /// Internal cleanup implementation
    async fn cleanup_internal(
        executions: &Arc<RwLock<HashMap<String, FlawlessExecutionState>>>,
        config: &CleanupConfig,
    ) -> (usize, usize) {
        let mut exec_map = executions.write().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();
        let mut ttl_cleaned = 0;

        // First pass: Remove entries that exceed TTL based on status
        exec_map.retain(|_, state| {
            if let Some(completed_at) = state.completed_at {
                let age = now.saturating_sub(completed_at);
                let should_remove = match &state.status {
                    ExecutionStatus::Completed(_) => age > config.completed_ttl.as_secs(),
                    ExecutionStatus::Failed(_) => age > config.failed_ttl.as_secs(),
                    ExecutionStatus::Cancelled => age > config.cancelled_ttl.as_secs(),
                    _ => false,
                };

                if should_remove {
                    ttl_cleaned += 1;
                    return false;
                }
            }
            true
        });

        // Second pass: If still over size limit, evict oldest by last_accessed (LRU)
        let mut lru_cleaned = 0;
        if exec_map.len() > config.max_entries {
            let mut entries: Vec<_> = exec_map
                .iter()
                .map(|(k, v)| (k.clone(), v.last_accessed))
                .collect();

            entries.sort_by_key(|(_, last_accessed)| *last_accessed);

            let to_remove = exec_map.len() - config.max_entries;

            for (key, _) in entries.iter().take(to_remove) {
                exec_map.remove(key);
                lru_cleaned += 1;
            }
        }

        (ttl_cleaned, lru_cleaned)
    }

    /// Get cleanup metrics
    pub async fn get_cleanup_metrics(&self) -> CleanupMetrics {
        let metrics = self.cleanup_metrics.read().await;
        metrics.clone()
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

        // Generate execution handle before acquiring lock
        let exec_id = self.generate_id().await;
        let handle = ExecutionHandle::flawless(exec_id.clone(), job.id.clone());

        // Create execution record
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();
        let state = FlawlessExecutionState {
            handle: handle.clone(),
            job: job.clone(),
            status: ExecutionStatus::Running,
            started_at: now,
            completed_at: None,
            last_accessed: now,
        };

        // ATOMIC: Check limit and insert in single critical section (fixes TOCTOU race)
        let mut executions = self.executions.write().await;

        // Count running executions while holding write lock
        let running_count = executions
            .values()
            .filter(|s| matches!(s.status, ExecutionStatus::Running))
            .count();

        if running_count >= self.max_concurrent {
            return Err(anyhow::anyhow!(
                "Maximum concurrent executions reached ({}/{})",
                running_count,
                self.max_concurrent
            ));
        }

        // Insert immediately while still holding lock
        executions.insert(handle.id.clone(), state);
        drop(executions);

        // Spawn background task to execute the job
        let self_clone = Arc::new(Self {
            worker: self.worker.clone(),
            executions: self.executions.clone(),
            execution_counter: self.execution_counter.clone(),
            max_concurrent: self.max_concurrent,
            cleanup_config: self.cleanup_config.clone(),
            cleanup_metrics: self.cleanup_metrics.clone(),
            cleanup_task_handle: Arc::new(RwLock::new(None)),
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
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            state.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_secs();
            Ok(state.status.clone())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", handle.id))
        }
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
        let mut executions = self.executions.write().await;
        let state = executions
            .get_mut(&handle.id)
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))?;

        state.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();

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
        let timeout = timeout.unwrap_or(super::DEFAULT_EXECUTION_TIMEOUT);

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

        // Stop background cleanup task
        let mut handle = self.cleanup_task_handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Stopped Flawless adapter background cleanup task");
        }
        drop(handle);

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
