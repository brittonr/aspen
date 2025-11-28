// Health Checker - Coordinator for VM health monitoring system
//
// This is a clean coordinator that orchestrates specialized components:
// - HealthStateMachine: Pure state transition logic (no I/O)
// - CircuitBreaker: Circuit breaker pattern implementation
// - VmHealthMonitor: Unix socket communication with VMs
// - SideEffectExecutor: Side effect execution (logging, registry updates)
// - HealthStats: Metrics aggregation and fleet health statistics
//
// The coordinator maintains the health status map and delegates all
// specialized operations to focused components. This architecture enables:
// - Pure testable state logic separated from I/O
// - Composable components with single responsibilities
// - Clear separation of concerns
// - Easy unit testing of individual components
//
// For state transition details, see: src/infrastructure/vm/health_checker/state_machine.rs

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use uuid::Uuid;

use super::super::registry::DefaultVmRepository as VmRegistry;
use super::{
    CircuitBreaker, CircuitBreakerMetrics, CircuitState, HealthCheckConfig, HealthStats,
    HealthStateMachine, HealthStatus, SideEffect, SideEffectExecutor, StateTransition,
    VmHealthMonitor,
};

/// Health checker coordinator for VMs
///
/// Orchestrates specialized health checking components to monitor VM fleet health.
/// Acts as a thin coordinator that delegates to focused components for each concern.
pub struct HealthChecker {
    registry: Arc<VmRegistry>,
    health_status: Arc<RwLock<HashMap<Uuid, HealthStatus>>>,
    config: HealthCheckConfig,
    /// Pure state transition logic (no I/O)
    state_machine: HealthStateMachine,
    /// Circuit breaker for preventing cascading failures
    circuit_breaker: Arc<CircuitBreaker>,
    /// Unix socket communication with VMs
    monitor: VmHealthMonitor,
    /// Side effect execution (logging, registry updates)
    side_effect_executor: SideEffectExecutor,
    stopped: Arc<RwLock<bool>>,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(registry: Arc<VmRegistry>, config: HealthCheckConfig) -> Self {
        let state_machine = HealthStateMachine::new(config.clone());
        let circuit_breaker = Arc::new(CircuitBreaker::new(config.clone()));
        let monitor = VmHealthMonitor::new(config.clone(), registry.clone());
        let side_effect_executor = SideEffectExecutor::new(registry.clone());
        Self {
            registry,
            health_status: Arc::new(RwLock::new(HashMap::new())),
            config,
            state_machine,
            circuit_breaker,
            monitor,
            side_effect_executor,
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
        let vms = self.registry.list_running_vms().await;

        for vm in vms {
            // Skip ephemeral VMs (they terminate after job)
            if matches!(
                vm.config.mode,
                super::super::vm_types::VmMode::Ephemeral { .. }
            ) {
                continue;
            }

            // Check if circuit breaker is open
            if self.is_circuit_open(vm.config.id).await {
                continue;
            }

            // Perform health check using monitor
            let check_result = self.monitor.check_vm_health(vm.config.id).await;

            // Convert CheckResult to Result and response time
            let (result, response_time) = match check_result {
                super::CheckResult::Success { response_time_ms } => (Ok(()), response_time_ms),
                super::CheckResult::Failure { error, .. } => (Err(anyhow!(error)), 0),
            };

            // Update health status
            self.update_health_status(vm.config.id, result, response_time)
                .await;
        }

        Ok(())
    }

    /// Update health status based on check result
    async fn update_health_status(&self, vm_id: Uuid, result: Result<()>, response_time_ms: u64) {
        let mut status_map = self.health_status.write().await;
        let current = status_map.entry(vm_id).or_insert(HealthStatus::Unknown);

        let transition = match &result {
            Ok(()) => {
                // Record success with circuit breaker
                self.circuit_breaker.record_success(vm_id).await;
                self.handle_successful_check(current, response_time_ms)
            }
            Err(e) => {
                // Record failure with circuit breaker
                self.circuit_breaker.record_failure(vm_id).await;
                self.handle_failed_check(current, &e.to_string())
            }
        };

        // Update state
        *current = transition.new_state;

        // Execute side effect if present
        if let Some(action) = transition.side_effect {
            drop(status_map);
            self.execute_side_effect(vm_id, action).await;
        }
    }

    /// Handle successful health check - compute state transition
    /// Delegates to HealthStateMachine for pure logic
    fn handle_successful_check(
        &self,
        current: &HealthStatus,
        response_time_ms: u64,
    ) -> StateTransition {
        self.state_machine
            .handle_successful_check(current, response_time_ms)
    }

    /// Handle failed health check - compute state transition
    /// Delegates to HealthStateMachine for pure logic
    fn handle_failed_check(&self, current: &HealthStatus, error: &str) -> StateTransition {
        self.state_machine.handle_failed_check(current, error)
    }

    /// Execute side effects from state transitions
    /// Delegates to SideEffectExecutor for all I/O operations
    async fn execute_side_effect(&self, vm_id: Uuid, effect: SideEffect) {
        self.side_effect_executor.execute(vm_id, effect).await;
    }

    /// Check if circuit breaker is open for a VM
    /// Delegates to CircuitBreaker for enhanced state management
    async fn is_circuit_open(&self, vm_id: Uuid) -> bool {
        self.circuit_breaker.is_open(vm_id).await
    }

    /// Get health status of a VM
    pub async fn get_health_status(&self, vm_id: Uuid) -> HealthStatus {
        let status_map = self.health_status.read().await;
        status_map
            .get(&vm_id)
            .cloned()
            .unwrap_or(HealthStatus::Unknown)
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
        // Use monitor for health check
        let check_result = self.monitor.check_vm_health(vm_id).await;

        // Convert CheckResult to Result and response time
        let (result, response_time) = match check_result {
            super::CheckResult::Success { response_time_ms } => (Ok(()), response_time_ms),
            super::CheckResult::Failure { error, .. } => (Err(anyhow::anyhow!(error)), 0),
        };

        self.update_health_status(vm_id, result, response_time)
            .await;

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
    /// Delegates to HealthStats for metrics aggregation
    pub async fn get_stats(&self) -> HealthStats {
        let status_map = self.health_status.read().await;
        HealthStats::from_status_map(&status_map)
    }

    /// Get circuit breaker state for a VM
    /// Exposes underlying circuit breaker state for monitoring
    pub async fn get_circuit_state(&self, vm_id: Uuid) -> CircuitState {
        self.circuit_breaker.get_state(vm_id).await
    }

    /// Reset circuit breaker for a VM
    /// Useful for manual recovery after fixing underlying issues
    pub async fn reset_circuit(&self, vm_id: Uuid) {
        self.circuit_breaker.reset(vm_id).await;
    }

    /// Get circuit breaker metrics across all VMs
    /// Provides visibility into circuit breaker system health
    pub async fn get_circuit_metrics(&self) -> CircuitBreakerMetrics {
        self.circuit_breaker.get_metrics().await
    }

    /// Get the configuration used by this health checker
    pub fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}
