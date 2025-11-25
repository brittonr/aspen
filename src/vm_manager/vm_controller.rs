// VM Controller - Manages VM lifecycle operations

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use super::vm_registry::VmRegistry;
use super::vm_types::{VmConfig, VmControlMessage, VmInstance, VmMode, VmState};
use crate::vm_manager::VmManagerConfig;

/// Controller for VM lifecycle operations
pub struct VmController {
    config: VmManagerConfig,
    registry: Arc<VmRegistry>,
    /// Semaphore to limit concurrent VMs
    semaphore: Arc<Semaphore>,
}

impl VmController {
    /// Create new VM controller
    pub fn new(config: VmManagerConfig, registry: Arc<VmRegistry>) -> Result<Self> {
        let semaphore = Arc::new(Semaphore::new(config.max_vms));

        Ok(Self {
            config,
            registry,
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

    /// Allocate an IP address for the VM
    /// Simple static allocation: 192.168.100.X where X is based on a hash of the VM ID
    fn allocate_ip_address(&self, vm_id: Uuid) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        vm_id.hash(&mut hasher);
        let hash = hasher.finish();

        // Use hash to generate last octet (2-254 to avoid network/broadcast addresses)
        let last_octet = 2 + (hash % 253) as u8;
        format!("192.168.100.{}", last_octet)
    }

    /// Start ephemeral VM (one job then terminate)
    async fn start_ephemeral_vm(&self, vm: &mut VmInstance, job_id: &str) -> Result<()> {
        let vm_id = vm.config.id;
        let vm_dir = self.config.state_dir.join(format!("vms/{}", vm_id));
        let job_dir = self.config.state_dir.join(format!("jobs/{}", vm_id));

        // Create directories
        tokio::fs::create_dir_all(&vm_dir).await?;
        tokio::fs::create_dir_all(&job_dir).await?;

        vm.job_dir = Some(job_dir.clone());

        // Allocate IP address for the VM
        vm.ip_address = Some(self.allocate_ip_address(vm_id));

        // Get job data (would normally fetch from queue)
        let _job_file = job_dir.join("job.json");

        // Build the microvm runner path
        let runner_path = self.config.flake_dir.join("result/bin/microvm-run");

        // If result symlink doesn't exist, build it first
        if !runner_path.exists() {
            tracing::info!("Building worker VM from flake");
            let build_output = Command::new("nix")
                .args(&[
                    "build",
                    ".#worker-vm",
                    "--out-link",
                    "./result",
                ])
                .current_dir(&self.config.flake_dir)
                .output()
                .await?;

            if !build_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to build worker VM: {}",
                    String::from_utf8_lossy(&build_output.stderr)
                ));
            }
        }

        // Run the VM directly using cloud-hypervisor via microvm-run
        let mut cmd = Command::new(&runner_path);

        // Set environment variables for the VM
        cmd.env("VM_TYPE", "ephemeral");
        cmd.env("VM_ID", vm_id.to_string());
        cmd.env("VM_MEM_MB", vm.config.memory_mb.to_string());
        cmd.env("VM_VCPUS", vm.config.vcpus.to_string());
        cmd.env("JOB_DIR", job_dir.to_str().unwrap());

        // Set working directory to flake directory (where virtiofs sockets are created)
        cmd.current_dir(&self.config.flake_dir);

        // Redirect stdout/stderr to files for debugging
        let stdout_file = tokio::fs::File::create(vm_dir.join("stdout.log")).await?;
        let stderr_file = tokio::fs::File::create(vm_dir.join("stderr.log")).await?;
        cmd.stdout(stdout_file.into_std().await);
        cmd.stderr(stderr_file.into_std().await);

        tracing::info!(
            vm_id = %vm_id,
            runner = %runner_path.display(),
            mem_mb = vm.config.memory_mb,
            vcpus = vm.config.vcpus,
            "Starting ephemeral cloud-hypervisor VM"
        );

        let child = cmd.spawn()?;
        vm.pid = Some(child.id().expect("Failed to get child PID"));

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
        let vm_dir = self.config.state_dir.join(format!("vms/{}", vm_id));
        let job_dir = self.config.state_dir.join(format!("jobs/{}", vm_id));

