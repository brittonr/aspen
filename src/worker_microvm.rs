// MicroVM Worker Backend - Orchestrated through VM Manager
//
// This implementation properly uses the VM Manager for complete orchestration
// of both ephemeral and service VMs with proper lifecycle management

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::vm_manager::{VmManager, VmManagerConfig};
use crate::vm_manager::vm_types::{IsolationLevel, VmConfig, VmMode};
use crate::worker_trait::{WorkerBackend, WorkResult};
use crate::Job;

/// Configuration for the orchestrated MicroVM worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroVmWorkerConfig {
    /// Base directory for VM state
    pub state_dir: PathBuf,

    /// Path to microvm flake directory
    pub flake_dir: PathBuf,

    /// Maximum number of concurrent VMs
    pub max_vms: usize,

    /// Default memory for ephemeral VMs (MB)
    pub ephemeral_memory_mb: u32,

    /// Default memory for service VMs (MB)
    pub service_memory_mb: u32,

    /// Default vCPUs for VMs
    pub default_vcpus: u32,

    /// Enable service VMs for queue processing
    pub enable_service_vms: bool,

    /// Service VM configuration
    pub service_vm_queues: Vec<String>,
}

impl Default for MicroVmWorkerConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            state_dir: PathBuf::from(format!("{}/mvm-ci-orchestrated", home)),
            flake_dir: PathBuf::from("./microvms"),
            max_vms: 10,
            ephemeral_memory_mb: 512,
            service_memory_mb: 1024,
            default_vcpus: 2,
            enable_service_vms: true,
            service_vm_queues: vec!["default".to_string(), "builds".to_string()],
        }
    }
}

/// Worker backend that properly orchestrates MicroVMs through VM Manager
pub struct MicroVmWorker {
    config: MicroVmWorkerConfig,
    vm_manager: Arc<VmManager>,
    /// Track which service VMs we've started
    service_vms: Arc<RwLock<Vec<Uuid>>>,
}

impl MicroVmWorker {
    /// Create a new orchestrated MicroVM worker
    pub async fn new(config: MicroVmWorkerConfig) -> Result<Self> {
        // Create VM Manager configuration
        let vm_config = VmManagerConfig {
            max_vms: config.max_vms,
            state_dir: config.state_dir.clone(),
            flake_dir: config.flake_dir.clone(),
            database_path: config.state_dir.join("vms.db"),
            ephemeral_memory_mb: config.ephemeral_memory_mb,
            ephemeral_vcpus: config.default_vcpus,
            service_memory_mb: config.service_memory_mb,
            service_vcpus: config.default_vcpus,
            health_check_interval_secs: 30,
            resource_check_interval_secs: 10,
            max_ephemeral_uptime_secs: 300,
            max_service_uptime_secs: 3600,
        };

        // Create VM Manager
        let vm_manager = Arc::new(VmManager::new(vm_config).await?);

        // Start VM manager background tasks
        let manager_clone = vm_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = manager_clone.start_monitoring().await {
                tracing::error!(error = ?e, "VM monitoring failed");
            }
        });

        let service_vms = Arc::new(RwLock::new(Vec::new()));

        // Start service VMs if enabled
        if config.enable_service_vms {
            let mut vms = service_vms.write().await;
            for queue_name in &config.service_vm_queues {
                tracing::info!(queue = %queue_name, "Starting service VM for queue");

                let vm_config = VmConfig {
                    id: Uuid::new_v4(),
                    mode: VmMode::Service {
                        queue_name: queue_name.clone(),
                        max_jobs: Some(100),
                        max_uptime_secs: Some(3600),
                    },
                    isolation_level: IsolationLevel::Standard,
                    memory_mb: config.service_memory_mb,
                    vcpus: config.default_vcpus,
                    kernel_image: None, // Use default from flake
                    rootfs_image: None, // Use default from flake
                    network_mode: "user".to_string(),
                    metadata: Default::default(),
                };

                match vm_manager.start_vm(vm_config.clone()).await {
                    Ok(vm_instance) => {
                        tracing::info!(
                            vm_id = %vm_instance.config.id,
                            queue = %queue_name,
                            "Service VM started successfully"
                        );
                        vms.push(vm_instance.config.id);
                    }
                    Err(e) => {
                        tracing::error!(
                            queue = %queue_name,
                            error = ?e,
                            "Failed to start service VM"
                        );
                    }
                }
            }
        }

        tracing::info!(
            state_dir = ?config.state_dir,
            max_vms = config.max_vms,
            service_vms = config.enable_service_vms,
            "MicroVM worker initialized with VM Manager orchestration"
        );

        Ok(Self {
            config,
            vm_manager,
            service_vms,
        })
    }

    /// Determine isolation level based on job metadata
    fn determine_isolation_level(&self, job: &Job) -> IsolationLevel {
        // Check job metadata for isolation requirements
        if let Some(isolation) = job.metadata.get("isolation_level") {
            match isolation.as_str() {
                Some("strict") => return IsolationLevel::Strict,
                Some("trusted") => return IsolationLevel::Trusted,
                _ => {}
            }
        }

        // Check for untrusted sources
        if job.metadata.get("untrusted").and_then(|v| v.as_bool()).unwrap_or(false) {
            return IsolationLevel::Strict;
        }

        IsolationLevel::Standard
    }

    /// Check if a job should use a service VM
    fn should_use_service_vm(&self, job: &Job) -> bool {
        // Don't use service VMs if disabled
        if !self.config.enable_service_vms {
            return false;
        }

        // Strict isolation always gets ephemeral VM
        if self.determine_isolation_level(job) == IsolationLevel::Strict {
            return false;
        }

        // Check if job explicitly requests ephemeral
        if job.metadata.get("ephemeral").and_then(|v| v.as_bool()).unwrap_or(false) {
            return false;
        }

        // Default to service VM for standard jobs
        true
    }

    /// Health check for the VM worker
    pub async fn health_check(&self) -> Result<()> {
        // Check VM Manager health
        let stats = self.vm_manager.get_stats().await?;

        tracing::debug!(
            total_vms = stats.total_vms,
            ready_vms = stats.ready_vms,
            busy_vms = stats.busy_vms,
            failed_vms = stats.failed_vms,
            "VM Manager health check"
        );

        // Ensure we have healthy VMs
        if stats.total_vms == 0 {
            return Err(anyhow!("No VMs running"));
        }

        if stats.failed_vms > stats.total_vms / 2 {
            return Err(anyhow!(
                "Too many failed VMs: {}/{}",
                stats.failed_vms,
                stats.total_vms
            ));
        }

        Ok(())
    }

    /// Shutdown the VM worker and all VMs
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down MicroVM worker");

        // Shutdown all service VMs
        let vms = self.service_vms.read().await;
        for vm_id in vms.iter() {
            tracing::info!(vm_id = %vm_id, "Stopping service VM");
            if let Err(e) = self.vm_manager.stop_vm(*vm_id).await {
                tracing::error!(vm_id = %vm_id, error = ?e, "Failed to stop service VM");
            }
        }

        // Shutdown VM Manager
        self.vm_manager.shutdown().await?;

        Ok(())
    }
}

