//! VM Controller - Manages VM lifecycle operations

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::Duration;
use uuid::Uuid;

use super::vm_registry::VmRegistry;
use super::vm_types::{VmConfig, VmInstance, VmMode, VmState};
use super::{network_manager::VmNetworkManager, process_manager::VmProcessManager, virtiofs_daemon::VirtiofsDaemon, filesystem::VmFilesystem, control_socket::VmControlSocket};
use crate::infrastructure::vm::VmManagerConfig;

/// Controller for VM lifecycle operations
pub struct VmController {
    config: VmManagerConfig,
    registry: Arc<VmRegistry>,
    network_manager: VmNetworkManager,
    process_manager: VmProcessManager,
    virtiofs_daemon: VirtiofsDaemon,
    filesystem: VmFilesystem,
    control_socket: VmControlSocket,
    /// Semaphore to limit concurrent VMs
    semaphore: Arc<Semaphore>,
}

impl VmController {
    /// Create new VM controller
    pub fn new(config: VmManagerConfig, registry: Arc<VmRegistry>) -> Result<Self> {
        let semaphore = Arc::new(Semaphore::new(config.max_vms));
        let network_manager = VmNetworkManager::new();
        let process_manager = VmProcessManager::new(config.flake_dir.clone());
        let virtiofs_daemon = VirtiofsDaemon::new(config.flake_dir.clone());
        let filesystem = VmFilesystem::new(config.state_dir.clone());
        let control_socket = VmControlSocket::new();

        Ok(Self {
            config,
            registry,
            network_manager,
            process_manager,
            virtiofs_daemon,
            filesystem,
            control_socket,
            semaphore,
        })
    }

    /// Start a new VM
    pub async fn start_vm(&self, config: VmConfig) -> Result<VmInstance> {
        // Acquire semaphore permit
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Failed to acquire VM semaphore: {}", e))?;

        tracing::info!(vm_id = %config.id, mode = ?config.mode, "Starting VM");

        let mut vm = VmInstance::new(config.clone());

        match &config.mode {
            VmMode::Ephemeral { job_id } => {
                self.start_ephemeral_vm(&mut vm, job_id).await?;
            }
            VmMode::Service { queue_name, .. } => {
                self.start_service_vm(&mut vm, queue_name).await?;
            }
        }

        // Register VM
        self.registry.register(vm.clone()).await?;

        // Log event
        self.registry
            .log_event(config.id, "Started", None)
            .await?;

        Ok(vm)
    }

    /// Start ephemeral VM (one job then terminate)
    async fn start_ephemeral_vm(&self, vm: &mut VmInstance, job_id: &str) -> Result<()> {
        let vm_id = vm.config.id;

        // Create directories
        let (vm_dir, job_dir) = self.filesystem.create_vm_directories(vm_id).await?;
        vm.job_dir = Some(job_dir.clone());

        // Allocate IP address
        vm.ip_address = Some(self.network_manager.allocate_ip_address(vm_id));

        // Spawn VM process
        let pid = self.process_manager.spawn_ephemeral_vm(
            vm_id,
            vm.config.memory_mb,
            vm.config.vcpus,
            &job_dir,
            &vm_dir,
        ).await?;

        vm.pid = Some(pid);

        // Update state
        vm.state = VmState::Busy {
            job_id: job_id.to_string(),
            started_at: chrono::Utc::now().timestamp(),
        };

        Ok(())
    }

    /// Start service VM (long-running, multiple jobs)
    async fn start_service_vm(&self, vm: &mut VmInstance, queue_name: &str) -> Result<()> {
        let vm_id = vm.config.id;

        // Create directories
        let (vm_dir, job_dir) = self.filesystem.create_vm_directories(vm_id).await?;
        vm.job_dir = Some(job_dir.clone());

        // Allocate IP address
        vm.ip_address = Some(self.network_manager.allocate_ip_address(vm_id));

        // Create control socket path
        let control_socket = vm_dir.join("control.sock");
        vm.control_socket = Some(control_socket.clone());

        // Generate VM configuration
        let vm_config = serde_json::json!({
            "id": vm_id.to_string(),
            "mode": "service",
            "queue": queue_name,
            "memory_mb": vm.config.memory_mb,
            "vcpus": vm.config.vcpus,
        });

        self.filesystem.write_vm_config(vm_id, &vm_config).await?;

        // Start virtiofsd daemon
        let _virtiofsd_pid = self.virtiofs_daemon.start_virtiofsd(vm_id, &vm_dir).await?;

        // Spawn VM process
        let pid = self.process_manager.spawn_service_vm(
            vm_id,
            vm.config.memory_mb,
            vm.config.vcpus,
            &job_dir,
            &control_socket,
            &vm_dir,
            queue_name,
        ).await?;

        vm.pid = Some(pid);

        // Wait for VM to be ready
        self.control_socket.wait_for_vm_ready(&control_socket).await?;

        // Update state
        vm.state = VmState::Ready;

        tracing::info!(
            vm_id = %vm_id,
            pid = pid,
            "Service VM started and ready"
        );

        Ok(())
    }

