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

use crate::infrastructure::vm::{
    VmManagement, VmConfig, VmInstance, VmState, JobResult, VmStats, VmAssignment,
    vm_types::JobRequirements
};

/// Error returned when VM operations are attempted without an available VM manager
const VM_UNAVAILABLE_ERROR: &str = "VM manager is not available (SKIP_VM_MANAGER may be set)";

/// Service abstraction for VM management
///
/// This domain service provides a clean API for VM operations, isolating
/// handlers and other services from direct VmManager access. It depends on
/// the VmManagement trait rather than concrete implementations, enabling
/// better testability and decoupling.
///
/// The VM manager may be unavailable if the application was started with
/// SKIP_VM_MANAGER=true or if the vm-backend feature is disabled. In such
/// cases, all operations will return an error indicating VMs are unavailable.
pub struct VmService {
    vm_manager: Option<Arc<dyn VmManagement>>,
}

impl VmService {
    /// Create a new VM service with an available VM manager
    ///
    /// # Arguments
    /// * `vm_manager` - The underlying VM manager instance (must implement VmManagement)
    pub fn new(vm_manager: Arc<dyn VmManagement>) -> Self {
        Self { vm_manager: Some(vm_manager) }
    }

    /// Create a stub VM service when VM manager is not available
    ///
    /// All operations on this service will return errors indicating
    /// that the VM manager is unavailable.
    pub fn unavailable() -> Self {
        Self { vm_manager: None }
    }

    /// Check if VM manager is available
    pub fn is_available(&self) -> bool {
        self.vm_manager.is_some()
    }

    /// Get the underlying VM manager (for internal use by execution adapters)
    ///
    /// Returns an error if VM manager is not available.
    pub fn vm_manager(&self) -> Result<Arc<dyn VmManagement>> {
        self.vm_manager
            .clone()
            .ok_or_else(|| anyhow::anyhow!(VM_UNAVAILABLE_ERROR))
    }

    // === VM Lifecycle Operations ===

