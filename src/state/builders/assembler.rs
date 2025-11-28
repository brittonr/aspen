// State Assembler - Assembles final state containers
//
// Responsibility:
// - Take components from all builders and assemble into final state containers
// - Creates the 4 focused state containers: DomainState, InfraState, ConfigState, FeaturesState

use flawless_utils::DeployedModule;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::state::{ConfigState, DomainState, FeaturesState, InfraState};

use super::components::{
    DomainServiceComponents, FeatureComponents, InfrastructureComponents, RepositoryComponents,
};

pub struct StateAssembler<'a> {
    infra: InfrastructureComponents,
    _repos: RepositoryComponents,
    services: DomainServiceComponents,
    features: FeatureComponents,
    config: &'a AppConfig,
    module: Option<DeployedModule>,
}

impl<'a> StateAssembler<'a> {
    pub fn new(
        infra: InfrastructureComponents,
        repos: RepositoryComponents,
        services: DomainServiceComponents,
        features: FeatureComponents,
        config: &'a AppConfig,
        module: Option<DeployedModule>,
    ) -> Self {
        Self {
            infra,
            _repos: repos,
            services,
            features,
            config,
            module,
        }
    }

    /// Assemble all state containers
    pub fn build(self) -> (DomainState, InfraState, ConfigState, FeaturesState) {
        let domain = DomainState::new(
            self.services.cluster_status,
            self.services.health,
            self.services.job_commands,
            self.services.job_queries,
            self.services.worker_management,
        );

        let infra = InfraState::new(
            self.infra.iroh,
            self.infra.hiqlite,
            self.infra.node_id,
            self.infra.execution_registry,
        );

        let auth_config = Arc::new(self.config.auth.clone());
        let config = ConfigState::new(auth_config, self.module);

        let features = FeaturesState::new(
            #[cfg(feature = "vm-backend")]
            self.features.vm_service,
            #[cfg(feature = "tofu-support")]
            self.features.tofu_service,
        );

        (domain, infra, config, features)
    }
}
