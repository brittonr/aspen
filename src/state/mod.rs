//! Application state management
//!
//! Modular state architecture separating infrastructure and domain concerns.

use std::sync::Arc;
use flawless_utils::DeployedModule;

use crate::iroh_service::IrohService;
use crate::hiqlite_service::HiqliteService;
use crate::work_queue::WorkQueue;

pub mod infrastructure;
pub mod services;

// Re-export sub-states for convenience
pub use infrastructure::InfrastructureState;
pub use services::DomainServices;

/// Root application state
///
/// Composes focused state containers for different concerns:
/// - Infrastructure: databases, queues, networking
/// - Services: pre-built domain services
#[derive(Clone)]
pub struct AppState {
    infrastructure: Arc<InfrastructureState>,
    services: Arc<DomainServices>,
}

impl AppState {
    /// Create new application state with all sub-states
    pub fn new(
        infrastructure: InfrastructureState,
        services: DomainServices,
    ) -> Self {
        Self {
            infrastructure: Arc::new(infrastructure),
            services: Arc::new(services),
        }
    }

    /// Create application state from infrastructure services
    pub fn from_infrastructure(
        module: DeployedModule,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
    ) -> Self {
        let infrastructure = InfrastructureState::new(module, iroh, hiqlite, work_queue);
        let services = DomainServices::new(&infrastructure);

        Self::new(infrastructure, services)
    }

    // === Sub-state accessors ===

    /// Get infrastructure state
    pub fn infrastructure(&self) -> &InfrastructureState {
        &self.infrastructure
    }

    /// Get domain services
    pub fn services(&self) -> &DomainServices {
        &self.services
    }
}
