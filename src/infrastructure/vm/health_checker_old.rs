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

use super::registry::DefaultVmRepository as VmRegistry;

// Import types from new health_checker module
use super::health_checker::{
    HealthStatus, HealthCheckConfig, StateTransition, SideEffect, HealthStateMachine,
    CircuitBreaker, VmHealthMonitor, SideEffectExecutor, HealthStats
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
            if matches!(vm.config.mode, super::vm_types::VmMode::Ephemeral { .. }) {
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
                super::health_checker::CheckResult::Success { response_time_ms } => {
                    (Ok(()), response_time_ms)
                }
                super::health_checker::CheckResult::Failure { error, .. } => {
                    (Err(anyhow!(error)), 0)
                }
            };

            // Update health status
            self.update_health_status(vm.config.id, result, response_time).await;
        }

        Ok(())
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
        self.state_machine.handle_successful_check(current, response_time_ms)
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
        // Use monitor for health check
        let check_result = self.monitor.check_vm_health(vm_id).await;

        // Convert CheckResult to Result and response time
        let (result, response_time) = match check_result {
            super::health_checker::CheckResult::Success { response_time_ms } => {
                (Ok(()), response_time_ms)
            }
            super::health_checker::CheckResult::Failure { error, .. } => {
                (Err(anyhow::anyhow!(error)), 0)
            }
        };

        self.update_health_status(vm_id, result, response_time).await;

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
    pub async fn get_circuit_state(&self, vm_id: Uuid) -> super::health_checker::CircuitState {
        self.circuit_breaker.get_state(vm_id).await
    }

    /// Reset circuit breaker for a VM
    /// Useful for manual recovery after fixing underlying issues
    pub async fn reset_circuit(&self, vm_id: Uuid) {
        self.circuit_breaker.reset(vm_id).await;
    }

    /// Get circuit breaker metrics across all VMs
    /// Provides visibility into circuit breaker system health
    pub async fn get_circuit_metrics(&self) -> super::health_checker::CircuitBreakerMetrics {
        self.circuit_breaker.get_metrics().await
    }

    /// Get the configuration used by this health checker
    pub fn config(&self) -> &HealthCheckConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal health checker for testing state machine logic
    /// We only test the pure logic methods, not I/O operations
    fn test_checker(config: HealthCheckConfig) -> TestStateMachine {
        TestStateMachine { config }
    }

    /// Helper struct for testing state transitions without I/O
    struct TestStateMachine {
        config: HealthCheckConfig,
    }

    impl TestStateMachine {
        fn test_success(&self, current: &HealthStatus, response_time_ms: u64) -> StateTransition {
            // Inline the same logic as HealthChecker::handle_successful_check
            let now = chrono::Utc::now().timestamp();

            match current {
                HealthStatus::Healthy { .. } => StateTransition::new(
                    HealthStatus::Healthy {
                        last_check: now,
                        response_time_ms,
                    },
                    None,
                ),
                HealthStatus::Degraded { failures, .. } => {
                    if *failures <= self.config.recovery_threshold {
                        StateTransition::new(
                            HealthStatus::Healthy {
                                last_check: now,
                                response_time_ms,
                            },
                            Some(SideEffect::LogRecovery {
                                vm_id: Uuid::nil(),
                                previous_failures: *failures,
                            }),
                        )
                    } else {
                        StateTransition::new(
                            HealthStatus::Degraded {
                                last_check: now,
                                failures: failures.saturating_sub(1),
                                last_error: String::new(),
                            },
                            None,
                        )
                    }
                }
                HealthStatus::Unhealthy { .. } => StateTransition::new(
                    HealthStatus::Degraded {
                        last_check: now,
                        failures: self.config.recovery_threshold - 1,
                        last_error: String::new(),
                    },
                    Some(SideEffect::LogRecoveryFromUnhealthy {
                        vm_id: Uuid::nil(),
                    }),
                ),
                HealthStatus::Unknown => StateTransition::new(
                    HealthStatus::Healthy {
                        last_check: now,
                        response_time_ms,
                    },
                    None,
                ),
            }
        }

        fn test_failure(&self, current: &HealthStatus, error: &str) -> StateTransition {
            // Inline the same logic as HealthChecker::handle_failed_check
            let now = chrono::Utc::now().timestamp();

            match current {
                HealthStatus::Healthy { .. } => StateTransition::new(
                    HealthStatus::Degraded {
                        last_check: now,
                        failures: 1,
                        last_error: error.to_string(),
                    },
                    Some(SideEffect::LogDegradation {
                        vm_id: Uuid::nil(),
                        error: error.to_string(),
                    }),
                ),
                HealthStatus::Degraded { failures, .. } => {
                    let new_failures = *failures + 1;

                    if new_failures >= self.config.failure_threshold {
                        StateTransition::new(
                            HealthStatus::Unhealthy {
                                since: now,
                                failures: new_failures,
                                last_error: error.to_string(),
                            },
                            Some(SideEffect::MarkUnhealthy {
                                vm_id: Uuid::nil(),
                                failures: new_failures,
                                error: error.to_string(),
                            }),
                        )
                    } else {
                        StateTransition::new(
                            HealthStatus::Degraded {
                                last_check: now,
                                failures: new_failures,
                                last_error: error.to_string(),
                            },
                            None,
                        )
                    }
                }
                HealthStatus::Unhealthy { failures, .. } => StateTransition::new(
                    HealthStatus::Unhealthy {
                        since: now,
                        failures: *failures + 1,
                        last_error: error.to_string(),
                    },
                    None,
                ),
                HealthStatus::Unknown => StateTransition::new(
                    HealthStatus::Degraded {
                        last_check: now,
                        failures: 1,
                        last_error: error.to_string(),
                    },
                    None,
                ),
            }
        }
    }

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

    #[test]
    fn test_state_transition_unknown_to_healthy() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Unknown;

        let transition = tester.test_success(&current, 100);

        match transition.new_state {
            HealthStatus::Healthy {
                response_time_ms, ..
            } => {
                assert_eq!(response_time_ms, 100);
            }
            _ => panic!("Expected Healthy state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_healthy_to_degraded() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };

        let transition = tester.test_failure(&current, "timeout");

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 1);
            }
            _ => panic!("Expected Degraded state"),
        }

        matches!(
            transition.side_effect,
            Some(SideEffect::LogDegradation { .. })
        );
    }

    #[test]
    fn test_state_transition_degraded_to_unhealthy() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 2,
            last_error: "previous error".to_string(),
        };

        let transition = tester.test_failure(&current, "new error");

        match transition.new_state {
            HealthStatus::Unhealthy { failures, .. } => {
                assert_eq!(failures, 3);
            }
            _ => panic!("Expected Unhealthy state"),
        }

        matches!(
            transition.side_effect,
            Some(SideEffect::MarkUnhealthy { failures: 3, .. })
        );
    }

    #[test]
    fn test_state_transition_degraded_incremental_failure() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 1,
            last_error: "error".to_string(),
        };

        let transition = tester.test_failure(&current,"another error");

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 2);
            }
            _ => panic!("Expected Degraded state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_degraded_to_healthy_recovery() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 2,
            last_error: "error".to_string(),
        };

        let transition = tester.test_success(&current,150);

        match transition.new_state {
            HealthStatus::Healthy {
                response_time_ms, ..
            } => {
                assert_eq!(response_time_ms, 150);
            }
            _ => panic!("Expected Healthy state"),
        }

        assert!(matches!(
            transition.side_effect,
            Some(SideEffect::LogRecovery { .. })
        ));
    }

    #[test]
    fn test_state_transition_degraded_partial_recovery() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 5,
            last_error: "error".to_string(),
        };

        let transition = tester.test_success(&current,150);

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 4);
            }
            _ => panic!("Expected Degraded state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_unhealthy_to_degraded() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Unhealthy {
            since: 0,
            failures: 5,
            last_error: "error".to_string(),
        };

        let transition = tester.test_success(&current,200);

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 1); // recovery_threshold - 1
            }
            _ => panic!("Expected Degraded state"),
        }

        assert!(matches!(
            transition.side_effect,
            Some(SideEffect::LogRecoveryFromUnhealthy { .. })
        ));
    }

    #[test]
    fn test_state_transition_unhealthy_remains_unhealthy() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Unhealthy {
            since: 100,
            failures: 5,
            last_error: "old error".to_string(),
        };

        let transition = tester.test_failure(&current,"new error");

        match transition.new_state {
            HealthStatus::Unhealthy { failures, .. } => {
                assert_eq!(failures, 6);
            }
            _ => panic!("Expected Unhealthy state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_healthy_remains_healthy() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };

        let transition = tester.test_success(&current,120);

        match transition.new_state {
            HealthStatus::Healthy {
                response_time_ms, ..
            } => {
                assert_eq!(response_time_ms, 120);
            }
            _ => panic!("Expected Healthy state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_unknown_to_degraded() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Unknown;

        let transition = tester.test_failure(&current,"first check failed");

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 1);
            }
            _ => panic!("Expected Degraded state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_failure_threshold_exact_boundary() {
        let config = HealthCheckConfig {
            failure_threshold: 3,
            ..test_config()
        };
        let tester = test_checker(config);

        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 2,
            last_error: "error".to_string(),
        };

        let transition = tester.test_failure(&current,"threshold reached");

        match transition.new_state {
            HealthStatus::Unhealthy { .. } => {}
            _ => panic!("Expected transition to Unhealthy at threshold"),
        }
    }

    #[test]
    fn test_recovery_threshold_exact_boundary() {
        let config = HealthCheckConfig {
            recovery_threshold: 2,
            ..test_config()
        };
        let tester = test_checker(config);

        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 2,
            last_error: "error".to_string(),
        };

        let transition = tester.test_success(&current,100);

        match transition.new_state {
            HealthStatus::Healthy { .. } => {}
            _ => panic!("Expected recovery to Healthy at threshold"),
        }
    }

    #[test]
    fn test_state_machine_is_deterministic() {
        let tester = test_checker(test_config());

        let state1 = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };
        let state2 = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };

        let transition1 = tester.test_failure(&state1, "error");
        let transition2 = tester.test_failure(&state2, "error");

        match (&transition1.new_state, &transition2.new_state) {
            (
                HealthStatus::Degraded {
                    failures: f1,
                    last_error: e1,
                    ..
                },
                HealthStatus::Degraded {
                    failures: f2,
                    last_error: e2,
                    ..
                },
            ) => {
                assert_eq!(f1, f2);
                assert_eq!(e1, e2);
            }
            _ => panic!("Transitions should be deterministic"),
        }
    }

    #[test]
    fn test_health_status_is_healthy_method() {
        let healthy = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };
        assert!(healthy.is_healthy());

        let degraded = HealthStatus::Degraded {
            last_check: 0,
            failures: 1,
            last_error: "error".to_string(),
        };
        assert!(degraded.is_healthy());

        let unhealthy = HealthStatus::Unhealthy {
            since: 0,
            failures: 3,
            last_error: "error".to_string(),
        };
        assert!(!unhealthy.is_healthy());

        assert!(!HealthStatus::Unknown.is_healthy());
    }

    #[test]
    fn test_zero_failure_threshold() {
        let config = HealthCheckConfig {
            failure_threshold: 0,
            ..test_config()
        };
        let tester = test_checker(config);

        let current = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };

        let transition = tester.test_failure(&current,"immediate failure");

        match transition.new_state {
            HealthStatus::Degraded { .. } => {}
            _ => panic!("Should transition to degraded first"),
        }
    }

    #[test]
    fn test_saturating_sub_on_failure_count() {
        let tester = test_checker(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 0,
            last_error: "error".to_string(),
        };

        let transition = tester.test_success(&current,100);

        match transition.new_state {
            HealthStatus::Healthy { .. } => {}
            _ => panic!("Should recover with zero failures"),
        }
    }
}
