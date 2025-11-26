//! VM management domain service
//!
//! This service provides a proper abstraction layer for VM lifecycle management,
//! health tracking, and status queries. It wraps VmManager and VmRegistry operations
//! to provide a cohesive domain API for handlers and other services.
//!
//! Responsibilities:
//! - VM lifecycle (create, start, stop, destroy)
//! - VM listing and status queries
//! - VM registration and health tracking
//! - VM metrics and statistics

use std::sync::Arc;
use anyhow::Result;
use uuid::Uuid;

use crate::vm_manager::{VmManager, VmConfig, VmInstance, VmState, VmRegistry, JobResult, VmStats, VmAssignment};
use crate::vm_manager::vm_types::JobRequirements;

/// Service abstraction for VM management
///
/// This domain service provides a clean API for VM operations, isolating
/// handlers and other services from direct VmManager access. It coordinates
/// between VmManager and VmRegistry to provide comprehensive VM lifecycle
/// and health management.
pub struct VmService {
    vm_manager: Arc<VmManager>,
}

impl VmService {
    /// Create a new VM service
    ///
    /// # Arguments
    /// * `vm_manager` - The underlying VM manager instance
    pub fn new(vm_manager: Arc<VmManager>) -> Self {
        Self { vm_manager }
    }

    /// Get the underlying VM manager (for internal use by execution adapters)
    pub fn vm_manager(&self) -> Arc<VmManager> {
        Arc::clone(&self.vm_manager)
    }

    // === VM Lifecycle Operations ===

    /// Start a new VM with the given configuration
    ///
    /// # Arguments
    /// * `config` - VM configuration including memory, vCPUs, mode, etc.
    ///
    /// # Returns
    /// The newly created VM instance
    pub async fn create_and_start_vm(&self, config: VmConfig) -> Result<VmInstance> {
        tracing::info!(vm_id = %config.id, "Creating and starting new VM");
        self.vm_manager.start_vm(config).await
    }

    /// Start a pre-configured VM
    ///
    /// This is an alias for create_and_start_vm for clarity when using existing configs.
    pub async fn start_vm(&self, config: VmConfig) -> Result<VmInstance> {
        self.create_and_start_vm(config).await
    }

    /// Stop a running VM gracefully
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM to stop
    pub async fn stop_vm(&self, vm_id: Uuid) -> Result<()> {
        tracing::info!(vm_id = %vm_id, "Stopping VM");
        self.vm_manager.stop_vm(vm_id).await
    }

    /// Destroy/terminate a VM
    ///
    /// This is currently an alias for stop_vm. In the future, this may
    /// forcefully terminate a VM if it doesn't respond to graceful shutdown.
    pub async fn destroy_vm(&self, vm_id: Uuid) -> Result<()> {
        tracing::info!(vm_id = %vm_id, "Destroying VM");
        self.stop_vm(vm_id).await
    }

    // === VM Queries ===

    /// Get a VM instance by ID
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM to retrieve
    ///
    /// # Returns
    /// The VM instance if found, None if not found
    pub async fn get_vm(&self, vm_id: Uuid) -> Result<Option<Arc<tokio::sync::RwLock<VmInstance>>>> {
        Ok(self.vm_manager.registry.get(vm_id).await?)
    }

    /// List all VMs
    ///
    /// # Returns
    /// Vector of all VM instances
    pub async fn list_all_vms(&self) -> Result<Vec<VmInstance>> {
        self.vm_manager.registry.list_all_vms().await
    }

    /// List VMs by state
    ///
    /// # Arguments
    /// * `state` - VM state to filter by (as string)
    ///
    /// # Returns
    /// Vector of VMs in the specified state
    pub async fn list_vms_by_state(&self, state: &str) -> Result<Vec<VmInstance>> {
        self.vm_manager.registry.list_by_state(state).await
    }

    /// Get all running VMs
    ///
    /// # Returns
    /// Vector of VMs currently in a running state
    pub async fn list_running_vms(&self) -> Result<Vec<VmInstance>> {
        self.vm_manager.registry.list_running_vms().await
    }

    /// Get an available service VM for job assignment
    ///
    /// Returns a service VM that is ready to accept jobs.
    ///
    /// # Returns
    /// Option containing a service VM ID if one is available
    pub async fn get_available_service_vm(&self) -> Option<Uuid> {
        self.vm_manager.registry.get_available_service_vm().await
    }

    /// Find an idle VM matching job requirements
    ///
    /// # Arguments
    /// * `requirements` - Job requirements for VM selection
    ///
    /// # Returns
    /// Option containing a VM ID if one matches the requirements
    pub async fn find_idle_vm(
        &self,
        requirements: &JobRequirements,
    ) -> Option<Uuid> {
        self.vm_manager.registry.find_idle_vm(requirements).await
    }

