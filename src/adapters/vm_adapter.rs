//! VM execution backend adapter wrapping the existing VmManager
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use crate::common::get_unix_timestamp_or_zero;
use crate::domain::types::Job;
use crate::infrastructure::vm::{VmAssignment, VmManagement, VmManager, VmManagerConfig};
use crate::worker_trait::WorkResult;
use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};
use super::cleanup::{CleanupConfig, CleanupMetrics, CleanableExecution, cleanup_executions};

/// Adapter that wraps the VM management trait to implement ExecutionBackend
pub struct VmAdapter {
    vm_manager: Arc<dyn VmManagement>,
    // Track execution handles to VM IDs
    executions: Arc<tokio::sync::RwLock<HashMap<String, VmExecutionState>>>,
    // Limit concurrent monitoring tasks to prevent resource exhaustion
    monitor_semaphore: Arc<tokio::sync::Semaphore>,
    // Cleanup configuration
    cleanup_config: CleanupConfig,
    // Cleanup metrics
    cleanup_metrics: Arc<tokio::sync::RwLock<CleanupMetrics>>,
    // Background cleanup task handle
    cleanup_task_handle: Arc<tokio::sync::RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

#[derive(Debug, Clone)]
struct VmExecutionState {
    handle: ExecutionHandle,
    vm_id: uuid::Uuid,
    assignment: VmAssignment,
    status: ExecutionStatus,
    started_at: u64,
    completed_at: Option<u64>,
    last_accessed: u64,
}

impl CleanableExecution for VmExecutionState {
    fn completed_at(&self) -> Option<u64> {
        self.completed_at
    }

    fn last_accessed(&self) -> u64 {
        self.last_accessed
    }

    fn status(&self) -> &ExecutionStatus {
        &self.status
    }
}
impl VmAdapter {
    /// Create a new VM adapter wrapping a VM management implementation
    pub fn new(vm_manager: Arc<dyn VmManagement>) -> Self {
        Self::with_cleanup_config(vm_manager, CleanupConfig::default())
    }

    /// Create a new VM adapter with custom cleanup configuration
    pub fn with_cleanup_config(vm_manager: Arc<dyn VmManagement>, cleanup_config: CleanupConfig) -> Self {
        // Limit to 1000 concurrent monitoring tasks (prevent unbounded spawning)
        const MAX_CONCURRENT_MONITORS: usize = 1000;

        let adapter = Self {
            vm_manager,
            executions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            monitor_semaphore: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_MONITORS)),
            cleanup_config: cleanup_config.clone(),
            cleanup_metrics: Arc::new(tokio::sync::RwLock::new(CleanupMetrics::new())),
            cleanup_task_handle: Arc::new(tokio::sync::RwLock::new(None)),
        };

        // Start background cleanup if enabled
        if cleanup_config.enable_background_cleanup {
            adapter.start_background_cleanup();
        }

