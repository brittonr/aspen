//! Application state management
//!
//! Modular state architecture separating infrastructure, UI, and domain concerns.

use std::sync::Arc;
use flawless_utils::DeployedModule;

use crate::iroh_service::IrohService;
use crate::hiqlite_service::HiqliteService;
use crate::work_queue::WorkQueue;

pub mod infrastructure;
pub mod ui;
pub mod services;

// Re-export sub-states for convenience
pub use infrastructure::InfrastructureState;
pub use ui::{UiState, UpdateUI, JobProgress};
pub use services::DomainServices;

/// Root application state
///
/// Composes focused state containers for different concerns:
/// - Infrastructure: databases, queues, networking
/// - UI: legacy job progress tracking
/// - Services: pre-built domain services
#[derive(Clone)]
pub struct AppState {
    infrastructure: Arc<InfrastructureState>,
    ui: Arc<UiState>,
    services: Arc<DomainServices>,
}

impl AppState {
    /// Create new application state with all sub-states
    pub fn new(
        infrastructure: InfrastructureState,
        ui: UiState,
        services: DomainServices,
    ) -> Self {
        Self {
            infrastructure: Arc::new(infrastructure),
            ui: Arc::new(ui),
            services: Arc::new(services),
        }
    }

    /// Legacy constructor for backward compatibility
    ///
    /// Creates all state containers from infrastructure services.
    /// This allows incremental migration - handlers can be updated one by one.
    pub fn from_infrastructure(
        module: DeployedModule,
        iroh: IrohService,
        hiqlite: HiqliteService,
        work_queue: WorkQueue,
    ) -> Self {
        let infrastructure = InfrastructureState::new(module, iroh, hiqlite, work_queue);
        let ui = UiState::new();
        let services = DomainServices::new(&infrastructure, &ui);

        Self::new(infrastructure, ui, services)
    }

    // === Sub-state accessors ===

    /// Get infrastructure state
    pub fn infrastructure(&self) -> &InfrastructureState {
        &self.infrastructure
    }

    /// Get UI state
    pub fn ui(&self) -> &UiState {
        &self.ui
    }

    /// Get domain services
    pub fn services(&self) -> &DomainServices {
        &self.services
    }
}
