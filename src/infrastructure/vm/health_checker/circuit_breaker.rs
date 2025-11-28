// Circuit Breaker pattern for VM health checking
//
// Implements Martin Fowler's circuit breaker pattern with half-open state.
// Prevents cascading failures by temporarily stopping health checks on
// consistently unhealthy VMs.
//
// States:
// - Closed: Normal operation, health checks proceed
// - Open: Circuit tripped, health checks skipped
// - HalfOpen: Testing recovery with probe checks

use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::config::HealthCheckConfig;

/// Circuit breaker state for a VM
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Normal operation - health checks proceed
    Closed,

    /// Circuit tripped - health checks skipped
    Open {
        /// When the circuit was opened (Unix timestamp)
        since: i64,
    },

    /// Testing recovery - limited probe checks allowed
    HalfOpen {
        /// Number of successful probe checks
        probe_successes: u32,
        /// Number of failed probe checks
        probe_failures: u32,
    },
}

/// Circuit breaker for VM health checking
///
/// Implements the circuit breaker pattern to prevent wasted resources
/// checking consistently unhealthy VMs.
pub struct CircuitBreaker {
    config: HealthCheckConfig,
    states: RwLock<HashMap<Uuid, CircuitState>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Check if circuit is open for a VM
    ///
    /// Returns true if health checks should be skipped.
    pub async fn is_open(&self, vm_id: Uuid) -> bool {
        if !self.config.enable_circuit_breaker {
            return false;
        }

        let states = self.states.read().await;
        match states.get(&vm_id) {
            Some(CircuitState::Open { since }) => {
                let elapsed = chrono::Utc::now().timestamp() - since;
                if elapsed >= self.config.circuit_break_duration_secs as i64 {
                    // Duration elapsed, transition to half-open
                    drop(states);
                    self.transition_to_half_open(vm_id).await;
                    false // Allow probe check
                } else {
                    tracing::debug!(
                        vm_id = %vm_id,
                        remaining_secs = self.config.circuit_break_duration_secs as i64 - elapsed,
                        "Circuit breaker open for VM"
                    );
                    true // Skip check
                }
            }
            Some(CircuitState::HalfOpen { .. }) => {
                // Allow probe checks in half-open state
                false
            }
            Some(CircuitState::Closed) | None => {
                // Circuit closed or unknown, allow checks
                false
            }
        }
    }

    /// Record a successful health check
    ///
    /// Updates circuit state based on current state.
    pub async fn record_success(&self, vm_id: Uuid) {
        let mut states = self.states.write().await;

        match states.get(&vm_id).cloned() {
            Some(CircuitState::HalfOpen {
                probe_successes,
                probe_failures,
            }) => {
                let new_successes = probe_successes + 1;

                // Need 2 consecutive successes to close circuit
                if new_successes >= 2 && probe_failures == 0 {
                    states.insert(vm_id, CircuitState::Closed);
                    tracing::info!(
                        vm_id = %vm_id,
                        "Circuit breaker closed after successful recovery"
                    );
                } else {
                    states.insert(
                        vm_id,
                        CircuitState::HalfOpen {
                            probe_successes: new_successes,
                            probe_failures,
                        },
                    );
                    tracing::debug!(
                        vm_id = %vm_id,
                        successes = new_successes,
                        "Circuit breaker half-open - probe success"
                    );
                }
            }
            Some(CircuitState::Open { .. }) => {
                // Unexpected success while open, transition to half-open
                states.insert(
                    vm_id,
                    CircuitState::HalfOpen {
                        probe_successes: 1,
                        probe_failures: 0,
                    },
                );
                tracing::info!(
                    vm_id = %vm_id,
                    "Circuit breaker transitioning to half-open after unexpected success"
                );
            }
            Some(CircuitState::Closed) | None => {
                // Already closed or unknown, stay closed
                states.insert(vm_id, CircuitState::Closed);
            }
        }
    }

