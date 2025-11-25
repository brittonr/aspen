// Health Checker - Monitors VM health and implements circuit breakers

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tokio::time::{interval, timeout, Duration};
use uuid::Uuid;

use super::vm_registry::VmRegistry;
use super::vm_types::{VmControlMessage, VmState};

/// Health status for a VM
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// VM is healthy and responding
    Healthy {
        last_check: i64,
        response_time_ms: u64,
    },
    /// VM is degraded but still functioning
    Degraded {
        last_check: i64,
        failures: u32,
        last_error: String,
    },
    /// VM is not responding (circuit open)
    Unhealthy {
        since: i64,
        failures: u32,
        last_error: String,
    },
    /// VM health unknown (not yet checked)
    Unknown,
}

/// Configuration for health checking
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Interval between health checks
    pub check_interval_secs: u64,
    /// Timeout for health check response
    pub check_timeout_secs: u64,
    /// Number of failures before marking unhealthy
    pub failure_threshold: u32,
    /// Number of successes to recover from unhealthy
    pub recovery_threshold: u32,
    /// Enable circuit breaker behavior
    pub enable_circuit_breaker: bool,
    /// Time to wait before retrying unhealthy VM
    pub circuit_break_duration_secs: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            check_timeout_secs: 5,
            failure_threshold: 3,
            recovery_threshold: 2,
            enable_circuit_breaker: true,
            circuit_break_duration_secs: 60,
        }
    }
}

/// Health checker for VMs
pub struct HealthChecker {
    registry: Arc<VmRegistry>,
    health_status: Arc<RwLock<HashMap<Uuid, HealthStatus>>>,
    config: HealthCheckConfig,
    stopped: Arc<RwLock<bool>>,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(registry: Arc<VmRegistry>, config: HealthCheckConfig) -> Self {
        Self {
            registry,
            health_status: Arc::new(RwLock::new(HashMap::new())),
            config,
            stopped: Arc::new(RwLock::new(false)),
        }
    }

    /// Start health checking loop
    pub async fn health_check_loop(&self) {
        self.start_monitoring().await
    }

    /// Start health monitoring (public for compatibility)
    pub async fn start_monitoring(&self) {
        let mut interval = interval(Duration::from_secs(self.config.check_interval_secs));

        loop {
            interval.tick().await;

            if *self.stopped.read().await {
                tracing::info!("Health checker stopped");
                break;
            }

            if let Err(e) = self.check_all_vms().await {
                tracing::error!(error = %e, "Health check cycle failed");
            }
        }
    }

    /// Check health of all VMs
    async fn check_all_vms(&self) -> Result<()> {
        let vms = self.registry.list_running_vms().await?;

        for vm in vms {
            // Skip ephemeral VMs (they terminate after job)
            if matches!(vm.config.mode, super::vm_types::VmMode::Ephemeral { .. }) {
                continue;
            }

            // Check if circuit breaker is open
            if self.is_circuit_open(vm.config.id).await {
                continue;
            }

            // Perform health check
            let start = std::time::Instant::now();
            let result = self.check_vm_health(&vm).await;
            let response_time = start.elapsed().as_millis() as u64;

            // Update health status
            self.update_health_status(vm.config.id, result, response_time).await;
        }

        Ok(())
    }

    /// Check individual VM health
    async fn check_vm_health(&self, vm: &super::vm_types::VmInstance) -> Result<()> {
        // Skip if no control socket
        let control_socket = vm.control_socket.as_ref()
            .ok_or_else(|| anyhow!("No control socket"))?;

        // Try to connect and ping
        let check_future = async {
            let mut stream = UnixStream::connect(control_socket).await?;

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
                match msg {
                    VmControlMessage::Pong { uptime_secs, jobs_completed } => {
                        tracing::debug!(
                            vm_id = %vm.config.id,
                            uptime_secs = uptime_secs,
                            jobs_completed = jobs_completed,
                            "VM health check passed"
                        );

                        // Update VM metrics
                        let mut vm_lock = self.registry.get(vm.config.id).await?
                            .ok_or_else(|| anyhow!("VM not found"))?;
                        let mut vm = vm_lock.write().await;
                        vm.metrics.record_health_check(true);
                        vm.metrics.jobs_completed = jobs_completed;

                        Ok(())
                    }
                    _ => Err(anyhow!("Unexpected response"))
                }
            } else {
                Err(anyhow!("Invalid response"))
            }
        };