#[async_trait]
impl WorkerBackend for MicroVmWorker {
    async fn execute(&self, job: Job) -> WorkResult {
        tracing::info!(
            job_id = %job.id,
            job_type = %job.job_type,
            "Executing job through VM Manager"
        );

        let isolation_level = self.determine_isolation_level(&job);
        let use_service = self.should_use_service_vm(&job);

        if use_service {
            // Route to service VM through VM Manager
            tracing::info!(job_id = %job.id, "Routing job to service VM");

            match self.vm_manager.submit_job(job.clone()).await {
                Ok(result) => {
                    tracing::info!(
                        job_id = %job.id,
                        vm_id = %result.vm_id,
                        "Job completed via service VM"
                    );

                    WorkResult {
                        success: result.success,
                        output: result.output,
                        error: result.error,
                        metadata: result.metadata,
                    }
                }
                Err(e) => {
                    tracing::error!(job_id = %job.id, error = ?e, "Service VM execution failed");
                    WorkResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Service VM execution failed: {}", e)),
                        metadata: Default::default(),
                    }
                }
            }
        } else {
            // Create ephemeral VM for this job
            tracing::info!(
                job_id = %job.id,
                isolation = ?isolation_level,
                "Creating ephemeral VM for job"
            );

            let vm_config = VmConfig {
                id: Uuid::new_v4(),
                mode: VmMode::Ephemeral {
                    job_id: job.id.clone(),
                },
                isolation_level,
                memory_mb: self.config.ephemeral_memory_mb,
                vcpus: self.config.default_vcpus,
                kernel_image: None,
                rootfs_image: None,
                network_mode: match isolation_level {
                    IsolationLevel::Strict => "none".to_string(),
                    _ => "user".to_string(),
                },
                metadata: job.metadata.clone(),
            };

            match self.vm_manager.start_vm(vm_config.clone()).await {
                Ok(vm_instance) => {
                    tracing::info!(
                        job_id = %job.id,
                        vm_id = %vm_instance.config.id,
                        "Ephemeral VM started for job"
                    );

                    // Submit job to the ephemeral VM
                    match self.vm_manager.submit_job_to_vm(vm_instance.config.id, job.clone()).await {
                        Ok(result) => {
                            WorkResult {
                                success: result.success,
                                output: result.output,
                                error: result.error,
                                metadata: result.metadata,
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                job_id = %job.id,
                                vm_id = %vm_instance.config.id,
                                error = ?e,
                                "Failed to execute job in ephemeral VM"
                            );
                            WorkResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Ephemeral VM execution failed: {}", e)),
                                metadata: Default::default(),
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        job_id = %job.id,
                        error = ?e,
                        "Failed to start ephemeral VM"
                    );
                    WorkResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Failed to start ephemeral VM: {}", e)),
                        metadata: Default::default(),
                    }
                }
            }
        }
    }

}