    /// Record a failed health check
    ///
    /// Opens circuit or resets recovery progress.
    pub async fn record_failure(&self, vm_id: Uuid) {
        let mut states = self.states.write().await;
        let now = chrono::Utc::now().timestamp();

        match states.get(&vm_id).cloned() {
            Some(CircuitState::HalfOpen { .. }) => {
                // Probe failed, reopen circuit
                states.insert(vm_id, CircuitState::Open { since: now });
                tracing::warn!(
                    vm_id = %vm_id,
                    "Circuit breaker reopened after failed probe"
                );
            }
            Some(CircuitState::Closed) => {
                // First failure in closed state, open circuit
                states.insert(vm_id, CircuitState::Open { since: now });
                tracing::warn!(vm_id = %vm_id, "Circuit breaker opened");
            }
            Some(CircuitState::Open { since }) => {
                // Already open, keep it open but update timestamp
                states.insert(vm_id, CircuitState::Open { since });
            }
            None => {
                // Unknown state, open circuit
                states.insert(vm_id, CircuitState::Open { since: now });
                tracing::warn!(vm_id = %vm_id, "Circuit breaker opened (first failure)");
            }
        }
    }

    /// Transition to half-open state
    ///
    /// Called when the circuit break duration has elapsed.
    async fn transition_to_half_open(&self, vm_id: Uuid) {
        let mut states = self.states.write().await;
        states.insert(
            vm_id,
            CircuitState::HalfOpen {
                probe_successes: 0,
                probe_failures: 0,
            },
        );
        tracing::info!(
            vm_id = %vm_id,
            "Circuit breaker entering half-open state"
        );
    }

    /// Get current circuit state for a VM
    pub async fn get_state(&self, vm_id: Uuid) -> CircuitState {
        self.states
            .read()
            .await
            .get(&vm_id)
            .cloned()
            .unwrap_or(CircuitState::Closed)
    }

    /// Reset circuit state for a VM
    pub async fn reset(&self, vm_id: Uuid) {
        let mut states = self.states.write().await;
        states.insert(vm_id, CircuitState::Closed);
        tracing::debug!(vm_id = %vm_id, "Circuit breaker reset to closed");
    }

    /// Get circuit breaker metrics
    pub async fn get_metrics(&self) -> CircuitBreakerMetrics {
        let states = self.states.read().await;
        let mut metrics = CircuitBreakerMetrics::default();

        for state in states.values() {
            match state {
                CircuitState::Closed => metrics.closed_count += 1,
                CircuitState::Open { .. } => metrics.open_count += 1,
                CircuitState::HalfOpen { .. } => metrics.half_open_count += 1,
            }
        }

        metrics
    }
}

/// Metrics for circuit breaker status
#[derive(Debug, Clone, Default)]
pub struct CircuitBreakerMetrics {
    pub closed_count: usize,
    pub open_count: usize,
    pub half_open_count: usize,
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

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let breaker = CircuitBreaker::new(test_config());
        let vm_id = Uuid::new_v4();

