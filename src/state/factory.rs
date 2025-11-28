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
    ClusterStatusService, HealthService, JobCommandService, JobQueryService,
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
use crate::state::{ConfigState, DomainState, FeaturesState, InfraState};
use crate::infrastructure::vm::{VmManager, VmManagerConfig, VmManagement};
use crate::{WorkerBackend, WorkerType};

/// Helper struct to group domain services
struct DomainServices {
    cluster_status: Arc<ClusterStatusService>,
    health: Arc<HealthService>,
    job_commands: Arc<JobCommandService>,
    job_queries: Arc<JobQueryService>,
    worker_management: Arc<WorkerManagementService>,
}

/// Helper struct to group optional features
struct OptionalFeatures {
    #[cfg(feature = "vm-backend")]
    vm_service: Option<Arc<VmService>>,
    #[cfg(feature = "tofu-support")]
    tofu_service: Option<Arc<TofuService>>,
}

/// Builder for constructing focused state containers
///
/// This builder contains all infrastructure and domain services needed
/// to construct the four focused state containers (DomainState, InfraState,
/// ConfigState, FeaturesState).
pub struct StateBuilder {
    // Infrastructure components
    iroh: IrohService,
    hiqlite: HiqliteService,
    node_id: String,
    execution_registry: Arc<ExecutionRegistry>,

    // Configuration
    auth_config: Arc<crate::config::AuthConfig>,
    module: Option<DeployedModule>,

