// Worker Factory - Creates worker instances
//
// Responsibility:
// - Create Flawless WASM workers
// - Create Firecracker MicroVM workers
//
// This is a standalone utility, separate from application state building.

use anyhow::Result;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::infrastructure::vm::VmManager;
use crate::{WorkerBackend, WorkerType};

pub struct WorkerFactory;

impl WorkerFactory {
    /// Create a worker instance based on type
    pub async fn create(
        config: &AppConfig,
        worker_type: WorkerType,
        vm_manager: Option<Arc<VmManager>>,
    ) -> Result<Box<dyn WorkerBackend>> {
        match worker_type {
            WorkerType::Wasm => Self::create_wasm_worker(config).await,
            WorkerType::Firecracker => Self::create_firecracker_worker(config, vm_manager).await,
        }
    }

    async fn create_wasm_worker(config: &AppConfig) -> Result<Box<dyn WorkerBackend>> {
        use crate::worker_flawless::FlawlessWorker;
        tracing::info!("Creating Flawless WASM worker");
        let worker = FlawlessWorker::new(&config.flawless.flawless_url).await?;
        Ok(Box::new(worker))
    }

    async fn create_firecracker_worker(
        config: &AppConfig,
        vm_manager: Option<Arc<VmManager>>,
    ) -> Result<Box<dyn WorkerBackend>> {
        use crate::worker_microvm::{MicroVmWorker, MicroVmWorkerConfig};

        let vm_manager = vm_manager
            .ok_or_else(|| anyhow::anyhow!("VM Manager required for Firecracker worker"))?;

        tracing::info!("Creating MicroVM worker with shared VM Manager");

        let microvm_config = MicroVmWorkerConfig {
            flake_dir: config.vm.flake_dir.clone(),
            state_dir: config.vm.state_dir.clone(),
            max_vms: config.vm.max_concurrent_vms,
            ephemeral_memory_mb: config.vm.default_memory_mb,
            service_memory_mb: config.vm.default_memory_mb,
            default_vcpus: config.vm.default_vcpus,
            enable_service_vms: false,
            service_vm_queues: vec![],
        };

        let worker = MicroVmWorker::with_vm_manager(microvm_config, vm_manager).await?;
        Ok(Box::new(worker))
    }
}
