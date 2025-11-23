//! Infrastructure state container
//!
//! Holds all infrastructure service dependencies (database, queue, networking, flawless).

use flawless_utils::DeployedModule;

use crate::iroh_service::IrohService;
use crate::hiqlite_service::HiqliteService;
use crate::work_queue::WorkQueue;

/// Container for infrastructure services
///
/// This struct holds all the "infrastructure layer" dependencies:
/// - Database/state (HiqliteService)
/// - Job queue (WorkQueue)
/// - P2P networking (IrohService)
/// - Flawless module (DeployedModule)
#[derive(Clone)]
pub struct InfrastructureState {
    pub(crate) module: Arc<DeployedModule>,
    pub(crate) iroh: IrohService,
    pub(crate) hiqlite: HiqliteService,
    pub(crate) work_queue: WorkQueue,
}

use std::sync::Arc;

impl InfrastructureState {
    /// Create a new infrastructure state container
    pub fn new(
        module: DeployedModule,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
    ) -> Self {
        Self {
            module: Arc::new(module),
            iroh,
            hiqlite,
            work_queue,
        }
    }

    /// Get the Flawless module
    pub fn module(&self) -> &DeployedModule {
        &self.module
    }

    /// Get the Iroh service
    pub fn iroh(&self) -> &IrohService {
        &self.iroh
    }

    /// Get the Hiqlite service
    pub fn hiqlite(&self) -> &HiqliteService {
        &self.hiqlite
    }

    /// Get the work queue
    pub fn work_queue(&self) -> &WorkQueue {
        &self.work_queue
    }
}
