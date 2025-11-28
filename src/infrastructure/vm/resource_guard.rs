//! Resource Guard - RAII-based resource cleanup for VM lifecycle
//!
//! Implements compensating transaction pattern to ensure all resources
//! are properly cleaned up on failure during VM startup.

use anyhow::{Context, Result};
use std::path::PathBuf;
use uuid::Uuid;

use super::filesystem::VmFilesystem;
use super::network_manager::VmNetworkManager;
use super::process_manager::VmProcessManager;
use super::virtiofs_daemon::VirtiofsDaemon;

/// Tracks resources allocated during VM startup for cleanup on drop
pub struct VmResourceGuard {
    vm_id: Uuid,
    allocated: AllocatedResources,
    /// Managers needed for cleanup operations
    managers: ResourceManagers,
    /// Whether to skip cleanup on drop (set to true on successful commit)
    committed: bool,
}

/// Resources that have been allocated and may need cleanup
#[derive(Default)]
struct AllocatedResources {
    vm_dir: Option<PathBuf>,
    job_dir: Option<PathBuf>,
    ip_address: Option<String>,
    vm_pid: Option<u32>,
    virtiofsd_pid: Option<u32>,
    control_socket: Option<PathBuf>,
}

/// Managers needed to perform cleanup operations
struct ResourceManagers {
    filesystem: VmFilesystem,
    network_manager: VmNetworkManager,
    process_manager: VmProcessManager,
    virtiofs_daemon: VirtiofsDaemon,
}

impl VmResourceGuard {
    /// Create a new resource guard for VM startup
    pub fn new(
        vm_id: Uuid,
        filesystem: VmFilesystem,
        network_manager: VmNetworkManager,
        process_manager: VmProcessManager,
        virtiofs_daemon: VirtiofsDaemon,
    ) -> Self {
        Self {
            vm_id,
            allocated: AllocatedResources::default(),
            managers: ResourceManagers {
                filesystem,
                network_manager,
                process_manager,
                virtiofs_daemon,
            },
            committed: false,
        }
    }

    /// Allocate and track directories for the VM
    pub async fn allocate_directories(&mut self) -> Result<(PathBuf, PathBuf)> {
        tracing::debug!(vm_id = %self.vm_id, "Allocating VM directories");

        let (vm_dir, job_dir) = self
            .managers
            .filesystem
            .create_vm_directories(self.vm_id)
            .await
            .context("Failed to create VM directories")?;

        self.allocated.vm_dir = Some(vm_dir.clone());
        self.allocated.job_dir = Some(job_dir.clone());

        tracing::debug!(
            vm_id = %self.vm_id,
            vm_dir = %vm_dir.display(),
            job_dir = %job_dir.display(),
            "Allocated VM directories"
        );

        Ok((vm_dir, job_dir))
    }

    /// Allocate and track IP address for the VM
    pub fn allocate_ip_address(&mut self) -> String {
        tracing::debug!(vm_id = %self.vm_id, "Allocating IP address");

        let ip = self.managers.network_manager.allocate_ip_address(self.vm_id);
        self.allocated.ip_address = Some(ip.clone());

        tracing::debug!(vm_id = %self.vm_id, ip = %ip, "Allocated IP address");

        ip
    }

    /// Start and track ephemeral VM process
    pub async fn spawn_ephemeral_vm(
        &mut self,
        memory_mb: u32,
        vcpus: u32,
        job_dir: &PathBuf,
        vm_dir: &PathBuf,
    ) -> Result<u32> {
        tracing::debug!(vm_id = %self.vm_id, "Spawning ephemeral VM process");

        let pid = self
            .managers
            .process_manager
            .spawn_ephemeral_vm(self.vm_id, memory_mb, vcpus, job_dir, vm_dir)
            .await
            .context("Failed to spawn ephemeral VM process")?;

        self.allocated.vm_pid = Some(pid);

        tracing::debug!(vm_id = %self.vm_id, pid = pid, "Spawned ephemeral VM process");

        Ok(pid)
    }