    // Domain services
    cluster_status: Arc<ClusterStatusService>,
    health: Arc<HealthService>,
    job_commands: Arc<JobCommandService>,
    job_queries: Arc<JobQueryService>,
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
            self.job_commands,
            self.job_queries,
            self.worker_management,
        );

        let infra = InfraState::new(
            self.iroh,
            self.hiqlite,
            self.node_id,
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
            self.job_commands,
            self.job_queries,
            self.worker_management,
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

    /// Create VM manager for orchestrating microvms
    async fn create_vm_manager(
        &self,
        config: &AppConfig,
        hiqlite: Arc<HiqliteService>,
    ) -> Result<Arc<VmManager>>;

    /// Create state repository abstraction
    fn create_state_repository(&self, hiqlite: Arc<HiqliteService>) -> Arc<dyn StateRepository>;

    /// Create work repository using new clean architecture (no WorkQueue)
    async fn create_work_repository_impl(
        &self,
        hiqlite: Arc<HiqliteService>,
    ) -> Result<Arc<dyn WorkRepository>>;

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
        // Phase 1: Initialize core infrastructure
        let (hiqlite, iroh) = self.init_infrastructure(
            config,
            endpoint.clone(),
            node_id.clone()
        ).await?;

        let hiqlite_arc = Arc::new(hiqlite.clone());

        // Phase 2: Create execution infrastructure
        let (execution_registry, vm_manager) = self.init_execution_infrastructure(
            config,
            hiqlite_arc.clone()
        ).await?;

        // Phase 3: Create repositories
        let (state_repo, work_repo, worker_repo) = self.init_repositories(
            hiqlite_arc.clone()
        ).await?;

        // Phase 4: Initialize domain services
        let services = self.init_domain_services(
            state_repo,
            work_repo.clone(),
            worker_repo
        );

        // Phase 5: Create optional features
        let optional_features = self.init_optional_features(
            config,
            #[cfg(feature = "vm-backend")]
            vm_manager,
            execution_registry.clone(),
            hiqlite_arc.clone()
        );

        // Phase 6: Final assembly
        self.assemble_state_builder(
            config,
            module,
            hiqlite,
            iroh,
            node_id,
            execution_registry,
            services,
            optional_features
        )
    }

    /// Phase 1: Initialize core infrastructure services
    async fn init_infrastructure(
        &self,
        config: &AppConfig,
        endpoint: Endpoint,
        node_id: String,
    ) -> Result<(HiqliteService, IrohService)> {
        // Create and initialize Hiqlite database
        let hiqlite = self.create_hiqlite(config).await?;
        hiqlite.initialize_schema().await?;

        // Create P2P networking service
        let iroh = self.create_iroh(config, endpoint.clone()).await?;

        Ok((hiqlite, iroh))
    }

    /// Phase 2: Create execution infrastructure (registry and VM manager)
    async fn init_execution_infrastructure(
        &self,
        config: &AppConfig,
        hiqlite_arc: Arc<HiqliteService>,
    ) -> Result<(Arc<ExecutionRegistry>, Option<Arc<dyn VmManagement>>)> {
        // Create execution registry
        let registry_config = RegistryConfig::default();
        let execution_registry = Arc::new(ExecutionRegistry::new(registry_config));

        // Create VM manager if enabled
        let vm_manager = if !config.features.is_vm_manager_enabled() {
            tracing::info!("Skipping VM manager creation (feature disabled in config)");
            None
        } else {
            match self.create_vm_manager(config, hiqlite_arc).await {
                Ok(manager) => Some(manager as Arc<dyn VmManagement>),
                Err(e) => {
                    tracing::warn!(
                        "Failed to create VM manager: {}. Continuing without VM support.",
                        e
                    );
                    None
                }
            }
        };

        Ok((execution_registry, vm_manager))
    }

    /// Phase 3: Create repository layer
    async fn init_repositories(
        &self,
        hiqlite_arc: Arc<HiqliteService>,
    ) -> Result<(
        Arc<dyn StateRepository>,
        Arc<dyn WorkRepository>,
        Arc<dyn WorkerRepository>
    )> {
        let state_repo = self.create_state_repository(hiqlite_arc.clone());

        // NEW: Use clean architecture repository instead of WorkQueue wrapper
        let work_repo = self.create_work_repository_impl(hiqlite_arc.clone()).await?;

        let worker_repo = self.create_worker_repository(hiqlite_arc);

        Ok((state_repo, work_repo, worker_repo))
    }

    /// Phase 4: Initialize domain services with CQRS pattern
    fn init_domain_services(
        &self,
        state_repo: Arc<dyn StateRepository>,
        work_repo: Arc<dyn WorkRepository>,
        worker_repo: Arc<dyn WorkerRepository>,
    ) -> DomainServices {
        // Create event publisher
        let event_publisher = Arc::new(LoggingEventPublisher::new());

        // Create CQRS services
        let commands = Arc::new(JobCommandService::with_events(
            work_repo.clone(),
            event_publisher.clone(),
        ));
        let queries = Arc::new(JobQueryService::new(work_repo.clone()));

        // Create domain services
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let health = Arc::new(HealthService::new(state_repo));

        let worker_management = Arc::new(WorkerManagementService::new(
            worker_repo,
            work_repo,
            Some(60), // Default heartbeat timeout: 60 seconds
        ));

        DomainServices {
            cluster_status,
            health,
            job_commands: commands,
            job_queries: queries,
            worker_management,
        }
    }

    /// Phase 5: Initialize optional/feature-gated services
    fn init_optional_features(
        &self,
        config: &AppConfig,
        #[cfg(feature = "vm-backend")]
        vm_manager: Option<Arc<dyn VmManagement>>,
        execution_registry: Arc<ExecutionRegistry>,
        hiqlite_arc: Arc<HiqliteService>,
    ) -> OptionalFeatures {
        #[cfg(feature = "vm-backend")]
        let vm_service = vm_manager.map(|vm_manager| {
            Arc::new(VmService::new(vm_manager))
        });

        #[cfg(feature = "tofu-support")]
        let tofu_service = Some(Arc::new(TofuService::new(
            execution_registry,
            hiqlite_arc,
            config.storage.work_dir.clone(),
        )));

        OptionalFeatures {
            #[cfg(feature = "vm-backend")]
            vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service,
        }
    }

    /// Phase 6: Assemble final StateBuilder
    fn assemble_state_builder(
        &self,
        config: &AppConfig,
        module: Option<DeployedModule>,
        hiqlite: HiqliteService,
        iroh: IrohService,
        node_id: String,
        execution_registry: Arc<ExecutionRegistry>,
        services: DomainServices,
        optional: OptionalFeatures,
    ) -> Result<StateBuilder> {
        let auth_config = Arc::new(config.auth.clone());

        Ok(StateBuilder {
            iroh,
            hiqlite,
            node_id,
            execution_registry,
            auth_config,
            module,
            cluster_status: services.cluster_status,
            health: services.health,
            job_commands: services.job_commands,
            job_queries: services.job_queries,
            worker_management: services.worker_management,
            #[cfg(feature = "vm-backend")]
            vm_service: optional.vm_service,
            #[cfg(feature = "tofu-support")]
            tofu_service: optional.tofu_service,
        })
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

    /// Create work repository using new clean architecture
    ///
    /// This replaces the old WorkQueue God Object with WorkRepositoryImpl,
    /// which is a focused repository layer with no business logic or caching.
    async fn create_work_repository_impl(
        &self,
        hiqlite: Arc<HiqliteService>,
    ) -> Result<Arc<dyn WorkRepository>> {
        use crate::work::WorkRepositoryImpl;
        use crate::hiqlite_persistent_store::HiqlitePersistentStore;

        let persistent_store = Arc::new(HiqlitePersistentStore::new(hiqlite.as_ref().clone()));
        Ok(Arc::new(WorkRepositoryImpl::new(persistent_store)))
    }

    // Legacy method - kept for backward compatibility during migration
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

        async fn create_work_repository_impl(
            &self,
            _hiqlite: Arc<HiqliteService>,
        ) -> Result<Arc<dyn WorkRepository>> {
            // Return mock repository for testing
            Ok(Arc::new(MockWorkRepository::new()))
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