        // Apply timeout
        timeout(
            Duration::from_secs(self.config.check_timeout_secs),
            check_future
        ).await
            .map_err(|_| anyhow!("Health check timeout"))?
    }

    /// Update health status based on check result
    async fn update_health_status(
        &self,
        vm_id: Uuid,
        result: Result<()>,
        response_time_ms: u64,
    ) {
        let mut status_map = self.health_status.write().await;
        let current = status_map.entry(vm_id).or_insert(HealthStatus::Unknown);

        match result {
            Ok(()) => {
                // Health check passed
                match current {
                    HealthStatus::Healthy { .. } => {
                        // Still healthy
                        *current = HealthStatus::Healthy {
                            last_check: chrono::Utc::now().timestamp(),
                            response_time_ms,
                        };
                    }
                    HealthStatus::Degraded { failures, .. } => {
                        // Recovering from degraded
                        if *failures <= self.config.recovery_threshold {
                            *current = HealthStatus::Healthy {
                                last_check: chrono::Utc::now().timestamp(),
                                response_time_ms,
                            };
                            tracing::info!(vm_id = %vm_id, "VM recovered to healthy");
                        } else {
                            *current = HealthStatus::Degraded {
                                last_check: chrono::Utc::now().timestamp(),
                                failures: failures.saturating_sub(1),
                                last_error: String::new(),
                            };
                        }
                    }
                    HealthStatus::Unhealthy { .. } => {
                        // Recovering from unhealthy
                        *current = HealthStatus::Degraded {
                            last_check: chrono::Utc::now().timestamp(),
                            failures: self.config.recovery_threshold - 1,
                            last_error: String::new(),
                        };
                        tracing::info!(vm_id = %vm_id, "VM recovering from unhealthy");
                    }
                    HealthStatus::Unknown => {
                        // First successful check
                        *current = HealthStatus::Healthy {
                            last_check: chrono::Utc::now().timestamp(),
                            response_time_ms,
                        };
                    }
                }
            }
            Err(e) => {
                // Health check failed
                let error_msg = e.to_string();

                match current {
                    HealthStatus::Healthy { .. } => {
                        // First failure
                        *current = HealthStatus::Degraded {
                            last_check: chrono::Utc::now().timestamp(),
                            failures: 1,
                            last_error: error_msg.clone(),
                        };
                        tracing::warn!(
                            vm_id = %vm_id,
                            error = %error_msg,
                            "VM health check failed, marking degraded"
                        );
                    }
                    HealthStatus::Degraded { failures, .. } => {
                        // Additional failure
                        let new_failures = *failures + 1;
                        if new_failures >= self.config.failure_threshold {
                            // Mark as unhealthy
                            *current = HealthStatus::Unhealthy {
                                since: chrono::Utc::now().timestamp(),
                                failures: new_failures,
                                last_error: error_msg.clone(),
                            };

                            // Update VM state in registry
                            if let Err(e) = self.mark_vm_unhealthy(vm_id).await {
                                tracing::error!(
                                    vm_id = %vm_id,
                                    error = %e,
                                    "Failed to mark VM as unhealthy in registry"
                                );
                            }

                            tracing::error!(
                                vm_id = %vm_id,
                                failures = new_failures,
                                error = %error_msg,
                                "VM marked as unhealthy after {} failures",
                                new_failures
                            );
                        } else {
                            *current = HealthStatus::Degraded {
                                last_check: chrono::Utc::now().timestamp(),
                                failures: new_failures,
                                last_error: error_msg,
                            };
                        }
                    }
                    HealthStatus::Unhealthy { failures, .. } => {
                        // Still unhealthy
                        *current = HealthStatus::Unhealthy {
                            since: chrono::Utc::now().timestamp(),
                            failures: *failures + 1,
                            last_error: error_msg,
                        };
                    }
                    HealthStatus::Unknown => {
                        // First check failed
                        *current = HealthStatus::Degraded {
                            last_check: chrono::Utc::now().timestamp(),
                            failures: 1,
                            last_error: error_msg,
                        };
                    }
                }

                // Record failed health check in metrics
                if let Ok(Some(vm_lock)) = self.registry.get(vm_id).await {
                    let mut vm = vm_lock.write().await;
                    vm.metrics.record_health_check(false);
                }
            }
        }
    }

    /// Check if circuit breaker is open for a VM
    async fn is_circuit_open(&self, vm_id: Uuid) -> bool {
        if !self.config.enable_circuit_breaker {
            return false;
        }

        let status_map = self.health_status.read().await;
        if let Some(status) = status_map.get(&vm_id) {
            if let HealthStatus::Unhealthy { since, .. } = status {
                let elapsed = chrono::Utc::now().timestamp() - since;
                if elapsed < self.config.circuit_break_duration_secs as i64 {
                    tracing::debug!(
                        vm_id = %vm_id,
                        remaining_secs = self.config.circuit_break_duration_secs as i64 - elapsed,
                        "Circuit breaker open for VM"
                    );
                    return true;
                }
            }
        }

        false
    }

    /// Mark VM as unhealthy in registry
    async fn mark_vm_unhealthy(&self, vm_id: Uuid) -> Result<()> {
        self.registry.update_state(
            vm_id,
            VmState::Failed {
                error: "Health check failures exceeded threshold".to_string(),
            },
        ).await
    }

    /// Get health status of a VM
    pub async fn get_health_status(&self, vm_id: Uuid) -> HealthStatus {
        let status_map = self.health_status.read().await;
        status_map.get(&vm_id).cloned().unwrap_or(HealthStatus::Unknown)
    }

    /// Check if VM is healthy
    pub async fn is_healthy(&self, vm_id: Uuid) -> bool {
        matches!(
            self.get_health_status(vm_id).await,
            HealthStatus::Healthy { .. }
        )
    }

    /// Force health check on specific VM
    pub async fn force_check(&self, vm_id: Uuid) -> Result<()> {
        if let Some(vm_lock) = self.registry.get(vm_id).await? {
            let vm = vm_lock.read().await;

            let start = std::time::Instant::now();
            let result = self.check_vm_health(&vm).await;
            let response_time = start.elapsed().as_millis() as u64;

            self.update_health_status(vm_id, result, response_time).await;
        }

        Ok(())
    }

    /// Reset health status for a VM
    pub async fn reset_health(&self, vm_id: Uuid) {
        let mut status_map = self.health_status.write().await;
        status_map.remove(&vm_id);

        tracing::info!(vm_id = %vm_id, "Health status reset");
    }

    /// Stop health checking
    pub async fn stop(&self) {
        *self.stopped.write().await = true;
    }

    /// Get health statistics
    pub async fn get_stats(&self) -> HealthStats {
        let status_map = self.health_status.read().await;

        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        let mut unknown = 0;

        for status in status_map.values() {
            match status {
                HealthStatus::Healthy { .. } => healthy += 1,
                HealthStatus::Degraded { .. } => degraded += 1,
                HealthStatus::Unhealthy { .. } => unhealthy += 1,
                HealthStatus::Unknown => unknown += 1,
            }
        }

        HealthStats {
            total_vms: status_map.len(),
            healthy,
            degraded,
            unhealthy,
            unknown,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStats {
    pub total_vms: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    pub unknown: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_status_transitions() {
        let registry = Arc::new(
            VmRegistry::new(&std::path::PathBuf::from("/tmp")).await.unwrap()
        );

        let config = HealthCheckConfig {
            failure_threshold: 3,
            recovery_threshold: 2,
            ..Default::default()
        };

        let checker = HealthChecker::new(registry, config);
        let vm_id = Uuid::new_v4();

        // Initial state is Unknown
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            HealthStatus::Unknown
        ));

        // Success -> Healthy
        checker.update_health_status(vm_id, Ok(()), 100).await;
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            HealthStatus::Healthy { .. }
        ));

        // First failure -> Degraded
        checker.update_health_status(
            vm_id,
            Err(anyhow!("Test failure")),
            0
        ).await;
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            HealthStatus::Degraded { failures: 1, .. }
        ));

        // Third failure -> Unhealthy
        checker.update_health_status(
            vm_id,
            Err(anyhow!("Test failure")),
            0
        ).await;
        checker.update_health_status(
            vm_id,
            Err(anyhow!("Test failure")),
            0
        ).await;
        assert!(matches!(
            checker.get_health_status(vm_id).await,
            HealthStatus::Unhealthy { .. }
        ));
    }
}