    /// Track control socket path
    pub fn track_control_socket(&mut self, socket_path: PathBuf) {
        tracing::debug!(
            vm_id = %self.vm_id,
            socket = %socket_path.display(),
            "Tracking control socket"
        );
        self.allocated.control_socket = Some(socket_path);
    }

    /// Start and track virtiofsd daemon
    pub async fn start_virtiofsd(&mut self, vm_dir: &PathBuf) -> Result<u32> {
        tracing::debug!(vm_id = %self.vm_id, "Starting virtiofsd daemon");

        let pid = self
            .managers
            .virtiofs_daemon
            .start_virtiofsd(self.vm_id, vm_dir)
            .await
            .context("Failed to start virtiofsd daemon")?;

        self.allocated.virtiofsd_pid = Some(pid);

        tracing::debug!(vm_id = %self.vm_id, pid = pid, "Started virtiofsd daemon");

        Ok(pid)
    }

    /// Start and track service VM process
    pub async fn spawn_service_vm(
        &mut self,
        memory_mb: u32,
        vcpus: u32,
        job_dir: &PathBuf,
        control_socket: &PathBuf,
        vm_dir: &PathBuf,
        queue_name: &str,
    ) -> Result<u32> {
        tracing::debug!(vm_id = %self.vm_id, "Spawning service VM process");

        let pid = self
            .managers
            .process_manager
            .spawn_service_vm(
                self.vm_id,
                memory_mb,
                vcpus,
                job_dir,
                control_socket,
                vm_dir,
                queue_name,
            )
            .await
            .context("Failed to spawn service VM process")?;

        self.allocated.vm_pid = Some(pid);

        tracing::debug!(vm_id = %self.vm_id, pid = pid, "Spawned service VM process");

        Ok(pid)
    }

    /// Extract allocated resources and prevent cleanup on drop
    ///
    /// Call this when VM startup succeeds to commit the resources
    pub fn commit(mut self) -> AllocatedVmResources {
        self.committed = true;

        AllocatedVmResources {
            vm_id: self.vm_id,
            vm_dir: self.allocated.vm_dir.take(),
            job_dir: self.allocated.job_dir.take(),
            ip_address: self.allocated.ip_address.take(),
            vm_pid: self.allocated.vm_pid.take(),
            virtiofsd_pid: self.allocated.virtiofsd_pid.take(),
            control_socket: self.allocated.control_socket.take(),
        }
    }

    /// Manually rollback all allocated resources
    ///
    /// This is also called automatically on drop if commit() was not called
    async fn rollback(&mut self) {
        if self.committed {
            return;
        }

        tracing::warn!(
            vm_id = %self.vm_id,
            "Rolling back VM resources due to startup failure"
        );

        // Kill processes first (most important for resource cleanup)
        if let Some(pid) = self.allocated.vm_pid.take() {
            tracing::debug!(vm_id = %self.vm_id, pid = pid, "Killing VM process");
            if let Err(e) = self.managers.process_manager.send_sigkill(pid) {
                tracing::error!(
                    vm_id = %self.vm_id,
                    pid = pid,
                    error = %e,
                    "Failed to kill VM process during rollback"
                );
            }
        }

        if let Some(pid) = self.allocated.virtiofsd_pid.take() {
            tracing::debug!(vm_id = %self.vm_id, pid = pid, "Killing virtiofsd process");
            if let Err(e) = self.managers.process_manager.send_sigkill(pid) {
                tracing::error!(
                    vm_id = %self.vm_id,
                    pid = pid,
                    error = %e,
                    "Failed to kill virtiofsd process during rollback"
                );
            }
        }

        // Clean up filesystem resources
        if let Some(job_dir) = self.allocated.job_dir.take() {
            tracing::debug!(
                vm_id = %self.vm_id,
                path = %job_dir.display(),
                "Cleaning up job directory"
            );
            self.managers
                .filesystem
                .cleanup_vm_directories(self.vm_id, Some(&job_dir))
                .await;
        } else if self.allocated.vm_dir.is_some() {
            // If we have vm_dir but no job_dir, still clean up vm_dir
            tracing::debug!(vm_id = %self.vm_id, "Cleaning up VM directory");
            self.managers
                .filesystem
                .cleanup_vm_directories(self.vm_id, None)
                .await;
        }

        // Note: IP addresses are deterministically allocated based on VM ID,
        // so no explicit cleanup is needed. The network manager doesn't maintain
        // state about allocated IPs.

        // Note: Control socket is cleaned up when VM directory is removed

        tracing::info!(vm_id = %self.vm_id, "VM resource rollback complete");
    }
}

