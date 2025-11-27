//! Infrastructure factory pattern for dependency injection
//!
//! This module provides abstractions for creating infrastructure components,
//! enabling testability and dependency inversion.

use anyhow::Result;
use async_trait::async_trait;
use flawless_utils::DeployedModule;
use iroh::Endpoint;
use std::sync::Arc;

use crate::adapters::{ExecutionRegistry, RegistryConfig};
use crate::config::AppConfig;
use crate::domain::{
    ClusterStatusService, HealthService, JobLifecycleService, JobCommandService, JobQueryService,
    WorkerManagementService, LoggingEventPublisher,
};
#[cfg(feature = "vm-backend")]
use crate::domain::VmService;
#[cfg(feature = "tofu-support")]
use crate::domain::TofuService;
use crate::hiqlite_persistent_store::HiqlitePersistentStore;
use crate::hiqlite::HiqliteService;
use crate::iroh_service::IrohService;
use crate::repositories::{StateRepository, WorkRepository, WorkerRepository};
use crate::state::{AppState, ConfigState, DomainState, FeaturesState, InfraState};
use crate::infrastructure::vm::{VmManager, VmManagerConfig};
use crate::work_queue::WorkQueue;
use crate::{WorkerBackend, WorkerType};

/// Builder for constructing focused state containers
///
/// This builder contains all infrastructure and domain services needed
/// to construct the four focused state containers (DomainState, InfraState,
/// ConfigState, FeaturesState).
pub struct StateBuilder {
    // Infrastructure components
    iroh: IrohService,
    hiqlite: HiqliteService,
    work_queue: WorkQueue,
    execution_registry: Arc<ExecutionRegistry>,

    // Configuration
    auth_config: Arc<crate::config::AuthConfig>,
    module: Option<DeployedModule>,

    // Domain services
    cluster_status: Arc<ClusterStatusService>,
    health: Arc<HealthService>,
    job_lifecycle: Arc<JobLifecycleService>,
    worker_management: Arc<WorkerManagementService>,

    // Optional features
    #[cfg(feature = "vm-backend")]
    vm_service: Option<Arc<VmService>>,

    #[cfg(feature = "tofu-support")]
    tofu_service: Option<Arc<TofuService>>,
}

impl StateBuilder {
    /// Build all state containers
    ///
    /// Returns a tuple of (DomainState, InfraState, ConfigState, FeaturesState)
    pub fn build(self) -> (DomainState, InfraState, ConfigState, FeaturesState) {
        let domain = DomainState::new(
            self.cluster_status,
            self.health,
            self.job_lifecycle,
            self.worker_management,
        );

        let infra = InfraState::new(
            self.iroh,
            self.hiqlite,
            self.work_queue,
            self.execution_registry,
        );

        let config = ConfigState::new(self.auth_config, self.module);

        let features = FeaturesState::new(
            #[cfg(feature = "vm-backend")]
            self.vm_service,
            #[cfg(feature = "tofu-support")]
            self.tofu_service,
        );

        (domain, infra, config, features)
    }

    /// Build only domain state (useful for testing)
    pub fn build_domain(self) -> DomainState {
        DomainState::new(
            self.cluster_status,
            self.health,
            self.job_lifecycle,
            self.worker_management,
        )
    }

