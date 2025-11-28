//! VmManagement trait - Domain abstraction for VM operations
//!
//! This trait defines the interface for VM management operations at the domain level.
//! Infrastructure implements this trait, allowing domain services to work with VMs
//! without depending on concrete infrastructure implementations.
//!
//! This follows the Dependency Inversion Principle from clean architecture.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::{JobRequirements, JobResult, VmAssignment, VmConfig, VmInstance, VmState, VmStats};
use crate::domain::job::Job;
use crate::infrastructure::vm::health_checker::HealthStatus;

/// Trait for VM lifecycle management
///
/// This abstraction allows domain services to work with VM operations
/// without coupling to the concrete VmManager implementation.
///
/// Infrastructure layer provides the implementation of this trait.
#[async_trait::async_trait]
pub trait VmManagement: Send + Sync {
    // === Lifecycle Operations ===

    /// Start the VM manager background tasks
    async fn start(&self) -> Result<()>;

    /// Gracefully shutdown all VMs
    async fn shutdown(&self) -> Result<()>;

    /// Start a new VM with the given configuration
    async fn start_vm(&self, config: VmConfig) -> Result<VmInstance>;

    /// Stop a running VM
    async fn stop_vm(&self, vm_id: Uuid) -> Result<()>;

    // === Job Execution ===

    /// Route a job to an appropriate VM
    async fn execute_job(&self, job: Job) -> Result<VmAssignment>;

    /// Submit a job to the VM manager (routes to best VM)
    async fn submit_job(&self, job: Job) -> Result<JobResult>;

    /// Submit a job to a specific VM
    async fn submit_job_to_vm(&self, vm_id: Uuid, job: Job) -> Result<JobResult>;

    // === VM Queries ===

    /// Get a VM instance by ID (returns Arc<RwLock<VmInstance>> for internal state management)
    async fn get_vm(&self, vm_id: Uuid) -> Result<Option<Arc<RwLock<VmInstance>>>>;

    /// List all VMs
    async fn list_all_vms(&self) -> Result<Vec<VmInstance>>;

    /// List VMs by state (string parameter for compatibility)
    async fn list_by_state(&self, state: &str) -> Result<Vec<VmInstance>>;

    /// List running VMs
    async fn list_running_vms(&self) -> Result<Vec<VmInstance>>;

    /// Get an available service VM
    async fn get_available_service_vm(&self) -> Option<Uuid>;

    /// Find an idle VM matching requirements
    async fn find_idle_vm(&self, requirements: &JobRequirements) -> Option<Uuid>;

    // === Statistics and Monitoring ===

    /// Get current VM statistics
    async fn get_stats(&self) -> Result<VmStats>;

    /// Count all VMs
    async fn count_all(&self) -> usize;

    /// Count VMs by state
    async fn count_by_state(&self, state: VmState) -> usize;

    /// Get VM state
    async fn get_vm_status(&self, vm_id: Uuid) -> Result<Option<VmState>>;

    /// Get health status of a specific VM
    async fn get_health_status(&self, vm_id: Uuid) -> Result<HealthStatus>;

    // === State Management ===

    /// Recover VMs from persistence
    async fn recover_from_persistence(&self) -> Result<usize>;

    /// Log a VM event
    async fn log_event(&self, vm_id: Uuid, event_type: &str, details: Option<String>) -> Result<()>;

    /// Update VM state
    async fn update_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()>;

    // === Monitoring ===

    /// Start monitoring tasks
    async fn start_monitoring(&self) -> Result<()>;
}
