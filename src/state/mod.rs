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
pub use ui::{UiState, Status, UpdateUI, JobProgress};
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

    // === Backward compatibility accessors ===
    // These allow existing handlers to continue working during migration.
    // Can be removed once all handlers are updated to use services.

    /// Get Flawless module (backward compat)
    pub fn module(&self) -> &DeployedModule {
        self.infrastructure.module()
    }

    /// Get Iroh service (backward compat)
    pub fn iroh(&self) -> &IrohService {
        self.infrastructure.iroh()
    }

    /// Get Hiqlite service (backward compat)
    pub fn hiqlite(&self) -> &HiqliteService {
        self.infrastructure.hiqlite()
    }

    /// Get work queue (backward compat)
    pub fn work_queue(&self) -> &WorkQueue {
        self.infrastructure.work_queue()
    }

    /// Get job progress (backward compat for legacy UI)
    pub async fn progress(&self) -> Vec<JobProgress> {
        self.ui.progress().await
    }

    /// Add job to progress tracker (backward compat)
    pub async fn add_job(&self, url: String) -> usize {
        self.ui.add_job(url).await
    }

    /// Update job status (backward compat)
    pub async fn update_job(&self, id: usize, status: Status, url: String) {
        self.ui.update_job(id, status, url).await
    }
}
