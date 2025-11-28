// Pure state machine for health status transitions
//
// This module contains ONLY pure functions - no I/O, no side effects, no async.
// All state transitions are deterministic and easily testable.
//
// State transition diagram:
// ```
// Unknown → Healthy (first successful check)
// Unknown → Degraded (first failed check)
// Healthy → Healthy (continued success)
// Healthy → Degraded (first failure)
// Degraded → Healthy (recovery after <= recovery_threshold failures)
// Degraded → Degraded (gradual recovery or additional failures)
// Degraded → Unhealthy (failures >= failure_threshold)
// Unhealthy → Degraded (first successful check after unhealthy)
// Unhealthy → Unhealthy (continued failure)
// ```

use super::config::HealthCheckConfig;
use super::types::{HealthStatus, SideEffect, StateTransition};

/// Pure state machine for health status transitions
///
/// This struct contains ONLY pure functions with no I/O or side effects.
/// All methods are deterministic and can be tested without async/await.
pub struct HealthStateMachine {
    config: HealthCheckConfig,
}

impl HealthStateMachine {
    /// Create a new state machine with the given configuration
    pub fn new(config: HealthCheckConfig) -> Self {
        Self { config }
    }

    /// Handle a successful health check - compute state transition
    ///
    /// This method is pure logic with no side effects, making it testable.
    /// Returns a StateTransition containing the new state and optional side effect.
    pub fn handle_successful_check(
        &self,
        current: &HealthStatus,
        response_time_ms: u64,
    ) -> StateTransition {
        let now = chrono::Utc::now().timestamp();

        match current {
            HealthStatus::Unknown => StateTransition::new(
                HealthStatus::Healthy {
                    last_check: now,
                    response_time_ms,
                },
                None,
            ),
            HealthStatus::Healthy { .. } => StateTransition::new(
                HealthStatus::Healthy {
                    last_check: now,
                    response_time_ms,
                },
                None,
            ),
            HealthStatus::Degraded { failures, .. } => {
                if *failures <= self.config.recovery_threshold {
                    // Full recovery - back to healthy
                    StateTransition::new(
                        HealthStatus::Healthy {
                            last_check: now,
                            response_time_ms,
                        },
                        Some(SideEffect::LogRecovery {
                            vm_id: uuid::Uuid::nil(), // Filled by caller
                            previous_failures: *failures,
                        }),
                    )
                } else {
                    // Partial recovery - still degraded but fewer failures
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
                    failures: self.config.recovery_threshold.saturating_sub(1),
                    last_error: String::new(),
                },
                Some(SideEffect::LogRecoveryFromUnhealthy {
                    vm_id: uuid::Uuid::nil(), // Filled by caller
                }),
            ),
        }
    }

    /// Handle a failed health check - compute state transition
    ///
    /// This method is pure logic with no side effects.
    /// Returns a StateTransition containing the new state and optional side effect.
    pub fn handle_failed_check(
        &self,
        current: &HealthStatus,
        error: &str,
    ) -> StateTransition {
        let now = chrono::Utc::now().timestamp();

        match current {
            HealthStatus::Unknown => StateTransition::new(
                HealthStatus::Degraded {
                    last_check: now,
                    failures: 1,
                    last_error: error.to_string(),
                },
                None,
            ),
            HealthStatus::Healthy { .. } => StateTransition::new(
                HealthStatus::Degraded {
                    last_check: now,
                    failures: 1,
                    last_error: error.to_string(),
                },
                Some(SideEffect::LogDegradation {
                    vm_id: uuid::Uuid::nil(), // Filled by caller
                    error: error.to_string(),
                }),
            ),
            HealthStatus::Degraded { failures, .. } => {
                let new_failures = *failures + 1;

                if new_failures >= self.config.failure_threshold {
                    // Crossed threshold - now unhealthy
                    StateTransition::new(
                        HealthStatus::Unhealthy {
                            since: now,
                            failures: new_failures,
                            last_error: error.to_string(),
                        },
                        Some(SideEffect::MarkUnhealthy {
                            vm_id: uuid::Uuid::nil(), // Filled by caller
                            failures: new_failures,
                            error: error.to_string(),
                        }),
                    )
                } else {
                    // Still degraded, increment failure count
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
        }
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

    #[test]
    fn test_state_transition_unknown_to_healthy() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Unknown;

        let transition = machine.handle_successful_check(&current, 100);

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
    fn test_state_transition_unknown_to_degraded() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Unknown;

        let transition = machine.handle_failed_check(&current, "timeout");

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 1);
            }
            _ => panic!("Expected Degraded state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_healthy_to_healthy() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 50,
        };

        let transition = machine.handle_successful_check(&current, 100);

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
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 100,
        };

