use std::sync::Arc;

use crate::adapters::ExecutionRegistry;
use crate::hiqlite::HiqliteService;
use crate::iroh_service::IrohService;

/// Infrastructure services state container
///
/// Contains low-level infrastructure components that should NOT be accessed
/// by regular HTTP handlers. Handlers should use DomainState instead.
///
/// # Visibility Rules
///
/// - **Public for**: Iroh P2P API handlers (blob storage, gossip endpoints)
/// - **Internal for**: Health checks, service initialization, startup/shutdown
///
/// # Why separate from DomainState?
///
/// Infrastructure separation enforces architectural boundaries. Domain services
/// provide high-level abstractions; handlers shouldn't bypass them to access
/// databases or queues directly.
#[derive(Clone)]
pub struct InfraState {
    iroh: IrohService,
    hiqlite: HiqliteService,
    node_id: String,
    execution_registry: Arc<ExecutionRegistry>,
}

impl InfraState {
    /// Create new infrastructure state
    pub fn new(
        iroh: IrohService,
        hiqlite: HiqliteService,
        node_id: String,
        execution_registry: Arc<ExecutionRegistry>,
    ) -> Self {
        Self {
            iroh,
            hiqlite,
            node_id,
            execution_registry,
        }
    }

    /// Get Iroh P2P service
    ///
    /// PUBLIC for infrastructure-level P2P API handlers (iroh_api.rs)
    pub fn iroh(&self) -> &IrohService {
        &self.iroh
    }

    /// Get Hiqlite database service
    ///
    /// INTERNAL - for health checks and service initialization only
    pub(crate) fn hiqlite(&self) -> &HiqliteService {
        &self.hiqlite
    }

    /// Get node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get work queue connection ticket for worker registration
    ///
    /// This is exposed for startup/display purposes
    pub fn work_queue_ticket(&self) -> String {
        format!("work-queue://{}", self.node_id)
    }

    /// Get execution registry
    ///
    /// INTERNAL - for service initialization only
    pub(crate) fn execution_registry(&self) -> Arc<ExecutionRegistry> {
        self.execution_registry.clone()
    }
}
