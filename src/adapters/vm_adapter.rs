//! VM execution backend adapter wrapping the existing VmManager
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;
use crate::domain::types::Job;
use crate::infrastructure::vm::{VmAssignment, VmManager, VmManagerConfig};
use crate::worker_trait::WorkResult;
use super::{
    BackendHealth, ExecutionBackend, ExecutionConfig, ExecutionHandle, ExecutionMetadata,
    ExecutionStatus, ResourceInfo,
};
/// Adapter that wraps the existing VmManager to implement ExecutionBackend
pub struct VmAdapter {
    vm_manager: Arc<VmManager>,
    // Track execution handles to VM IDs
    executions: Arc<tokio::sync::RwLock<HashMap<String, VmExecutionState>>>,
}
#[derive(Debug, Clone)]
struct VmExecutionState {
    handle: ExecutionHandle,
    vm_id: uuid::Uuid,
    assignment: VmAssignment,
    status: ExecutionStatus,
    started_at: u64,
    completed_at: Option<u64>,
}
impl VmAdapter {
    /// Create a new VM adapter wrapping an existing VmManager
    pub fn new(vm_manager: Arc<VmManager>) -> Self {
        Self {
            vm_manager,
            executions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    /// Create a VM adapter with a new VmManager
    pub async fn create(
        config: VmManagerConfig,
        hiqlite: Arc<crate::hiqlite::HiqliteService>,
    ) -> Result<Self> {
        let vm_manager = Arc::new(VmManager::new(config, hiqlite).await?);
        Ok(Self::new(vm_manager))
    }
    /// Monitor VM job execution
    async fn monitor_execution(&self, handle: ExecutionHandle, vm_id: uuid::Uuid) {
        use tracing::{debug, warn, error};

        debug!("Starting job monitoring for handle {} on VM {}", handle.id, vm_id);

        // Monitor with exponential backoff
        let mut check_interval = Duration::from_secs(1);
        let max_interval = Duration::from_secs(10);
        let timeout = Duration::from_secs(300); // 5 minute timeout
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                warn!("Job {} timed out on VM {}", handle.id, vm_id);
                let mut executions = self.executions.write().await;
                if let Some(state) = executions.get_mut(&handle.id) {
                    state.status = ExecutionStatus::Failed(
                        format!("Job execution timed out after {:?}", timeout)
                    );
                    state.completed_at = Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    );
                }
                break;
            }

            // Check VM health
            match self.vm_manager.get_health_status(vm_id).await {
                Ok(health) if health.is_healthy() => {
                    debug!("VM {} is healthy, job still running", vm_id);
                }
                Ok(health) if !health.is_healthy() => {
                    warn!("VM {} is unhealthy: {:?}", vm_id, health);
                    let mut executions = self.executions.write().await;
                    if let Some(state) = executions.get_mut(&handle.id) {
                        state.status = ExecutionStatus::Failed(
                            format!("VM became unhealthy: {:?}", health)
                        );
                        state.completed_at = Some(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        );
                    }
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
                    let mut executions = self.executions.write().await;
                    if let Some(state) = executions.get_mut(&handle.id) {
                        state.status = ExecutionStatus::Completed(WorkResult {
                            success: true,
                            output: Some(serde_json::Value::String(format!(
                                "Job completed on VM {} (VM stopped)",
                                vm_id
                            ))),
                            error: None,
                        });
                        state.completed_at = Some(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        );
                    }
                    break;
                }
                Ok(Some(vm_state)) if vm_state.is_failed() => {
                    error!("VM {} failed", vm_id);
                    let mut executions = self.executions.write().await;
                    if let Some(state) = executions.get_mut(&handle.id) {
                        state.status = ExecutionStatus::Failed(
                            format!("VM {} failed", vm_id)
                        );
                        state.completed_at = Some(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        );
                    }
                    break;
                }
                Ok(None) => {
                    warn!("VM {} not found, marking job as failed", vm_id);
                    let mut executions = self.executions.write().await;
                    if let Some(state) = executions.get_mut(&handle.id) {
                        state.status = ExecutionStatus::Failed(
                            format!("VM {} not found", vm_id)
                        );
                        state.completed_at = Some(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        );
                    }
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
        let state = VmExecutionState {
            handle: handle.clone(),
            vm_id,
            assignment,
            status: ExecutionStatus::Running,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            completed_at: None,
        };
        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), state);
        drop(executions);
        // Start monitoring task
        let self_clone = Arc::new(Self {
            vm_manager: self.vm_manager.clone(),
            executions: self.executions.clone(),
        });
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            self_clone.monitor_execution(handle_clone, vm_id).await;
        });
        info!("VM adapter: job {} assigned to VM {}", job.id, vm_id);
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
                // In a real implementation, we would send a cancellation signal to the VM
                // For now, just mark as cancelled
                state.status = ExecutionStatus::Cancelled;
                state.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                );
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
        let executions = self.executions.read().await;
        let state = executions
            .get(&handle.id)
            .ok_or_else(|| anyhow::anyhow!("Execution not found: {}", handle.id))?;
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
        // Get VM statistics
        let stats = self.vm_manager.get_stats().await?;
        Ok(BackendHealth {
            healthy: true,
            status_message: format!(
                "VM backend healthy: {} VMs running, {} idle",
                stats.running_vms, stats.idle_vms
            ),
            resource_info: Some(self.get_resource_info().await?),
            last_check: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
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
        self.vm_manager.shutdown().await?;
        Ok(())
    }
}