        // Create directories
        tokio::fs::create_dir_all(&vm_dir).await?;
        tokio::fs::create_dir_all(&job_dir).await?;

        vm.job_dir = Some(job_dir.clone());

        // Allocate IP address for the VM
        vm.ip_address = Some(self.allocate_ip_address(vm_id));

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

        let config_file = vm_dir.join("config.json");
        tokio::fs::write(&config_file, vm_config.to_string()).await?;

        // Build the microvm runner path for service VMs
        let runner_path = self.config.flake_dir.join("result-service/bin/microvm-run");
        let virtiofsd_path = self.config.flake_dir.join("result-service/bin/virtiofsd-run");

        // If result symlink doesn't exist, build it first
        if !runner_path.exists() {
            tracing::info!("Building service VM from flake");
            let build_output = Command::new("nix")
                .args(&[
                    "build",
                    ".#service-vm",
                    "--out-link",
                    "./result-service",
                ])
                .current_dir(&self.config.flake_dir)
                .output()
                .await?;

            if !build_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to build service VM: {}",
                    String::from_utf8_lossy(&build_output.stderr)
                ));
            }
        }

        // Start virtiofsd daemons first (required for Cloud Hypervisor)
        tracing::info!(vm_id = %vm_id, "Starting virtiofsd for VM");

        // Clean up any existing sockets (they'll be created in the flake directory)
        let store_sock = self.config.flake_dir.join("service-vm-virtiofs-store.sock");
        let jobs_sock = self.config.flake_dir.join("service-vm-virtiofs-jobs.sock");
        let _ = tokio::fs::remove_file(&store_sock).await;
        let _ = tokio::fs::remove_file(&jobs_sock).await;

        // Start virtiofsd-run in the project/flake directory (where relative paths resolve correctly)
        let mut virtiofsd_cmd = Command::new(&virtiofsd_path);
        virtiofsd_cmd.current_dir(&self.config.flake_dir);

        // Redirect virtiofsd output to log files
        let virtiofsd_stdout = tokio::fs::File::create(vm_dir.join("virtiofsd.stdout.log")).await?;
        let virtiofsd_stderr = tokio::fs::File::create(vm_dir.join("virtiofsd.stderr.log")).await?;
        virtiofsd_cmd.stdout(virtiofsd_stdout.into_std().await);
        virtiofsd_cmd.stderr(virtiofsd_stderr.into_std().await);

        let virtiofsd_child = virtiofsd_cmd.spawn()?;
        let virtiofsd_pid = virtiofsd_child.id().expect("Failed to get virtiofsd PID");

        tracing::info!(vm_id = %vm_id, pid = virtiofsd_pid, "Started virtiofsd daemon");

        // Wait for virtiofs sockets to be created
        let mut retries = 0;
        loop {
            if store_sock.exists() && jobs_sock.exists() {
                tracing::info!(vm_id = %vm_id, "virtiofs sockets ready");
                break;
            }
            if retries >= 30 {
                return Err(anyhow!("virtiofs sockets not created after 15 seconds"));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            retries += 1;
        }

        // Store virtiofsd PID for cleanup later (you may want to add this to VmInstance struct)
        // vm.virtiofsd_pid = Some(virtiofsd_pid);

        // Run the VM directly using cloud-hypervisor via microvm-run
        let mut cmd = Command::new(&runner_path);

        // Set environment variables for the VM
        cmd.env("VM_TYPE", "service");
        cmd.env("VM_ID", vm_id.to_string());
        cmd.env("VM_MEM_MB", vm.config.memory_mb.to_string());
        cmd.env("VM_VCPUS", vm.config.vcpus.to_string());
        cmd.env("CONTROL_SOCKET", control_socket.to_str().unwrap());
        cmd.env("JOB_DIR", job_dir.to_str().unwrap());

        // Set working directory to flake directory (where virtiofs sockets are created)
        cmd.current_dir(&self.config.flake_dir);

        // Redirect stdout/stderr to files for debugging
        let stdout_file = tokio::fs::File::create(vm_dir.join("stdout.log")).await?;
        let stderr_file = tokio::fs::File::create(vm_dir.join("stderr.log")).await?;
        cmd.stdout(stdout_file.into_std().await);
        cmd.stderr(stderr_file.into_std().await);

        tracing::info!(
            vm_id = %vm_id,
            runner = %runner_path.display(),
            mem_mb = vm.config.memory_mb,
            vcpus = vm.config.vcpus,
            queue = queue_name,
            "Starting service cloud-hypervisor VM"
        );

        let child = cmd.spawn()?;
        let pid = child.id().expect("Failed to get child PID");
        vm.pid = Some(pid);

        // Wait for VM to be ready
        self.wait_for_vm_ready(&control_socket).await?;

        // Update state
        vm.state = VmState::Ready;

        tracing::info!(
            vm_id = %vm_id,
            pid = pid,
            "Service VM started and ready"
        );

        Ok(())
    }

    /// Wait for VM to become ready
    async fn wait_for_vm_ready(&self, control_socket: &PathBuf) -> Result<()> {
        let timeout_duration = Duration::from_secs(30);

        timeout(timeout_duration, async {
            loop {
                // Try to connect to control socket
                match UnixStream::connect(control_socket).await {
                    Ok(mut stream) => {
                        // Send ping
                        let ping = VmControlMessage::Ping;
                        let msg = serde_json::to_string(&ping)?;
                        stream.write_all(msg.as_bytes()).await?;
                        stream.write_all(b"\n").await?;

                        // Wait for pong
                        let mut reader = BufReader::new(stream);
                        let mut response = String::new();
                        reader.read_line(&mut response).await?;

                        if let Ok(msg) = serde_json::from_str::<VmControlMessage>(&response) {
                            if matches!(msg, VmControlMessage::Pong { .. }) {
                                return Ok(());
                            }
                        }
                    }
                    Err(_) => {
                        // Socket not ready yet
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        })
        .await
        .map_err(|_| anyhow!("Timeout waiting for VM to be ready"))?
    }

    /// Send job to service VM
    pub async fn send_job_to_vm(&self, vm_id: Uuid, job: &crate::Job) -> Result<()> {
        if let Some(vm_lock) = self.registry.get(vm_id).await? {
            let vm = vm_lock.read().await;

            if let Some(control_socket) = &vm.control_socket {
                // Connect to control socket
                let mut stream = UnixStream::connect(control_socket).await?;

                // Send job execution command
                let message = VmControlMessage::ExecuteJob {
                    job: serde_json::to_value(job)?,
                };

                let msg = serde_json::to_string(&message)?;
                stream.write_all(msg.as_bytes()).await?;
                stream.write_all(b"\n").await?;

                // Wait for acknowledgment
                let mut reader = BufReader::new(stream);
                let mut response = String::new();
                reader.read_line(&mut response).await?;

                if let Ok(msg) = serde_json::from_str::<VmControlMessage>(&response) {
                    match msg {
                        VmControlMessage::Ack => {
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
                        }
                        VmControlMessage::Error { message } => {
                            Err(anyhow!("VM rejected job: {}", message))
                        }
                        _ => Err(anyhow!("Unexpected response from VM")),
                    }
                } else {
                    Err(anyhow!("Invalid response from VM"))
                }
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
                    if let Ok(mut stream) = UnixStream::connect(control_socket).await {
                        let message = VmControlMessage::Shutdown { timeout_secs: 30 };
                        let msg = serde_json::to_string(&message)?;
                        let _ = stream.write_all(msg.as_bytes()).await;
                        let _ = stream.write_all(b"\n").await;

                        // Wait for VM to terminate
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }

            // Force kill if still running
            if let Some(pid) = vm.pid {
                let pid = nix::unistd::Pid::from_raw(pid as i32);
                let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM);

                // Give it a moment to terminate
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Force kill if still running
                let _ = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL);
            }

            // Clean up directories
            if let Some(job_dir) = &vm.job_dir {
                let _ = tokio::fs::remove_dir_all(job_dir).await;
            }

            let vm_dir = self.config.state_dir.join(format!("vms/{}", vm_id));
            let _ = tokio::fs::remove_dir_all(vm_dir).await;

            // Update state
            drop(vm); // Release read lock
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
            vm.config.clone()
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
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, None).is_ok()
    }

    /// Stop a VM (public method)
    pub async fn stop_vm(&self, vm_id: Uuid) -> Result<()> {
        self.shutdown_vm(vm_id, true).await
    }
}