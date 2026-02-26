//! VM health monitoring: API socket readiness, stderr capture, boot state polling.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::ManagedCiVm;
use crate::error::CloudHypervisorError;
use crate::error::Result;
use crate::error::{self};

impl ManagedCiVm {
    /// Wait for API socket with process health monitoring.
    ///
    /// This method waits for the Cloud Hypervisor API socket to become available,
    /// but also monitors the cloud-hypervisor process health. If the process dies
    /// before the socket appears, it captures and logs stderr for debugging.
    pub(super) async fn wait_for_socket_with_health_check(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(100);
        let api_socket = self.config.api_socket_path(&self.id);

        while tokio::time::Instant::now() < deadline {
            // Check if cloud-hypervisor process is still running
            let process_alive = {
                let mut guard = self.process.write().await;
                if let Some(ref mut child) = *guard {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Process exited - capture stderr and report
                            drop(guard); // Release lock before reading stderr
                            let stderr_output = self.capture_stderr().await;
                            let serial_log = self.config.serial_log_path(&self.id);

                            error!(
                                vm_id = %self.id,
                                exit_status = ?status,
                                stderr = %stderr_output,
                                serial_log = %serial_log.display(),
                                "cloud-hypervisor process exited unexpectedly"
                            );

                            return Err(CloudHypervisorError::CreateVmFailed {
                                reason: format!("cloud-hypervisor exited with {}: {}", status, stderr_output),
                            });
                        }
                        Ok(None) => true, // Still running
                        Err(e) => {
                            warn!(vm_id = %self.id, error = ?e, "failed to check process status");
                            true // Assume running
                        }
                    }
                } else {
                    false // No process
                }
            };

            if !process_alive {
                return Err(CloudHypervisorError::CreateVmFailed {
                    reason: "cloud-hypervisor process not found".to_string(),
                });
            }

            // Check if socket is ready
            if api_socket.exists() && UnixStream::connect(&api_socket).await.is_ok() {
                debug!(vm_id = %self.id, "API socket is ready");
                return Ok(());
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Timeout - capture any available stderr
        let stderr_output = self.capture_stderr().await;
        let serial_log = self.config.serial_log_path(&self.id);

        error!(
            vm_id = %self.id,
            timeout_ms = timeout.as_millis(),
            stderr = %stderr_output,
            serial_log = %serial_log.display(),
            "cloud-hypervisor API socket timeout"
        );

        error::SocketTimeoutSnafu {
            path: api_socket,
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }

    /// Capture any available stderr output from cloud-hypervisor.
    pub(super) async fn capture_stderr(&self) -> String {
        let mut guard = self.process_stderr.write().await;
        if let Some(ref mut stderr) = *guard {
            let mut buffer = Vec::new();
            // Read with a short timeout to avoid blocking
            let read_future = stderr.read_to_end(&mut buffer);
            match tokio::time::timeout(Duration::from_millis(500), read_future).await {
                Ok(Ok(_)) => String::from_utf8_lossy(&buffer).to_string(),
                Ok(Err(e)) => format!("<read error: {}>", e),
                Err(_) => "<timeout reading stderr>".to_string(),
            }
        } else {
            "<no stderr handle>".to_string()
        }
    }

    /// Wait for VM to reach Running state.
    pub(super) async fn wait_for_vm_running(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(500);
        let mut last_state = String::new();
        let mut polls = 0u32;

        while tokio::time::Instant::now() < deadline {
            polls += 1;
            match self.api.vm_info().await {
                Ok(info) => {
                    // Log state changes at info level for visibility
                    if info.state != last_state {
                        info!(vm_id = %self.id, state = %info.state, polls = polls, "VM state changed");
                        last_state = info.state.clone();
                    } else if polls.is_multiple_of(20) {
                        // Log every 10 seconds (20 polls * 500ms) if still waiting
                        info!(vm_id = %self.id, state = %info.state, polls = polls, elapsed_s = polls / 2, "still waiting for VM Running state");
                    }
                    if info.state == "Running" {
                        return Ok(());
                    }
                }
                Err(e) => {
                    if polls.is_multiple_of(20) {
                        warn!(vm_id = %self.id, error = ?e, polls = polls, "API query failed while waiting for VM");
                    } else {
                        debug!(vm_id = %self.id, error = ?e, "waiting for VM info");
                    }
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        error::BootTimeoutSnafu {
            vm_id: self.id.clone(),
            timeout_ms: timeout.as_millis() as u64,
        }
        .fail()
    }
}
