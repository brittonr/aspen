//! Application state management
//!
//! Modular state architecture separating infrastructure and domain concerns.

use std::sync::Arc;
use flawless_utils::DeployedModule;

use crate::iroh_service::IrohService;
use crate::hiqlite_service::HiqliteService;
use crate::work_queue::WorkQueue;

pub mod factory;
pub mod infrastructure;
pub mod services;

// Re-export sub-states for convenience
pub use infrastructure::InfrastructureState;
pub use services::DomainServices;

// Re-export factory types for convenience
pub use factory::{InfrastructureFactory, ProductionInfrastructureFactory};

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
    pub async fn from_infrastructure(
        module: DeployedModule,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
    ) -> Self {
        // Create a VM manager with default config
        let vm_config = crate::vm_manager::VmManagerConfig::default();
        let vm_manager = std::sync::Arc::new(
            crate::vm_manager::VmManager::new(vm_config, std::sync::Arc::new(hiqlite.clone()))
                .await
                .expect("Failed to create VM manager")
        );

        let infrastructure = InfrastructureState::new(module, iroh, hiqlite, work_queue, vm_manager);
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

    /// Get domain services (alias for ergonomic access)
    pub fn domain_services(&self) -> &DomainServices {
        &self.services
    }
}
