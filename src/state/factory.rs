//! Infrastructure factory pattern for dependency injection
//!
//! This module provides abstractions for creating infrastructure components,
//! enabling testability and dependency inversion.

use anyhow::Result;
use async_trait::async_trait;
use flawless_utils::DeployedModule;
use iroh::Endpoint;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::hiqlite_persistent_store::HiqlitePersistentStore;
use crate::hiqlite_service::HiqliteService;
use crate::iroh_service::IrohService;
use crate::repositories::{StateRepository, WorkRepository, WorkerRepository};
use crate::state::{AppState, DomainServices, InfrastructureState};
use crate::vm_manager::{VmManager, VmManagerConfig};
use crate::work_queue::WorkQueue;

/// Factory trait for creating infrastructure components
///
/// This trait enables dependency injection by abstracting infrastructure creation.
/// Implementations can provide real production services or test doubles.
#[async_trait]
pub trait InfrastructureFactory: Send + Sync {
    /// Create Hiqlite distributed state service
    async fn create_hiqlite(&self, config: &AppConfig) -> Result<HiqliteService>;

    /// Create Iroh P2P networking service
    async fn create_iroh(
        &self,
        config: &AppConfig,
        endpoint: Endpoint,
    ) -> Result<IrohService>;

    /// Create distributed work queue
    async fn create_work_queue(
        &self,
        config: &AppConfig,
        endpoint: Endpoint,
        node_id: String,
        hiqlite: HiqliteService,
    ) -> Result<WorkQueue>;

    /// Create VM manager for orchestrating microvms
    async fn create_vm_manager(
        &self,
        config: &AppConfig,
        hiqlite: Arc<HiqliteService>,
    ) -> Result<Arc<VmManager>>;

    /// Create state repository abstraction
    fn create_state_repository(&self, hiqlite: Arc<HiqliteService>) -> Arc<dyn StateRepository>;

    /// Create work repository abstraction
    fn create_work_repository(&self, work_queue: WorkQueue) -> Arc<dyn WorkRepository>;

    /// Create worker repository abstraction
    fn create_worker_repository(&self, hiqlite: Arc<HiqliteService>) -> Arc<dyn WorkerRepository>;

    /// Build complete application state (orchestrator method)
    ///
    /// This method coordinates the creation of all infrastructure components
    /// and constructs the AppState with proper dependency injection.
    async fn build_app_state(
        &self,
        config: &AppConfig,
        module: DeployedModule,
        endpoint: Endpoint,
        node_id: String,
    ) -> Result<AppState> {
        // Create infrastructure services
        let hiqlite = self.create_hiqlite(config).await?;
        hiqlite.initialize_schema().await?;

        let iroh = self.create_iroh(config, endpoint.clone()).await?;

        // Create work queue with hiqlite
        let work_queue = self.create_work_queue(config, endpoint, node_id, hiqlite.clone()).await?;

        // Create shared Hiqlite Arc for services that need it
        let hiqlite_arc = Arc::new(hiqlite.clone());

        // Create VM manager
        let vm_manager = self.create_vm_manager(config, hiqlite_arc.clone()).await?;

        // Create infrastructure state
        let infrastructure = InfrastructureState::new(
            module,
            iroh,
            hiqlite,
            work_queue.clone(),
            vm_manager
        );

        // Create repository abstractions
        let state_repo = self.create_state_repository(hiqlite_arc.clone());
        let work_repo = self.create_work_repository(work_queue);
        let worker_repo = self.create_worker_repository(hiqlite_arc);

        // Create domain services with injected repositories
        let services = DomainServices::from_repositories(state_repo, work_repo, worker_repo);

        Ok(AppState::new(infrastructure, services))
    }
}

/// Production implementation of InfrastructureFactory
///
/// Creates real infrastructure services for production use.
pub struct ProductionInfrastructureFactory;