    /// Build legacy AppState for backward compatibility
    ///
    /// This method is deprecated and will be removed once migration is complete
    #[deprecated(note = "Use build() to get focused state containers instead")]
    pub fn build_app_state(self) -> AppState {
        #[cfg(feature = "tofu-support")]
        let tofu_service = self.tofu_service.unwrap_or_else(|| {
            Arc::new(TofuService::new(
                self.execution_registry.clone(),
                Arc::new(self.hiqlite.clone()),
                std::path::PathBuf::from("/tmp"),
            ))
        });

        AppState::new(
            self.auth_config,
            self.module,
            self.iroh,
            self.hiqlite,
            self.work_queue,
            self.execution_registry,
            self.cluster_status,
            self.health,
            self.job_lifecycle,
            self.worker_management,
            #[cfg(feature = "vm-backend")]
            self.vm_service.unwrap_or_else(|| Arc::new(VmService::unavailable())),
            #[cfg(feature = "tofu-support")]
            tofu_service,
        )
    }
}

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

    /// Create a worker backend based on type
    ///
    /// This method creates worker instances with proper dependency injection,
    /// enabling the use of shared infrastructure and test doubles.
    ///
    /// # Arguments
    /// * `config` - Application configuration
    /// * `worker_type` - Type of worker to create (Wasm or Firecracker)
    /// * `vm_manager` - Optional VM manager for Firecracker workers
    async fn create_worker(
        &self,
        config: &AppConfig,
        worker_type: WorkerType,
        vm_manager: Option<Arc<VmManager>>,
    ) -> Result<Box<dyn WorkerBackend>>;

    /// Build state containers using focused architecture (RECOMMENDED)
    ///
    /// This method coordinates the creation of all infrastructure components
    /// and returns a StateBuilder that can construct focused state containers.
    ///
    /// Prefer this over build_app_state() for new code.
    async fn build_state(
        &self,
        config: &AppConfig,
        module: Option<DeployedModule>,
        endpoint: Endpoint,
        node_id: String,
    ) -> Result<StateBuilder> {
        // Create infrastructure services
        let hiqlite = self.create_hiqlite(config).await?;
        hiqlite.initialize_schema().await?;

        let iroh = self.create_iroh(config, endpoint.clone()).await?;

        // Create work queue with hiqlite
        let work_queue = self.create_work_queue(config, endpoint, node_id, hiqlite.clone()).await?;

        // Create shared Hiqlite Arc for services that need it
        let hiqlite_arc = Arc::new(hiqlite.clone());

        // Create execution registry
        let registry_config = RegistryConfig::default();
        let execution_registry = Arc::new(ExecutionRegistry::new(registry_config));

        // Create VM manager (skip if disabled in config)
        let vm_manager = if !config.features.is_vm_manager_enabled() {
            tracing::info!("Skipping VM manager creation (feature disabled in config)");
            None
        } else {
            match self.create_vm_manager(config, hiqlite_arc.clone()).await {
                Ok(manager) => Some(manager),
                Err(e) => {
                    tracing::warn!("Failed to create VM manager: {}. Continuing without VM support.", e);
                    None
                }
            }
        };

        // Create repositories
        let state_repo = self.create_state_repository(hiqlite_arc.clone());
        let work_repo = self.create_work_repository(work_queue.clone());
        let worker_repo = self.create_worker_repository(hiqlite_arc.clone());

        // Create event publisher
        let event_publisher = Arc::new(LoggingEventPublisher::new());

        // Create command and query services (CQRS pattern)
        let commands = Arc::new(JobCommandService::with_events(
            work_repo.clone(),
            event_publisher.clone(),
        ));
        let queries = Arc::new(JobQueryService::new(work_repo.clone()));

        // Create domain services with injected repositories
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let health = Arc::new(HealthService::new(state_repo.clone()));

        // JobLifecycleService wraps command and query services
        let job_lifecycle = Arc::new(JobLifecycleService::from_services(commands, queries));

        // WorkerManagementService with worker and work repositories
        let worker_management = Arc::new(WorkerManagementService::new(
            worker_repo.clone(),
            work_repo.clone(),
            Some(60), // Default heartbeat timeout: 60 seconds
        ));

        // Create VM service if VM manager is available
        #[cfg(feature = "vm-backend")]
        let vm_service = vm_manager.map(|vm_manager| Arc::new(VmService::new(vm_manager)));

        // Create Tofu service
        #[cfg(feature = "tofu-support")]
        let tofu_service = Some(Arc::new(TofuService::new(
            execution_registry.clone(),
            hiqlite_arc.clone(),
            config.storage.work_dir.clone(),
        )));

        // Extract auth config
        let auth_config = Arc::new(config.auth.clone());

        Ok(StateBuilder {
            iroh,
            hiqlite,
            work_queue,
            execution_registry,
            auth_config,
            module,
            cluster_status,
            health,
            job_lifecycle,
            worker_management,
            #[cfg(feature = "vm-backend")]
            vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service,
        })
    }

    /// Build complete application state (orchestrator method)
    ///
    /// DEPRECATED: Use build_state() instead for focused state containers.
    ///
    /// This method coordinates the creation of all infrastructure components
    /// and constructs the AppState with proper dependency injection.
    #[deprecated(note = "Use build_state() for focused state containers")]
    async fn build_app_state(
        &self,
        config: &AppConfig,
        module: Option<DeployedModule>,
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

        // Create execution registry
        let registry_config = RegistryConfig::default();
        let execution_registry = Arc::new(ExecutionRegistry::new(registry_config));

        // Create VM manager (skip if disabled in config)
        let vm_manager = if !config.features.is_vm_manager_enabled() {
            tracing::info!("Skipping VM manager creation (feature disabled in config)");
            None
        } else {
            match self.create_vm_manager(config, hiqlite_arc.clone()).await {
                Ok(manager) => Some(manager),
                Err(e) => {
                    tracing::warn!("Failed to create VM manager: {}. Continuing without VM support.", e);
                    None
                }
            }
        };

        // Create repositories
        let state_repo = self.create_state_repository(hiqlite_arc.clone());
        let work_repo = self.create_work_repository(work_queue.clone());
        let worker_repo = self.create_worker_repository(hiqlite_arc.clone());

        // Create event publisher
        let event_publisher = Arc::new(LoggingEventPublisher::new());

        // Create command and query services (CQRS pattern)
        let commands = Arc::new(JobCommandService::with_events(
            work_repo.clone(),
            event_publisher.clone(),
        ));
        let queries = Arc::new(JobQueryService::new(work_repo.clone()));

        // Create domain services with injected repositories
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let health = Arc::new(HealthService::new(state_repo.clone()));

        // JobLifecycleService wraps command and query services
        let job_lifecycle = Arc::new(JobLifecycleService::from_services(commands, queries));

        // WorkerManagementService with worker and work repositories
        let worker_management = Arc::new(WorkerManagementService::new(
            worker_repo.clone(),
            work_repo.clone(),
            Some(60), // Default heartbeat timeout: 60 seconds
        ));

        // Create VM service if VM manager is available
        #[cfg(feature = "vm-backend")]
        let vm_service = {
            if let Some(vm_manager) = vm_manager {
                Arc::new(VmService::new(vm_manager))
            } else {
                tracing::warn!("VM manager not available, VmService will return errors for all operations");
                Arc::new(VmService::unavailable())
            }
        };

        // Create Tofu service
        #[cfg(feature = "tofu-support")]
        let tofu_service = {
            Arc::new(TofuService::new(
                execution_registry.clone(),
                hiqlite_arc.clone(),
                config.storage.work_dir.clone(),
            ))
        };

        // Extract auth config
        let auth_config = Arc::new(config.auth.clone());

        Ok(AppState::new(
            auth_config,
            module,
            iroh,
            hiqlite,
            work_queue,
            execution_registry,
            cluster_status,
            health,
            job_lifecycle,
            worker_management,
            #[cfg(feature = "vm-backend")]
            vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service,
        ))
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

    async fn create_worker(
        &self,
        config: &AppConfig,
        worker_type: WorkerType,
        vm_manager: Option<Arc<VmManager>>,
    ) -> Result<Box<dyn WorkerBackend>> {
        match worker_type {
            WorkerType::Wasm => {
                use crate::worker_flawless::FlawlessWorker;
                tracing::info!("Creating Flawless WASM worker via factory");
                let worker = FlawlessWorker::new(&config.flawless.flawless_url).await?;
                Ok(Box::new(worker))
            }
            WorkerType::Firecracker => {
                use crate::worker_microvm::{MicroVmWorker, MicroVmWorkerConfig};

                let vm_manager = vm_manager.ok_or_else(||
                    anyhow::anyhow!("VM Manager required for Firecracker worker")
                )?;

                tracing::info!("Creating MicroVM worker via factory with shared VM Manager");

                let microvm_config = MicroVmWorkerConfig {
                    flake_dir: config.vm.flake_dir.clone(),
                    state_dir: config.vm.state_dir.clone(),
                    max_vms: config.vm.max_concurrent_vms,
                    ephemeral_memory_mb: config.vm.default_memory_mb,
                    service_memory_mb: config.vm.default_memory_mb,
                    default_vcpus: config.vm.default_vcpus,
                    enable_service_vms: false,  // Workers typically don't need service VMs
                    service_vm_queues: vec![],
                };

                let worker = MicroVmWorker::with_vm_manager(microvm_config, vm_manager).await?;
                Ok(Box::new(worker))
            }
        }
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

        async fn create_worker(
            &self,
            config: &AppConfig,
            worker_type: WorkerType,
            vm_manager: Option<Arc<VmManager>>,
        ) -> Result<Box<dyn WorkerBackend>> {
            match worker_type {
                WorkerType::Wasm => {
                    // For tests, create a mock Flawless worker or real one with test URL
                    use crate::worker_flawless::FlawlessWorker;
                    tracing::info!("Creating test Flawless WASM worker via factory");
                    let test_url = if config.flawless.flawless_url.is_empty() {
                        "http://localhost:9999".to_string()
                    } else {
                        config.flawless.flawless_url.clone()
                    };
                    let worker = FlawlessWorker::new(&test_url).await?;
                    Ok(Box::new(worker))
                }
                WorkerType::Firecracker => {
                    use crate::worker_microvm::{MicroVmWorker, MicroVmWorkerConfig};

                    let vm_manager = vm_manager.ok_or_else(||
                        anyhow::anyhow!("VM Manager required for Firecracker worker")
                    )?;

                    tracing::info!("Creating test MicroVM worker via factory");

                    // Test configuration with minimal resources
                    let microvm_config = MicroVmWorkerConfig {
                        flake_dir: config.vm.flake_dir.clone(),
                        state_dir: std::env::temp_dir().join("mvm-ci-test-workers"),
                        max_vms: 2,  // Minimal for testing
                        ephemeral_memory_mb: 256,  // Minimal memory
                        service_memory_mb: 256,
                        default_vcpus: 1,
                        enable_service_vms: false,  // Disabled for tests
                        service_vm_queues: vec![],
                    };

                    let worker = MicroVmWorker::with_vm_manager(microvm_config, vm_manager).await?;
                    Ok(Box::new(worker))
                }
            }
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub use test_factory::TestInfrastructureFactory;