        adapter
    }

    /// Create a VM adapter with a new VmManager
    pub async fn create(
        config: VmManagerConfig,
        hiqlite: Arc<crate::hiqlite::HiqliteService>,
    ) -> Result<Self> {
        let vm_manager: Arc<dyn VmManagement> = Arc::new(VmManager::new(config, hiqlite).await?);
        Ok(Self::new(vm_manager))
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

                debug!("Running background cleanup for VM adapter");
                let start = std::time::Instant::now();

                let (ttl_count, lru_count) = Self::cleanup_internal(&executions, &config).await;

                let duration = start.elapsed();

                if ttl_count > 0 || lru_count > 0 {
                    info!(
                        "VM adapter cleanup: {} TTL expired, {} LRU evicted in {:?}",
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

    /// Internal cleanup implementation using shared cleanup logic
    async fn cleanup_internal(
        executions: &Arc<tokio::sync::RwLock<HashMap<String, VmExecutionState>>>,
        config: &CleanupConfig,
    ) -> (usize, usize) {
        let mut exec_map = executions.write().await;
        let now = get_unix_timestamp_or_zero();

        cleanup_executions(&mut *exec_map, config, now)
    }

    /// Get cleanup metrics
    pub async fn get_cleanup_metrics(&self) -> CleanupMetrics {
        let metrics = self.cleanup_metrics.read().await;
        metrics.clone()
    }

    /// Helper method to finalize execution state with proper timestamp handling
    async fn finalize_execution(&self, handle_id: &str, status: ExecutionStatus) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = status;
            state.completed_at = Some(get_unix_timestamp_or_zero());
        }
    }

    /// Monitor VM job execution
    async fn monitor_execution(&self, handle: ExecutionHandle, vm_id: uuid::Uuid) {
        use tracing::{debug, warn, error};

        debug!("Starting job monitoring for handle {} on VM {}", handle.id, vm_id);

        // Monitor with exponential backoff
        let mut check_interval = Duration::from_secs(1);
        let max_interval = Duration::from_secs(10);
        let timeout = super::DEFAULT_EXECUTION_TIMEOUT;
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                warn!("Job {} timed out on VM {}", handle.id, vm_id);
                self.finalize_execution(
                    &handle.id,
                    ExecutionStatus::Failed(format!("Job execution timed out after {:?}", timeout))
                ).await;
                break;
            }

            // Check VM health
            match self.vm_manager.get_health_status(vm_id).await {
                Ok(health) if health.is_healthy() => {
                    debug!("VM {} is healthy, job still running", vm_id);
                }
                Ok(health) if !health.is_healthy() => {
                    warn!("VM {} is unhealthy: {:?}", vm_id, health);
                    self.finalize_execution(
                        &handle.id,
                        ExecutionStatus::Failed(format!("VM became unhealthy: {:?}", health))
                    ).await;
                    break;
                }
                _ => {
                    // VM health unknown, continue monitoring
                }
            }

            // Check VM status
            match self.vm_manager.get_vm_status(vm_id).await {
                Ok(Some(vm_state)) if vm_state.is_running() => {
                    // VM is still running, continue monitoring
                    debug!("VM {} is running, continuing to monitor", vm_id);
                }
                Ok(Some(vm_state)) if vm_state.is_stopped() => {
                    // VM has stopped, assume job completed
                    info!("VM {} has stopped, marking job as completed", vm_id);
                    self.finalize_execution(
                        &handle.id,
                        ExecutionStatus::Completed(WorkResult {
                            success: true,
                            output: Some(serde_json::Value::String(format!(
                                "Job completed on VM {} (VM stopped)",
                                vm_id
                            ))),
                            error: None,
                        })
                    ).await;
                    break;
                }
                Ok(Some(vm_state)) if vm_state.is_failed() => {
                    error!("VM {} failed", vm_id);
                    self.finalize_execution(
                        &handle.id,
                        ExecutionStatus::Failed(format!("VM {} failed", vm_id))
                    ).await;
                    break;
                }
                Ok(None) => {
                    warn!("VM {} not found, marking job as failed", vm_id);
                    self.finalize_execution(
                        &handle.id,
                        ExecutionStatus::Failed(format!("VM {} not found", vm_id))
                    ).await;
                    break;
                }
                Err(e) => {
                    warn!("Error checking VM {} status: {}", vm_id, e);
                }
                _ => {}
            }

            // Wait before next check with exponential backoff
            tokio::time::sleep(check_interval).await;
            check_interval = (check_interval * 2).min(max_interval);
        }

        debug!("Finished monitoring job {} on VM {}", handle.id, vm_id);
    }
}
#[async_trait]
impl ExecutionBackend for VmAdapter {
    fn backend_type(&self) -> &str {
        "vm"
    }
    async fn submit_job(&self, job: Job, _config: ExecutionConfig) -> Result<ExecutionHandle> {
        info!("VM adapter: submitting job {}", job.id);
        // Route job through VmManager
        let assignment = self.vm_manager.execute_job(job.clone()).await?;
        let vm_id = assignment.vm_id();
        // Create execution handle
        let handle = ExecutionHandle::vm(vm_id.to_string(), job.id.clone());
        // Store execution state
        let now = get_unix_timestamp_or_zero();
        let state = VmExecutionState {
            handle: handle.clone(),
            vm_id,
            assignment,
            status: ExecutionStatus::Running,
            started_at: now,
            completed_at: None,
            last_accessed: now,
        };
        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), state);
        drop(executions);

        // Start monitoring task (with semaphore to limit concurrency)
        let self_clone = Arc::new(Self {
            vm_manager: self.vm_manager.clone(),
            executions: self.executions.clone(),
            monitor_semaphore: self.monitor_semaphore.clone(),
            cleanup_config: self.cleanup_config.clone(),
            cleanup_metrics: self.cleanup_metrics.clone(),
            cleanup_task_handle: Arc::new(tokio::sync::RwLock::new(None)),
        });
        let handle_clone = handle.clone();
        let semaphore = self.monitor_semaphore.clone();

        tokio::spawn(async move {
            // Acquire permit (blocks if limit reached)
            // Handle semaphore closure gracefully (can happen during shutdown)
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    tracing::warn!(
                        job_id = %handle_clone.id,
                        "Monitor semaphore closed, shutting down monitor gracefully"
                    );
                    return;
                }
            };
            self_clone.monitor_execution(handle_clone, vm_id).await;
            // Permit automatically released when dropped
        });

        info!("VM adapter: job {} assigned to VM {}", job.id, vm_id);
        Ok(handle)
    }
    async fn get_status(&self, handle: &ExecutionHandle) -> Result<ExecutionStatus> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            state.last_accessed = get_unix_timestamp_or_zero();
            Ok(state.status.clone())
        } else {
            Err(anyhow::anyhow!("Execution not found: {}", handle.id))
        }
    }
    async fn cancel_execution(&self, handle: &ExecutionHandle) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            if matches!(state.status, ExecutionStatus::Running) {
                // In a real implementation, we would send a cancellation signal to the VM
                // For now, just mark as cancelled
                state.status = ExecutionStatus::Cancelled;
                state.completed_at = Some(get_unix_timestamp_or_zero());
                info!("VM adapter: cancelled execution {}", handle.id);
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

        state.last_accessed = get_unix_timestamp_or_zero();
        let mut custom = HashMap::new();
        custom.insert(
            "vm_id".to_string(),
            serde_json::json!(state.vm_id.to_string()),
        );
        custom.insert(
            "assignment_type".to_string(),
            serde_json::json!(match &state.assignment {
                VmAssignment::Ephemeral(_) => "ephemeral",
                VmAssignment::Service(_) => "service",
            }),
        );
        Ok(ExecutionMetadata {
            execution_id: handle.id.clone(),
            backend_type: "vm".to_string(),
            started_at: state.started_at,
            completed_at: state.completed_at,
            node_id: Some("control-plane".to_string()),
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
        // Get VM statistics
        let stats = self.vm_manager.get_stats().await?;
        Ok(BackendHealth {
            healthy: true,
            status_message: format!(
                "VM backend healthy: {} VMs running, {} idle",
                stats.running_vms, stats.idle_vms
            ),
            resource_info: Some(self.get_resource_info().await?),
            last_check: Some(get_unix_timestamp_or_zero()),
            details: HashMap::from([
                ("type".to_string(), "vm".to_string()),
                ("total_vms".to_string(), stats.total_vms.to_string()),
                ("running_vms".to_string(), stats.running_vms.to_string()),
                ("idle_vms".to_string(), stats.idle_vms.to_string()),
                ("failed_vms".to_string(), stats.failed_vms.to_string()),
            ]),
        })
    }
    async fn get_resource_info(&self) -> Result<ResourceInfo> {
        let stats = self.vm_manager.get_stats().await?;
        let executions = self.executions.read().await;
        let running_count = executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count();
        // These are placeholder values - in a real implementation,
        // we would query the actual resource usage from the VMs
        Ok(ResourceInfo {
            total_cpu_cores: 32.0, // Example: 32 vCPUs total
            available_cpu_cores: 32.0 - (stats.running_vms as f32 * 2.0), // Assume 2 vCPUs per VM
            total_memory_mb: 65536, // 64 GB total
            available_memory_mb: 65536 - (stats.running_vms as u32 * 512), // Assume 512MB per VM
            total_disk_mb: 500000, // 500 GB
            available_disk_mb: 400000, // 400 GB available
            running_executions: running_count,
            max_executions: 20, // From VmManagerConfig default
        })
    }
    async fn can_handle(&self, _job: &Job, _config: &ExecutionConfig) -> Result<bool> {
        // Check if we have capacity for another VM
        let stats = self.vm_manager.get_stats().await?;
        // Simple check: can we start another VM?
        // In a real implementation, we would check resource requirements
        Ok(stats.total_vms < 20) // Max VMs from default config
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
        let now = get_unix_timestamp_or_zero();
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
        info!("VM adapter: cleaned up {} old executions", cleaned);
        Ok(cleaned)
    }
    async fn initialize(&self) -> Result<()> {
        info!("Initializing VM adapter");
        self.vm_manager.start().await?;
        Ok(())
    }
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down VM adapter");

        // Stop background cleanup task
        let mut handle = self.cleanup_task_handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Stopped VM adapter background cleanup task");
        }
        drop(handle);

        self.vm_manager.shutdown().await?;
        Ok(())
    }
}
