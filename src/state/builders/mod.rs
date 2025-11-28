// Focused Builders for Application State
//
// This module replaces the old StateFactory God Object with focused builders,
// each with a single, clear responsibility.
//
// Architecture:
// - InfrastructureBuilder: Core platform services (Hiqlite, Iroh, VM Manager)
// - RepositoryBuilder: Data access layer (3 repositories)
// - DomainServiceBuilder: Business logic layer (5 services)
// - FeatureBuilder: Optional/feature-gated services
// - StateAssembler: Final composition of state containers
// - ApplicationBuilder: High-level orchestration
// - WorkerFactory: Standalone worker creation utility

mod assembler;
mod components;
mod domain;
mod feature;
mod infrastructure;
mod repository;
mod worker_factory;

pub use assembler::StateAssembler;
pub use components::{
    DomainServiceComponents, FeatureComponents, InfrastructureComponents, RepositoryComponents,
};
pub use domain::DomainServiceBuilder;
pub use feature::FeatureBuilder;
pub use infrastructure::InfrastructureBuilder;
pub use repository::RepositoryBuilder;
pub use worker_factory::WorkerFactory;

use anyhow::Result;
use flawless_utils::DeployedModule;
use iroh::Endpoint;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::infrastructure::vm::VmManagement;
use crate::state::{ConfigState, DomainState, FeaturesState, InfraState};

/// High-level application builder that orchestrates all focused builders
///
/// This replaces the old InfrastructureFactory::build_state() method.
pub struct ApplicationBuilder<'a> {
    config: &'a AppConfig,
    module: Option<DeployedModule>,
    endpoint: Endpoint,
    node_id: String,
}

impl<'a> ApplicationBuilder<'a> {
    pub fn new(
        config: &'a AppConfig,
        module: Option<DeployedModule>,
        endpoint: Endpoint,
        node_id: String,
    ) -> Self {
        Self {
            config,
            module,
            endpoint,
            node_id,
        }
    }

    /// Build complete application state using focused builders
    ///
    /// This orchestrates the creation of all components in the correct order:
    /// 1. Infrastructure (Hiqlite, Iroh, VM Manager, Execution Registry)
    /// 2. Repositories (StateRepo, WorkRepo, WorkerRepo)
    /// 3. Domain Services (5 business logic services)
    /// 4. Features (VmService, TofuService)
    /// 5. Assembly (4 state containers)
    pub async fn build(self) -> Result<(DomainState, InfraState, ConfigState, FeaturesState)> {
        // Phase 1: Build infrastructure
        let infra = InfrastructureBuilder::new(self.config, self.endpoint, self.node_id)
            .build()
            .await?;

        // Phase 2: Build repositories (needs Hiqlite from infra)
        let hiqlite_arc = Arc::new(infra.hiqlite.clone());
        let repos = RepositoryBuilder::new(hiqlite_arc.clone())
            .build()
            .await?;

        // Phase 3: Build domain services (needs repositories)
        let services =
            DomainServiceBuilder::new(repos.state.clone(), repos.work.clone(), repos.worker.clone())
                .with_heartbeat_timeout(60)
                .build();

        // Phase 4: Build feature services (needs infra components)
        let vm_manager_trait: Option<Arc<dyn VmManagement>> = infra
            .vm_manager
            .as_ref()
            .map(|vm| vm.clone() as Arc<dyn VmManagement>);

        let features = FeatureBuilder::new(
            self.config,
            vm_manager_trait,
            infra.execution_registry.clone(),
            hiqlite_arc,
        )
        .build();

        // Phase 5: Assemble final state containers
        let (domain, infra_state, config, features_state) =
            StateAssembler::new(infra, repos, services, features, self.config, self.module)
                .build();

        Ok((domain, infra_state, config, features_state))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use infrastructure::tests::TestInfrastructureBuilder;
    use repository::tests::TestRepositoryBuilder;

    /// Test application builder using mocks
    pub struct TestApplicationBuilder<'a> {
        config: &'a AppConfig,
        module: Option<DeployedModule>,
        endpoint: Endpoint,
        node_id: String,
    }

    impl<'a> TestApplicationBuilder<'a> {
        pub fn new(
            config: &'a AppConfig,
            module: Option<DeployedModule>,
            endpoint: Endpoint,
            node_id: String,
        ) -> Self {
            Self {
                config,
                module,
                endpoint,
                node_id,
            }
        }

        pub async fn build(
            self,
        ) -> Result<(DomainState, InfraState, ConfigState, FeaturesState)> {
            // Test infrastructure
            let infra = TestInfrastructureBuilder::new(self.config, self.endpoint, self.node_id)
                .build()
                .await?;

            // Mock repositories
            let repos = TestRepositoryBuilder::new().build().await?;

            // Domain services (same as production)
            let services = DomainServiceBuilder::new(
                repos.state.clone(),
                repos.work.clone(),
                repos.worker.clone(),
            )
            .with_heartbeat_timeout(60)
            .build();

            // Features (same as production)
            let hiqlite_arc = Arc::new(infra.hiqlite.clone());
            let vm_manager_trait: Option<Arc<dyn VmManagement>> = infra
                .vm_manager
                .as_ref()
                .map(|vm| vm.clone() as Arc<dyn VmManagement>);

            let features = FeatureBuilder::new(
                self.config,
                vm_manager_trait,
                infra.execution_registry.clone(),
                hiqlite_arc,
            )
            .build();

            // Assemble
            let (domain, infra_state, config, features_state) =
                StateAssembler::new(infra, repos, services, features, self.config, self.module)
                    .build();

            Ok((domain, infra_state, config, features_state))
        }
    }
}
