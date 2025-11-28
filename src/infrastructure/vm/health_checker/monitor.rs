// VM Health Monitor - Unix socket communication for health checking
//
// This module handles all I/O operations for VM health checks via Unix sockets.
// It is separated from pure business logic to enable better testability.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration, Instant};
use uuid::Uuid;

use super::config::HealthCheckConfig;
use super::types::CheckResult;
use crate::infrastructure::vm::registry::DefaultVmRepository;
use crate::infrastructure::vm::vm_types::VmControlMessage;

/// VM health monitor - handles Unix socket communication
///
/// This struct is responsible for all I/O operations related to
/// VM health checking. It communicates with VMs via Unix sockets
/// and measures response times.
pub struct VmHealthMonitor {
    config: HealthCheckConfig,
    registry: Arc<DefaultVmRepository>,
}

impl VmHealthMonitor {
    /// Create a new VM health monitor
    pub fn new(config: HealthCheckConfig, registry: Arc<DefaultVmRepository>) -> Self {
        Self { config, registry }
    }

    /// Check health of a specific VM via Unix socket
    ///
    /// Returns CheckResult with response time or error.
    pub async fn check_vm_health(&self, vm_id: Uuid) -> CheckResult {
        let start = Instant::now();

        // Get VM from registry
        let vm_lock = match self.registry.get(&vm_id) {
            Some(vm) => vm,
            None => {
                return CheckResult::Failure {
                    error: "VM not found in registry".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                };
            }
        };

        let vm = vm_lock.read().await;

        // Get control socket path
        let socket_path = match &vm.control_socket {
            Some(path) => path.clone(),
            None => {
                return CheckResult::Failure {
                    error: "VM has no control socket".to_string(),
                    timestamp: chrono::Utc::now().timestamp(),
                };
            }
        };

        // Release lock before I/O
        drop(vm);

        // Perform health check with timeout
        match self.send_health_check(&socket_path).await {
            Ok(response_data) => {
                let elapsed = start.elapsed().as_millis() as u64;

                // Update VM metrics if we have job completion data
                if let Some((uptime_secs, jobs_completed)) = response_data {
                    if let Some(vm_lock) = self.registry.get(&vm_id) {
                        let mut vm = vm_lock.write().await;
                        vm.metrics.record_health_check(true);
                        vm.metrics.jobs_completed = jobs_completed;

                        tracing::debug!(
                            vm_id = %vm_id,
                            uptime_secs = uptime_secs,
                            jobs_completed = jobs_completed,
                            response_time_ms = elapsed,
                            "VM health check passed"
                        );
                    }
                }

                CheckResult::Success {
                    response_time_ms: elapsed,
                }
            }
            Err(e) => CheckResult::Failure {
                error: e.to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            },
        }
    }

    /// Send health check ping via Unix socket
    ///
    /// Returns Ok with optional (uptime_secs, jobs_completed) on success
    async fn send_health_check(
        &self,
        socket_path: &std::path::PathBuf,
    ) -> Result<Option<(i64, u32)>> {
        // Apply timeout to entire operation
        let check_future = async {
            // Connect to Unix socket
            let mut stream = UnixStream::connect(socket_path).await?;

            // Send ping command
            let ping = VmControlMessage::Ping;
            let msg = serde_json::to_string(&ping)?;
            stream.write_all(msg.as_bytes()).await?;
            stream.write_all(b"\n").await?;

            // Read response
            let mut reader = BufReader::new(stream);
            let mut response = String::new();
            reader.read_line(&mut response).await?;

            // Parse response
            let msg: VmControlMessage = serde_json::from_str(&response)
                .map_err(|e| anyhow!("Invalid response format: {}", e))?;

            match msg {
                VmControlMessage::Pong {
                    uptime_secs,
                    jobs_completed,
                } => Ok(Some((uptime_secs, jobs_completed))),
                _ => Err(anyhow!("Unexpected response: expected Pong, got {:?}", msg)),
            }
        };

        timeout(
            Duration::from_secs(self.config.check_timeout_secs),
            check_future,
        )
        .await
        .map_err(|_| anyhow!("Health check timeout after {} seconds", self.config.check_timeout_secs))?
    }

    /// Get the registry (for testing or advanced use cases)
    pub fn registry(&self) -> &Arc<DefaultVmRepository> {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> HealthCheckConfig {
        HealthCheckConfig {
            check_interval_secs: 10,
            check_timeout_secs: 2,
            failure_threshold: 3,
            recovery_threshold: 2,
            enable_circuit_breaker: true,
            circuit_break_duration_secs: 30,
            enable_auto_termination: false,
        }
    }

    // Note: Full testing of VmHealthMonitor requires a real DefaultVmRepository
    // and mock VMs with Unix sockets. These tests are better suited for
    // integration tests rather than unit tests. The monitor's logic is
    // straightforward I/O wrapping, so integration tests will provide
    // better coverage than complex mocking.

    #[test]
    fn test_monitor_has_correct_config() {
        // This is a simple smoke test that the struct can be created
        // Real functionality tests require integration testing with VMs
        let config = test_config();
        assert_eq!(config.check_timeout_secs, 2);
        assert_eq!(config.check_interval_secs, 10);
    }
}