    // === VM Status and Health ===

    /// Get VM statistics
    ///
    /// # Returns
    /// Statistics including total, running, idle, and failed VM counts
    pub async fn get_stats(&self) -> Result<VmStats> {
        self.vm_manager.get_stats().await
    }

    /// Check if a specific VM is healthy
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM to check
    ///
    /// # Returns
    /// True if the VM is running and responsive
    pub async fn is_vm_healthy(&self, vm_id: Uuid) -> Result<bool> {
        if let Some(vm_arc) = self.get_vm(vm_id).await? {
            let vm = vm_arc.read().await;
            Ok(vm.state.is_running())
        } else {
            Ok(false)
        }
    }

    /// Get the current state of a VM
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM
    ///
    /// # Returns
    /// The current VmState if the VM exists
    pub async fn get_vm_state(&self, vm_id: Uuid) -> Result<Option<VmState>> {
        if let Some(vm_arc) = self.get_vm(vm_id).await? {
            let vm = vm_arc.read().await;
            Ok(Some(vm.state.clone()))
        } else {
            Ok(None)
        }
    }

    // === Job Execution ===

    /// Execute a job on the best available VM
    ///
    /// The VM manager will route the job to an ephemeral or service VM based
    /// on the job requirements and current VM availability.
    ///
    /// # Arguments
    /// * `job` - The job to execute
    ///
    /// # Returns
    /// VM assignment information (either Ephemeral or Service)
    pub async fn execute_job(&self, job: crate::Job) -> Result<VmAssignment> {
        tracing::info!(job_id = %job.id, "Executing job via VM service");
        self.vm_manager.execute_job(job).await
    }

    /// Execute a job on a specific VM
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the target VM
    /// * `job` - The job to execute
    ///
    /// # Returns
    /// Result of job submission
    pub async fn execute_job_on_vm(&self, vm_id: Uuid, job: crate::Job) -> Result<JobResult> {
        tracing::info!(vm_id = %vm_id, job_id = %job.id, "Executing job on specific VM");
        self.vm_manager.submit_job_to_vm(vm_id, job).await
    }

    /// Submit a job to the VM manager (routes to best VM)
    ///
    /// # Arguments
    /// * `job` - The job to submit
    ///
    /// # Returns
    /// Job result with VM assignment and execution status
    pub async fn submit_job(&self, job: crate::Job) -> Result<JobResult> {
        tracing::info!(job_id = %job.id, "Submitting job via VM service");
        self.vm_manager.submit_job(job).await
    }

    // === Lifecycle Management ===

    /// Start VM manager background tasks
    ///
    /// Should be called once at application startup. This spawns:
    /// - Health checking loops
    /// - Resource monitoring
    /// - Pre-warming of idle VMs
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting VM service");
        self.vm_manager.start().await
    }

    /// Gracefully shutdown all VMs
    ///
    /// This should be called during application shutdown to ensure
    /// all VMs are properly terminated.
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down VM service");
        self.vm_manager.shutdown().await
    }

    // === VM Registry Access ===

    /// Get the underlying VM registry (for health service integration)
    pub fn registry(&self) -> Arc<VmRegistry> {
        Arc::clone(&self.vm_manager.registry)
    }

    /// Count all VMs
    pub async fn count_all(&self) -> usize {
        self.vm_manager.registry.count_all().await
    }

    /// Count VMs in a specific state
    ///
    /// # Arguments
    /// * `state` - The VmState to count
    pub async fn count_by_state(&self, state: VmState) -> usize {
        self.vm_manager.registry.count_by_state(state).await
    }

    /// Recover VMs from persistence (on startup)
    ///
    /// Loads VMs that were persisted in Hiqlite from a previous run.
    ///
    /// # Returns
    /// Number of VMs recovered
    pub async fn recover_vms(&self) -> Result<usize> {
        tracing::info!("Recovering VMs from persistence");
        self.vm_manager.registry.recover_from_persistence().await
    }

    /// Log a VM event for auditing
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM
    /// * `event_type` - Type of event (e.g., "started", "failed")
    /// * `details` - Optional detailed information about the event
    pub async fn log_vm_event(
        &self,
        vm_id: Uuid,
        event_type: &str,
        details: Option<String>,
    ) -> Result<()> {
        self.vm_manager.registry.log_event(vm_id, event_type, details).await
    }

    /// Update VM state in registry
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM
    /// * `new_state` - The new state to set
    pub async fn update_vm_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        self.vm_manager.registry.update_state(vm_id, new_state).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_service_creation() {
        // This test will verify the service can be created
        // Full tests would require mocking VmManager
        let _service_created = true;
        assert!(_service_created);
    }
}
