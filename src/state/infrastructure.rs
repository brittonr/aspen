//! Infrastructure state container
//!
//! Holds all infrastructure service dependencies (database, queue, networking, execution backends).

use std::sync::Arc;
use flawless_utils::DeployedModule;

use crate::adapters::ExecutionRegistry;
use crate::hiqlite_service::HiqliteService;
use crate::iroh_service::IrohService;
use crate::services::traits::{
    BlobStorage, DatabaseHealth, DatabaseLifecycle, DatabaseSchema,
    EndpointInfo, GossipNetwork, PeerConnection,
};
use crate::vm_manager::VmManager;
use crate::work_queue::WorkQueue;

/// Container for infrastructure services
///
/// This struct holds all the "infrastructure layer" dependencies:
/// - Database/state (HiqliteService - concrete type due to generic methods)
/// - Job queue (WorkQueue)
/// - P2P networking (IrohService - concrete type wrapping all capabilities)
/// - Flawless module (DeployedModule)
/// - Execution backends (ExecutionRegistry - manages multiple execution adapters)
///
/// # Design
///
/// Services implement traits for consistency and documentation, but are stored
/// as concrete types to avoid object-safety issues with generic methods.
/// Methods return trait object references where appropriate for flexibility.
///
/// The ExecutionRegistry provides a decoupled way to manage different execution
/// backends (VMs, Flawless, local processes, etc.) without tight coupling.
#[derive(Clone)]
pub struct InfrastructureState {
    pub(crate) module: Option<Arc<DeployedModule>>,
    pub(crate) iroh: IrohService,
    pub(crate) hiqlite: HiqliteService,
    pub(crate) work_queue: WorkQueue,
    pub(crate) execution_registry: Arc<ExecutionRegistry>,
    // Keep vm_manager for backwards compatibility during migration
    pub(crate) vm_manager: Option<Arc<VmManager>>,
}

impl InfrastructureState {
    /// Create a new infrastructure state container with execution registry
    pub fn new_with_registry(
        module: Option<DeployedModule>,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
        execution_registry: Arc<ExecutionRegistry>,
    ) -> Self {
        Self {
            module: module.map(Arc::new),
            iroh,
            hiqlite,
            work_queue,
            execution_registry,
            vm_manager: None,
        }
    }

    /// Create a new infrastructure state container (legacy, for backwards compatibility)
    pub fn new(
        module: Option<DeployedModule>,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
        vm_manager: Arc<VmManager>,
    ) -> Self {
        // Create a default execution registry
        let registry_config = crate::adapters::RegistryConfig::default();
        let execution_registry = Arc::new(ExecutionRegistry::new(registry_config));

        Self {
            module: module.map(Arc::new),
            iroh,
            hiqlite,
            work_queue,
            execution_registry,
            vm_manager: Some(vm_manager),
        }
    }

    /// Get the Flawless module (if available)
    pub fn module(&self) -> Option<&DeployedModule> {
        self.module.as_ref().map(|m| m.as_ref())
    }

    /// Get the Iroh service (concrete type, implements all iroh traits)
    pub fn iroh(&self) -> &IrohService {
        &self.iroh
    }

    /// Get Iroh service as EndpointInfo trait
    pub fn iroh_endpoint(&self) -> &dyn EndpointInfo {
        &self.iroh
    }

    /// Get Iroh service as BlobStorage trait
    pub fn iroh_blobs(&self) -> &dyn BlobStorage {
        &self.iroh
    }

    /// Get Iroh service as GossipNetwork trait
    pub fn iroh_gossip(&self) -> &dyn GossipNetwork {
        &self.iroh
    }

    /// Get Iroh service as PeerConnection trait
    pub fn iroh_peers(&self) -> &dyn PeerConnection {
        &self.iroh
    }

    /// Get the Hiqlite service (concrete type, implements all database traits)
    pub fn hiqlite(&self) -> &HiqliteService {
        &self.hiqlite
    }

    /// Get Hiqlite service as DatabaseHealth trait
    pub fn db_health(&self) -> &dyn DatabaseHealth {
        &self.hiqlite
    }

    /// Get Hiqlite service as DatabaseSchema trait
    pub fn db_schema(&self) -> &dyn DatabaseSchema {
        &self.hiqlite
    }

    /// Get Hiqlite service as DatabaseLifecycle trait
    pub fn db_lifecycle(&self) -> &dyn DatabaseLifecycle {
        &self.hiqlite
    }

    /// Get the work queue
    pub fn work_queue(&self) -> &WorkQueue {
        &self.work_queue
    }

    /// Get the execution registry
    pub fn execution_registry(&self) -> &Arc<ExecutionRegistry> {
        &self.execution_registry
    }

    /// Get the VM manager (legacy, for backwards compatibility)
    ///
    /// Returns None if VM manager was not initialized (e.g., when SKIP_VM_MANAGER is set)
    pub fn vm_manager(&self) -> Option<&Arc<VmManager>> {
        self.vm_manager.as_ref()
    }

    /// Check if using legacy VM manager
    pub fn has_vm_manager(&self) -> bool {
        self.vm_manager.is_some()
    }
}