    /// Start a new VM with the given configuration
    ///
    /// # Arguments
    /// * `config` - VM configuration including memory, vCPUs, mode, etc.
    ///
    /// # Returns
    /// The newly created VM instance
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn create_and_start_vm(&self, config: VmConfig) -> Result<VmInstance> {
        let vm_manager = self.vm_manager()?;
        tracing::info!(vm_id = %config.id, "Creating and starting new VM");
        vm_manager.start_vm(config).await
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
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn stop_vm(&self, vm_id: Uuid) -> Result<()> {
        let vm_manager = self.vm_manager()?;
        tracing::info!(vm_id = %vm_id, "Stopping VM");
        vm_manager.stop_vm(vm_id).await
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
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn get_vm(&self, vm_id: Uuid) -> Result<Option<Arc<tokio::sync::RwLock<VmInstance>>>> {
        let vm_manager = self.vm_manager()?;
        Ok(vm_manager.get_vm(vm_id).await?)
    }

    /// List all VMs
    ///
    /// # Returns
    /// Vector of all VM instances
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn list_all_vms(&self) -> Result<Vec<VmInstance>> {
        let vm_manager = self.vm_manager()?;
        vm_manager.list_all_vms().await
    }

    /// List VMs by state
    ///
    /// # Arguments
    /// * `state` - VM state to filter by (as string)
    ///
    /// # Returns
    /// Vector of VMs in the specified state
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn list_vms_by_state(&self, state: &str) -> Result<Vec<VmInstance>> {
        let vm_manager = self.vm_manager()?;
        vm_manager.list_by_state(state).await
    }

    /// Get all running VMs
    ///
    /// # Returns
    /// Vector of VMs currently in a running state
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn list_running_vms(&self) -> Result<Vec<VmInstance>> {
        let vm_manager = self.vm_manager()?;
        vm_manager.list_running_vms().await
    }

    /// Get an available service VM for job assignment
    ///
    /// Returns a service VM that is ready to accept jobs.
    ///
    /// # Returns
    /// Option containing a service VM ID if one is available, None if no VMs available
    /// or if VM manager is not available
    pub async fn get_available_service_vm(&self) -> Option<Uuid> {
        let vm_manager = self.vm_manager.as_ref()?;
        vm_manager.get_available_service_vm().await
    }

    /// Find an idle VM matching job requirements
    ///
    /// # Arguments
    /// * `requirements` - Job requirements for VM selection
    ///
    /// # Returns
    /// Option containing a VM ID if one matches the requirements, None if no match
    /// or if VM manager is not available
    pub async fn find_idle_vm(
        &self,
        requirements: &JobRequirements,
    ) -> Option<Uuid> {
        let vm_manager = self.vm_manager.as_ref()?;
        vm_manager.find_idle_vm(requirements).await
    }

    // === VM Status and Health ===

    /// Get VM statistics
    ///
    /// # Returns
    /// Statistics including total, running, idle, and failed VM counts
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn get_stats(&self) -> Result<VmStats> {
        let vm_manager = self.vm_manager()?;
        vm_manager.get_stats().await
    }

    /// Check if a specific VM is healthy
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM to check
    ///
    /// # Returns
    /// True if the VM is running and responsive
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
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
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
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
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn execute_job(&self, job: crate::Job) -> Result<VmAssignment> {
        let vm_manager = self.vm_manager()?;
        tracing::info!(job_id = %job.id, "Executing job via VM service");
        vm_manager.execute_job(job).await
    }

    /// Execute a job on a specific VM
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the target VM
    /// * `job` - The job to execute
    ///
    /// # Returns
    /// Result of job submission
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn execute_job_on_vm(&self, vm_id: Uuid, job: crate::Job) -> Result<JobResult> {
        let vm_manager = self.vm_manager()?;
        tracing::info!(vm_id = %vm_id, job_id = %job.id, "Executing job on specific VM");
        vm_manager.submit_job_to_vm(vm_id, job).await
    }

    /// Submit a job to the VM manager (routes to best VM)
    ///
    /// # Arguments
    /// * `job` - The job to submit
    ///
    /// # Returns
    /// Job result with VM assignment and execution status
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn submit_job(&self, job: crate::Job) -> Result<JobResult> {
        let vm_manager = self.vm_manager()?;
        tracing::info!(job_id = %job.id, "Submitting job via VM service");
        vm_manager.submit_job(job).await
    }

    // === Lifecycle Management ===

    /// Start VM manager background tasks
    ///
    /// Should be called once at application startup. This spawns:
    /// - Health checking loops
    /// - Resource monitoring
    /// - Pre-warming of idle VMs
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn start(&self) -> Result<()> {
        let vm_manager = self.vm_manager()?;
        tracing::info!("Starting VM service");
        vm_manager.start().await
    }

    /// Gracefully shutdown all VMs
    ///
    /// This should be called during application shutdown to ensure
    /// all VMs are properly terminated.
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn shutdown(&self) -> Result<()> {
        let vm_manager = self.vm_manager()?;
        tracing::info!("Shutting down VM service");
        vm_manager.shutdown().await
    }

    // === VM Counting and Stats ===

    /// Count all VMs
    ///
    /// Returns 0 if VM manager is not available.
    pub async fn count_all(&self) -> usize {
        match &self.vm_manager {
            Some(vm_manager) => vm_manager.count_all().await,
            None => 0,
        }
    }

    /// Count VMs in a specific state
    ///
    /// # Arguments
    /// * `state` - The VmState to count
    ///
    /// Returns 0 if VM manager is not available.
    pub async fn count_by_state(&self, state: VmState) -> usize {
        match &self.vm_manager {
            Some(vm_manager) => vm_manager.count_by_state(state).await,
            None => 0,
        }
    }

    /// Recover VMs from persistence (on startup)
    ///
    /// Loads VMs that were persisted in Hiqlite from a previous run.
    ///
    /// # Returns
    /// Number of VMs recovered
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn recover_vms(&self) -> Result<usize> {
        let vm_manager = self.vm_manager()?;
        tracing::info!("Recovering VMs from persistence");
        vm_manager.recover_from_persistence().await
    }

    /// Log a VM event for auditing
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM
    /// * `event_type` - Type of event (e.g., "started", "failed")
    /// * `details` - Optional detailed information about the event
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn log_vm_event(
        &self,
        vm_id: Uuid,
        event_type: &str,
        details: Option<String>,
    ) -> Result<()> {
        let vm_manager = self.vm_manager()?;
        vm_manager.log_event(vm_id, event_type, details).await
    }

    /// Update VM state in registry
    ///
    /// # Arguments
    /// * `vm_id` - The ID of the VM
    /// * `new_state` - The new state to set
    ///
    /// # Errors
    /// Returns an error if VM manager is not available
    pub async fn update_vm_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        let vm_manager = self.vm_manager()?;
        vm_manager.update_state(vm_id, new_state).await
    }
}