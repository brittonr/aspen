//! Infrastructure state container
//!
//! Holds all infrastructure service dependencies (database, queue, networking, flawless).

use std::sync::Arc;
use flawless_utils::DeployedModule;

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
/// - VM management (VmManager - orchestrates VM lifecycle)
///
/// # Design
///
/// Services implement traits for consistency and documentation, but are stored
/// as concrete types to avoid object-safety issues with generic methods.
/// Methods return trait object references where appropriate for flexibility.
#[derive(Clone)]
pub struct InfrastructureState {
    pub(crate) module: Arc<DeployedModule>,
    pub(crate) iroh: IrohService,
    pub(crate) hiqlite: HiqliteService,
    pub(crate) work_queue: WorkQueue,
    pub(crate) vm_manager: Arc<VmManager>,
}

impl InfrastructureState {
    /// Create a new infrastructure state container
    pub fn new(
        module: DeployedModule,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
        vm_manager: Arc<VmManager>,
    ) -> Self {
        Self {
            module: Arc::new(module),
            iroh,
            hiqlite,
            work_queue,
            vm_manager,
        }
    }

    /// Get the Flawless module
    pub fn module(&self) -> &DeployedModule {
        &self.module
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

    /// Get the VM manager
    pub fn vm_manager(&self) -> &Arc<VmManager> {
        &self.vm_manager
    }
}