        let transition = machine.handle_failed_check(&current, "timeout");

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
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 2,
            last_error: "previous error".to_string(),
        };

        let transition = machine.handle_failed_check(&current, "new error");

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
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 1,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_failed_check(&current, "another error");

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 2);
            }
            _ => panic!("Expected Degraded state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_degraded_to_healthy() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 2,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_successful_check(&current, 100);

        match transition.new_state {
            HealthStatus::Healthy { .. } => {}
            _ => panic!("Expected Healthy state"),
        }

        matches!(transition.side_effect, Some(SideEffect::LogRecovery { .. }));
    }

    #[test]
    fn test_state_transition_degraded_gradual_recovery() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: 5,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_successful_check(&current, 100);

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, 4); // Should decrement
            }
            _ => panic!("Expected Degraded state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_state_transition_unhealthy_to_degraded() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Unhealthy {
            since: 0,
            failures: 5,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_successful_check(&current, 100);

        match transition.new_state {
            HealthStatus::Degraded { failures, .. } => {
                assert_eq!(failures, test_config().recovery_threshold - 1);
            }
            _ => panic!("Expected Degraded state"),
        }

        matches!(
            transition.side_effect,
            Some(SideEffect::LogRecoveryFromUnhealthy { .. })
        );
    }

    #[test]
    fn test_state_transition_unhealthy_continued_failure() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Unhealthy {
            since: 100,
            failures: 5,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_failed_check(&current, "still failing");

        match transition.new_state {
            HealthStatus::Unhealthy {
                since, failures, ..
            } => {
                assert_eq!(since, 100); // 'since' should not change
                assert_eq!(failures, 6); // failures should increment
            }
            _ => panic!("Expected Unhealthy state"),
        }
        assert!(transition.side_effect.is_none());
    }

    #[test]
    fn test_recovery_threshold_exact_match() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: test_config().recovery_threshold,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_successful_check(&current, 100);

        match transition.new_state {
            HealthStatus::Healthy { .. } => {}
            _ => panic!("Expected Healthy state when failures == recovery_threshold"),
        }
    }

    #[test]
    fn test_failure_threshold_exact_match() {
        let machine = HealthStateMachine::new(test_config());
        let current = HealthStatus::Degraded {
            last_check: 0,
            failures: test_config().failure_threshold - 1,
            last_error: "error".to_string(),
        };

        let transition = machine.handle_failed_check(&current, "final error");

        match transition.new_state {
            HealthStatus::Unhealthy { .. } => {}
            _ => panic!("Expected Unhealthy state when failures == failure_threshold"),
        }
    }

    #[test]
    fn test_multiple_transitions() {
        let machine = HealthStateMachine::new(test_config());
        let mut current = HealthStatus::Unknown;

        // Unknown -> Healthy
        let transition = machine.handle_successful_check(&current, 100);
        current = transition.new_state;
        assert!(matches!(current, HealthStatus::Healthy { .. }));

        // Healthy -> Degraded (1 failure)
        let transition = machine.handle_failed_check(&current, "error 1");
        current = transition.new_state;
        assert!(matches!(
            current,
            HealthStatus::Degraded { failures: 1, .. }
        ));

        // Degraded (1) -> Degraded (2)
        let transition = machine.handle_failed_check(&current, "error 2");
        current = transition.new_state;
        assert!(matches!(
            current,
            HealthStatus::Degraded { failures: 2, .. }
        ));

        // Degraded (2) -> Unhealthy (3)
        let transition = machine.handle_failed_check(&current, "error 3");
        current = transition.new_state;
        assert!(matches!(current, HealthStatus::Unhealthy { .. }));

        // Unhealthy -> Degraded (recovery starts)
        let transition = machine.handle_successful_check(&current, 100);
        current = transition.new_state;
        assert!(matches!(current, HealthStatus::Degraded { .. }));

        // Degraded -> Healthy (full recovery)
        let transition = machine.handle_successful_check(&current, 100);
        current = transition.new_state;
        assert!(matches!(current, HealthStatus::Healthy { .. }));
    }
}
