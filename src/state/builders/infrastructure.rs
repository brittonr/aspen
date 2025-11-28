// Infrastructure Builder - Creates core platform services
//
// Responsibilities:
// - Initialize Hiqlite distributed database
// - Initialize Iroh P2P networking
// - Create VM manager (optional, graceful failure)
// - Create execution registry

use anyhow::Result;
use iroh::Endpoint;
use std::sync::Arc;

use crate::adapters::{ExecutionRegistry, RegistryConfig};
use crate::config::AppConfig;
use crate::hiqlite::HiqliteService;
use crate::infrastructure::vm::{VmManager, VmManagerConfig};
use crate::iroh_service::IrohService;

use super::components::InfrastructureComponents;

pub struct InfrastructureBuilder<'a> {
    config: &'a AppConfig,
    endpoint: Endpoint,
    node_id: String,
}

impl<'a> InfrastructureBuilder<'a> {
    pub fn new(config: &'a AppConfig, endpoint: Endpoint, node_id: String) -> Self {
        Self {
            config,
            endpoint,
            node_id,
        }
    }

    /// Build all infrastructure components
    pub async fn build(self) -> Result<InfrastructureComponents> {
        // Create and initialize Hiqlite database
        let hiqlite = self.build_hiqlite().await?;

        // Create P2P networking service
        let iroh = self.build_iroh();

        // Create execution registry
        let execution_registry = self.build_execution_registry();

        // Create VM manager (optional, can fail gracefully)
        let vm_manager = self.build_vm_manager(&hiqlite).await;

        Ok(InfrastructureComponents {
            hiqlite,
            iroh,
            node_id: self.node_id,
            vm_manager,
            execution_registry,
        })
    }

    async fn build_hiqlite(&self) -> Result<HiqliteService> {
        let hiqlite =
            HiqliteService::new(self.config.storage.hiqlite_data_dir.clone()).await?;
        hiqlite.initialize_schema().await?;
        Ok(hiqlite)
    }

    fn build_iroh(&self) -> IrohService {
        IrohService::new(
            self.config.storage.iroh_blobs_path.clone(),
            self.endpoint.clone(),
        )
    }

    fn build_execution_registry(&self) -> Arc<ExecutionRegistry> {
        let registry_config = RegistryConfig::default();
        Arc::new(ExecutionRegistry::new(registry_config))
    }

    async fn build_vm_manager(&self, hiqlite: &HiqliteService) -> Option<Arc<VmManager>> {
        if !self.config.features.is_vm_manager_enabled() {
            tracing::info!("Skipping VM manager creation (feature disabled in config)");
            return None;
        }

        let vm_config = VmManagerConfig {
            max_vms: self.config.vm.max_concurrent_vms,
            auto_scaling: true,
            pre_warm_count: 2,
            flake_dir: self.config.vm.flake_dir.clone(),
            state_dir: self.config.storage.vm_state_dir.clone(),
            default_memory_mb: self.config.vm.default_memory_mb,
            default_vcpus: self.config.vm.default_vcpus,
        };

        match VmManager::new(vm_config, Arc::new(hiqlite.clone())).await {
            Ok(manager) => Some(Arc::new(manager)),
            Err(e) => {
                tracing::warn!(
                    "Failed to create VM manager: {}. Continuing without VM support.",
                    e
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct TestInfrastructureBuilder<'a> {
        config: &'a AppConfig,
        endpoint: Endpoint,
        node_id: String,
    }

    impl<'a> TestInfrastructureBuilder<'a> {
        pub fn new(config: &'a AppConfig, endpoint: Endpoint, node_id: String) -> Self {
            Self {
                config,
                endpoint,
                node_id,
            }
        }

        pub async fn build(self) -> Result<InfrastructureComponents> {
            // Create in-memory hiqlite for testing
            let hiqlite = HiqliteService::new(None).await?;
            hiqlite.initialize_schema().await?;

            // Use temporary test directory for iroh blobs
            let test_path = std::env::temp_dir().join("blixard-test-iroh");
            let iroh = IrohService::new(test_path, self.endpoint);

            // Test execution registry
            let execution_registry = Arc::new(ExecutionRegistry::new(RegistryConfig::default()));

            // Test VM configuration with minimal resources
            let vm_manager = if self.config.features.is_vm_manager_enabled() {
                let vm_config = VmManagerConfig {
                    max_vms: 3,
                    auto_scaling: false,
                    pre_warm_count: 0,
                    flake_dir: std::path::PathBuf::from("./microvms"),
                    state_dir: std::env::temp_dir().join("blixard-test-vms"),
                    default_memory_mb: 256,
                    default_vcpus: 1,
                };

                match VmManager::new(vm_config, Arc::new(hiqlite.clone())).await {
                    Ok(manager) => Some(Arc::new(manager)),
                    Err(_) => None,
                }
            } else {
                None
            };

            Ok(InfrastructureComponents {
                hiqlite,
                iroh,
                node_id: self.node_id,
                vm_manager,
                execution_registry,
            })
        }
    }
}