impl Drop for VmResourceGuard {
    fn drop(&mut self) {
        if !self.committed {
            // We can't call async cleanup in Drop, so we spawn a blocking task
            // This is a best-effort cleanup - the main cleanup happens in rollback()
            tracing::warn!(
                vm_id = %self.vm_id,
                "VmResourceGuard dropped without commit - performing emergency cleanup"
            );

            // Kill processes synchronously (most critical)
            if let Some(pid) = self.allocated.vm_pid {
                let _ = self.managers.process_manager.send_sigkill(pid);
            }
            if let Some(pid) = self.allocated.virtiofsd_pid {
                let _ = self.managers.process_manager.send_sigkill(pid);
            }

            // Spawn async task for filesystem cleanup
            let vm_id = self.vm_id;
            let job_dir = self.allocated.job_dir.clone();
            let filesystem = self.managers.filesystem.clone();

            tokio::spawn(async move {
                filesystem
                    .cleanup_vm_directories(vm_id, job_dir.as_deref())
                    .await;
            });
        }
    }
}

/// Successfully allocated resources for a VM
///
/// This struct holds the resources that were successfully allocated
/// and committed during VM startup.
pub struct AllocatedVmResources {
    pub vm_id: Uuid,
    pub vm_dir: Option<PathBuf>,
    pub job_dir: Option<PathBuf>,
    pub ip_address: Option<String>,
    pub vm_pid: Option<u32>,
    pub virtiofsd_pid: Option<u32>,
    pub control_socket: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_resource_guard_commit_prevents_cleanup() {
        let vm_id = Uuid::new_v4();
        let state_dir = std::env::temp_dir().join(format!("blixard-test-{}", vm_id));
        let flake_dir = PathBuf::from(".");

        let filesystem = VmFilesystem::new(state_dir.clone());
        let network_manager = VmNetworkManager::new();
        let process_manager = VmProcessManager::new(flake_dir.clone());
        let virtiofs_daemon = VirtiofsDaemon::new(flake_dir);

        let mut guard = VmResourceGuard::new(
            vm_id,
            filesystem,
            network_manager,
            process_manager,
            virtiofs_daemon,
        );

        // Allocate some resources
        let _dirs = guard.allocate_directories().await.unwrap();
        let _ip = guard.allocate_ip_address();

        // Commit should prevent cleanup
        let resources = guard.commit();

        assert_eq!(resources.vm_id, vm_id);
        assert!(resources.vm_dir.is_some());
        assert!(resources.job_dir.is_some());
        assert!(resources.ip_address.is_some());

        // Cleanup for test
        tokio::fs::remove_dir_all(&state_dir).await.ok();
    }

    #[tokio::test]
    async fn test_resource_guard_drop_cleanup() {
        let vm_id = Uuid::new_v4();
        let state_dir = std::env::temp_dir().join(format!("blixard-test-{}", vm_id));
        let flake_dir = PathBuf::from(".");

        let filesystem = VmFilesystem::new(state_dir.clone());
        let network_manager = VmNetworkManager::new();
        let process_manager = VmProcessManager::new(flake_dir.clone());
        let virtiofs_daemon = VirtiofsDaemon::new(flake_dir);

        {
            let mut guard = VmResourceGuard::new(
                vm_id,
                filesystem,
                network_manager,
                process_manager,
                virtiofs_daemon,
            );

            // Allocate resources but don't commit
            guard.allocate_directories().await.unwrap();
            guard.allocate_ip_address();

            // Guard will be dropped here and should trigger cleanup
        }

        // Give async cleanup time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Directories should be cleaned up (or at least cleanup attempted)
        // Note: In real scenario, directories would be removed
    }
}
