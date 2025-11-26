//! VM execution backend adapter wrapping the existing VmManager
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;
use crate::domain::types::Job;
use crate::vm_manager::{JobResult, VmAssignment, VmManager, VmManagerConfig};
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
        hiqlite: Arc<crate::hiqlite_service::HiqliteService>,
    ) -> Result<Self> {
        let vm_manager = Arc::new(VmManager::new(config, hiqlite).await?);
        Ok(Self::new(vm_manager))
    }
    /// Convert JobResult to WorkResult
    fn job_result_to_work_result(result: &JobResult) -> WorkResult {
        WorkResult {
            success: result.success,
            output: Some(serde_json::Value::String(result.output.clone())),
            error: result.error.clone(),
        }
    }
    /// Monitor VM job execution
    async fn monitor_execution(&self, handle: ExecutionHandle, vm_id: uuid::Uuid) {
        // In a real implementation, we would monitor the VM for job completion
        // For now, we'll just wait a bit and mark as completed
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&handle.id) {
            state.status = ExecutionStatus::Completed(WorkResult {
                success: true,
                output: Some(serde_json::Value::String(format!(
                    "Job completed on VM {}",
                    vm_id
                ))),
                error: None,
            });
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
                .unwrap()
                .as_secs(),
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
                        .unwrap()
                        .as_secs(),
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
                    .unwrap()
                    .as_secs(),
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