impl ProductionInfrastructureFactory {
    /// Create a new production factory
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl InfrastructureFactory for ProductionInfrastructureFactory {
    async fn create_hiqlite(&self, config: &AppConfig) -> Result<HiqliteService> {
        let hiqlite = HiqliteService::new(config.storage.hiqlite_data_dir.clone()).await?;
        Ok(hiqlite)
    }

    async fn create_iroh(
        &self,
        config: &AppConfig,
        endpoint: Endpoint,
    ) -> Result<IrohService> {
        Ok(IrohService::new(config.storage.iroh_blobs_path.clone(), endpoint))
    }

    async fn create_work_queue(
        &self,
        _config: &AppConfig,
        endpoint: Endpoint,
        node_id: String,
        hiqlite: HiqliteService,
    ) -> Result<WorkQueue> {
        let persistent_store = Arc::new(HiqlitePersistentStore::new(hiqlite));
        WorkQueue::new(endpoint, node_id, persistent_store).await
    }

    async fn create_vm_manager(
        &self,
        config: &AppConfig,
        hiqlite: Arc<HiqliteService>,
    ) -> Result<Arc<VmManager>> {
        // Configure VM manager from app config
        let vm_config = VmManagerConfig {
            max_vms: config.vm.max_concurrent_vms,
            auto_scaling: true,
            pre_warm_count: 2,
            flake_dir: config.vm.flake_dir.clone(),
            state_dir: config.storage.vm_state_dir.clone(),
            default_memory_mb: config.vm.default_memory_mb,
            default_vcpus: config.vm.default_vcpus,
        };

        let vm_manager = VmManager::new(vm_config, hiqlite).await?;
        Ok(Arc::new(vm_manager))
    }

    fn create_state_repository(&self, hiqlite: Arc<HiqliteService>) -> Arc<dyn StateRepository> {
        use crate::repositories::HiqliteStateRepository;
        Arc::new(HiqliteStateRepository::new(hiqlite))
    }

    fn create_work_repository(&self, work_queue: WorkQueue) -> Arc<dyn WorkRepository> {
        use crate::repositories::WorkQueueWorkRepository;
        Arc::new(WorkQueueWorkRepository::new(work_queue))
    }

    fn create_worker_repository(&self, hiqlite: Arc<HiqliteService>) -> Arc<dyn WorkerRepository> {
        use crate::repositories::HiqliteWorkerRepository;
        Arc::new(HiqliteWorkerRepository::new(hiqlite))
    }
}

// =============================================================================
// TEST INFRASTRUCTURE
// =============================================================================

pub mod test_factory {
    use super::*;
    use crate::repositories::mocks::{MockStateRepository, MockWorkRepository, MockWorkerRepository};

    /// Test implementation of InfrastructureFactory
    ///
    /// Creates minimal/mock implementations for testing without real infrastructure.
    pub struct TestInfrastructureFactory;

    impl TestInfrastructureFactory {
        /// Create a new test factory
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl InfrastructureFactory for TestInfrastructureFactory {
        async fn create_hiqlite(&self, _config: &AppConfig) -> Result<HiqliteService> {
            // Create in-memory hiqlite instance for testing
            HiqliteService::new(None).await
        }

        async fn create_iroh(
            &self,
            _config: &AppConfig,
            endpoint: Endpoint,
        ) -> Result<IrohService> {
            // Use temporary test directory for iroh blobs
            let test_path = std::env::temp_dir().join("mvm-ci-test-iroh");
            Ok(IrohService::new(test_path, endpoint))
        }

        async fn create_work_queue(
            &self,
            _config: &AppConfig,
            endpoint: Endpoint,
            node_id: String,
            hiqlite: HiqliteService,
        ) -> Result<WorkQueue> {
            // Same as production but with test hiqlite
            let persistent_store = Arc::new(HiqlitePersistentStore::new(hiqlite));
            WorkQueue::new(endpoint, node_id, persistent_store).await
        }

        async fn create_vm_manager(
            &self,
            _config: &AppConfig,
            hiqlite: Arc<HiqliteService>,
        ) -> Result<Arc<VmManager>> {
            // Test VM configuration with minimal resources
            let vm_config = VmManagerConfig {
                max_vms: 3,  // Limited for testing
                auto_scaling: false,  // Disabled for testing
                pre_warm_count: 0,  // No pre-warming in tests
                flake_dir: std::path::PathBuf::from("./microvms"),
                state_dir: std::env::temp_dir().join("mvm-ci-test-vms"),
                default_memory_mb: 256,  // Minimal memory for tests
                default_vcpus: 1,
            };

            let vm_manager = VmManager::new(vm_config, hiqlite).await?;
            Ok(Arc::new(vm_manager))
        }

        fn create_state_repository(&self, _hiqlite: Arc<HiqliteService>) -> Arc<dyn StateRepository> {
            // Return mock repository for testing
            Arc::new(MockStateRepository::new())
        }

        fn create_work_repository(&self, _work_queue: WorkQueue) -> Arc<dyn WorkRepository> {
            // Return mock repository for testing
            Arc::new(MockWorkRepository::new())
        }

        fn create_worker_repository(&self, _hiqlite: Arc<HiqliteService>) -> Arc<dyn WorkerRepository> {
            // Return mock repository for testing
            Arc::new(MockWorkerRepository::new())
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub use test_factory::TestInfrastructureFactory;
