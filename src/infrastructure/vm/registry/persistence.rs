// VM Persistence Trait - Abstract database operations
//
// This trait defines the contract for persisting VM state to a durable store.
// Implementations handle serialization, SQL operations, and event logging.

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::infrastructure::vm::vm_types::{VmInstance, VmMetrics, VmState};

#[async_trait]
pub trait VmPersistence: Send + Sync {
    /// Save a VM instance to persistent storage
    async fn save(&self, vm: &VmInstance) -> Result<()>;

    /// Load a VM instance by ID
    async fn load(&self, vm_id: &Uuid) -> Result<Option<VmInstance>>;

    /// Load all VMs for this node
    async fn load_all(&self) -> Result<Vec<VmInstance>>;

    /// Delete a VM from persistent storage
    async fn delete(&self, vm_id: &Uuid) -> Result<()>;

    /// Update just the state field (atomic operation)
    async fn update_state(&self, vm_id: &Uuid, state: &VmState) -> Result<()>;

    /// Update just the metrics field
    async fn update_metrics(&self, vm_id: &Uuid, metrics: &VmMetrics) -> Result<()>;

    /// Query VM IDs by state
    async fn query_by_state(&self, state: &VmState) -> Result<Vec<Uuid>>;

    /// Log a VM lifecycle event (best-effort)
    async fn log_event(&self, vm_id: &Uuid, event: &str, data: Option<String>) -> Result<()>;
}