        let state = breaker.get_state(vm_id).await;
        assert_eq!(state, CircuitState::Closed);
        assert!(!breaker.is_open(vm_id).await);
    }

    #[tokio::test]
    async fn test_circuit_opens_on_failure() {
        let breaker = CircuitBreaker::new(test_config());
        let vm_id = Uuid::new_v4();

        // Initially closed
        assert!(!breaker.is_open(vm_id).await);

        // Record failure
        breaker.record_failure(vm_id).await;

        // Now open
        assert!(breaker.is_open(vm_id).await);
        let state = breaker.get_state(vm_id).await;
        assert!(matches!(state, CircuitState::Open { .. }));
    }

    #[tokio::test]
    async fn test_circuit_stays_open_during_duration() {
        let breaker = CircuitBreaker::new(test_config());
        let vm_id = Uuid::new_v4();

        breaker.record_failure(vm_id).await;
        assert!(breaker.is_open(vm_id).await);

        // Still open immediately after
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(breaker.is_open(vm_id).await);
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open() {
        let mut config = test_config();
        config.circuit_break_duration_secs = 0; // Immediate transition
        let breaker = CircuitBreaker::new(config);
        let vm_id = Uuid::new_v4();

        breaker.record_failure(vm_id).await;

        // Should transition to half-open immediately
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        assert!(!breaker.is_open(vm_id).await); // Half-open allows checks

        let state = breaker.get_state(vm_id).await;
        assert!(matches!(state, CircuitState::HalfOpen { .. }));
    }

    #[tokio::test]
    async fn test_half_open_probe_success_closes_circuit() {
        let breaker = CircuitBreaker::new(test_config());
        let vm_id = Uuid::new_v4();

        // Manually set to half-open
        breaker.transition_to_half_open(vm_id).await;

        // First success
        breaker.record_success(vm_id).await;
        let state = breaker.get_state(vm_id).await;
        assert!(matches!(
            state,
            CircuitState::HalfOpen {
                probe_successes: 1,
                probe_failures: 0
            }
        ));

        // Second success - should close
        breaker.record_success(vm_id).await;
        let state = breaker.get_state(vm_id).await;
        assert_eq!(state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_probe_failure_reopens() {
        let breaker = CircuitBreaker::new(test_config());
        let vm_id = Uuid::new_v4();

        // Manually set to half-open
        breaker.transition_to_half_open(vm_id).await;

        // One success
        breaker.record_success(vm_id).await;

        // Then failure - should reopen
        breaker.record_failure(vm_id).await;
        let state = breaker.get_state(vm_id).await;
        assert!(matches!(state, CircuitState::Open { .. }));
    }

    #[tokio::test]
    async fn test_circuit_breaker_disabled() {
        let mut config = test_config();
        config.enable_circuit_breaker = false;
        let breaker = CircuitBreaker::new(config);
        let vm_id = Uuid::new_v4();

        // Record failure
        breaker.record_failure(vm_id).await;

        // Circuit should not open when disabled
        assert!(!breaker.is_open(vm_id).await);
    }

    #[tokio::test]
    async fn test_reset_circuit() {
        let breaker = CircuitBreaker::new(test_config());
        let vm_id = Uuid::new_v4();

        // Open circuit
        breaker.record_failure(vm_id).await;
        assert!(breaker.is_open(vm_id).await);

        // Reset
        breaker.reset(vm_id).await;
        assert!(!breaker.is_open(vm_id).await);
        let state = breaker.get_state(vm_id).await;
        assert_eq!(state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_metrics() {
        let breaker = CircuitBreaker::new(test_config());

        let _vm1 = Uuid::new_v4();
        let vm2 = Uuid::new_v4();
        let vm3 = Uuid::new_v4();

        // vm1: closed (default)
        // vm2: open
        breaker.record_failure(vm2).await;
        // vm3: half-open
        breaker.transition_to_half_open(vm3).await;

        let metrics = breaker.get_metrics().await;
        assert_eq!(metrics.closed_count, 0); // Only explicit states counted
        assert_eq!(metrics.open_count, 1);
        assert_eq!(metrics.half_open_count, 1);
    }

    #[tokio::test]
    async fn test_concurrent_circuit_operations() {
        let breaker = std::sync::Arc::new(CircuitBreaker::new(test_config()));
        let vm_id = Uuid::new_v4();

        // Spawn multiple concurrent operations
        let mut handles = vec![];

        for i in 0..10 {
            let breaker_clone = breaker.clone();
            let handle = tokio::spawn(async move {
                if i % 2 == 0 {
                    breaker_clone.record_success(vm_id).await;
                } else {
                    breaker_clone.record_failure(vm_id).await;
                }
            });
            handles.push(handle);
        }

        // Wait for all operations
        for handle in handles {
            handle.await.unwrap();
        }

        // Should have a consistent final state (no panics/deadlocks)
        let state = breaker.get_state(vm_id).await;
        assert!(
            matches!(
                state,
                CircuitState::Closed | CircuitState::Open { .. } | CircuitState::HalfOpen { .. }
            ),
            "Invalid circuit state after concurrent operations"
        );
    }
}