    /// Send job to service VM
    pub async fn send_job_to_vm(&self, vm_id: Uuid, job: &crate::Job) -> Result<()> {
        if let Some(vm_lock) = self.registry.get(vm_id).await? {
            let vm = vm_lock.read().await;

            if let Some(control_socket) = &vm.control_socket {
                // Send job via control socket
                self.control_socket.send_job(control_socket, job).await?;

                // Update VM state to busy
                drop(vm); // Release read lock
                self.registry
                    .update_state(
                        vm_id,
                        VmState::Busy {
                            job_id: job.id.clone(),
                            started_at: chrono::Utc::now().timestamp(),
                        },
                    )
                    .await?;

                tracing::debug!(vm_id = %vm_id, job_id = %job.id, "Job sent to VM");
                Ok(())
            } else {
                Err(anyhow!("VM has no control socket"))
            }
        } else {
            Err(anyhow!("VM not found: {}", vm_id))
        }
    }

    /// Shutdown VM
    pub async fn shutdown_vm(&self, vm_id: Uuid, graceful: bool) -> Result<()> {
        tracing::info!(vm_id = %vm_id, graceful = graceful, "Shutting down VM");

        if let Some(vm_lock) = self.registry.get(vm_id).await? {
            let vm = vm_lock.read().await;

            if graceful {
                // Try graceful shutdown via control socket
                if let Some(control_socket) = &vm.control_socket {
                    if let Err(e) = self.control_socket.send_shutdown(control_socket, 30).await {
                        tracing::warn!(vm_id = %vm_id, error = %e, "Failed to send shutdown message to VM");
                    }

                    // Wait for VM to terminate
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }

            // Force kill if still running
            if let Some(pid) = vm.pid {
                if let Err(e) = self.process_manager.send_sigterm(pid) {
                    tracing::warn!(vm_id = %vm_id, pid = pid, error = %e, "Failed to send SIGTERM");
                }

                // Give it a moment to terminate
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Force kill if still running
                if let Err(e) = self.process_manager.send_sigkill(pid) {
                    tracing::warn!(vm_id = %vm_id, pid = pid, error = %e, "Failed to send SIGKILL");
                }
            }

            // Clean up directories
            let job_dir = vm.job_dir.clone();
            drop(vm); // Release lock before filesystem operations

            self.filesystem.cleanup_vm_directories(vm_id, job_dir.as_deref()).await;

            // Update state
            self.registry
                .update_state(
                    vm_id,
                    VmState::Terminated {
                        reason: if graceful {
                            "Graceful shutdown".to_string()
                        } else {
                            "Force terminated".to_string()
                        },
                        exit_code: 0,
                    },
                )
                .await?;

            // Log event
            self.registry
                .log_event(vm_id, "Terminated", None)
                .await?;
        }

        Ok(())
    }

    /// Restart a VM
    pub async fn restart_vm(&self, vm_id: Uuid) -> Result<()> {
        tracing::info!(vm_id = %vm_id, "Restarting VM");

        // Get VM configuration
        let config = if let Some(vm_lock) = self.registry.get(vm_id).await? {
            let vm = vm_lock.read().await;
            vm.config.as_ref().clone()
        } else {
            return Err(anyhow!("VM not found: {}", vm_id));
        };

        // Shutdown existing VM
        self.shutdown_vm(vm_id, false).await?;

        // Remove from registry
        self.registry.remove(vm_id).await?;

        // Start new VM with same config
        self.start_vm(config).await?;

        Ok(())
    }

    /// Get VM status
    pub async fn get_vm_status(&self, vm_id: Uuid) -> Result<Option<VmState>> {
        if let Some(vm_lock) = self.registry.get(vm_id).await? {
            let vm = vm_lock.read().await;
            Ok(Some(vm.state.clone()))
        } else {
            Ok(None)
        }
    }

    /// Check if process is still running
    pub fn is_process_running(&self, pid: u32) -> bool {
        self.process_manager.is_process_running(pid)
    }

    /// Stop a VM (public method)
    pub async fn stop_vm(&self, vm_id: Uuid) -> Result<()> {
        self.shutdown_vm(vm_id, true).await
    }
}
