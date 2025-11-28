// Shared type definitions for health checking system
//
// This module contains pure data types with no I/O or side effects.
// All types are serializable for persistence and messaging.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Health status of a VM
///
/// Represents the current health state with relevant metadata.
/// State transitions are managed by HealthStateMachine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// VM health unknown (not yet checked)
    Unknown,

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
}

impl HealthStatus {
    /// Check if the VM is in a running state (healthy or degraded)
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            HealthStatus::Healthy { .. } | HealthStatus::Degraded { .. }
        )
    }

    /// Check if the VM is specifically healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy { .. })
    }

    /// Check if the VM is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self, HealthStatus::Degraded { .. })
    }

    /// Check if the VM is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, HealthStatus::Unhealthy { .. })
    }
}

/// Result of a state transition
///
/// Contains the new state and an optional side effect to execute.
/// This separation allows state machine to be pure (no I/O).
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub new_state: HealthStatus,
    pub side_effect: Option<SideEffect>,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(new_state: HealthStatus, side_effect: Option<SideEffect>) -> Self {
        Self {
            new_state,
            side_effect,
        }
    }
}

/// Side effects from state transitions
///
/// These represent actions that should be taken as a result of state changes.
/// They are pure data and executed by SideEffectExecutor.
#[derive(Debug, Clone)]
pub enum SideEffect {
    /// Log a health status change
    LogHealthChange {
        vm_id: Uuid,
        old_state: HealthStatus,
        new_state: HealthStatus,
    },

    /// Log recovery from degraded state
    LogRecovery {
        vm_id: Uuid,
        previous_failures: u32,
    },

    /// Log recovery from unhealthy state
    LogRecoveryFromUnhealthy {
        vm_id: Uuid,
    },

    /// Log degradation to degraded state (original format for backward compatibility)
    LogDegradation {
        vm_id: Uuid,
        error: String,
    },

    /// Mark VM as unhealthy and potentially terminate
    MarkUnhealthy {
        vm_id: Uuid,
        failures: u32,
        error: String,
    },
}

/// Result of a health check attempt
#[derive(Debug, Clone)]
pub enum CheckResult {
    /// Health check succeeded
    Success {
        response_time_ms: u64,
    },

    /// Health check failed
    Failure {
        error: String,
        timestamp: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_is_running() {
        assert!(!HealthStatus::Unknown.is_running());
        assert!(HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 10
        }
        .is_running());
        assert!(HealthStatus::Degraded {
            last_check: 0,
            failures: 1,
            last_error: "".to_string()
        }
        .is_running());
        assert!(!HealthStatus::Unhealthy {
            since: 0,
            failures: 3,
            last_error: "".to_string()
        }
        .is_running());
    }

    #[test]
    fn test_health_status_specific_checks() {
        let healthy = HealthStatus::Healthy {
            last_check: 0,
            response_time_ms: 10,
        };
        assert!(healthy.is_healthy());
        assert!(!healthy.is_degraded());
        assert!(!healthy.is_unhealthy());

        let degraded = HealthStatus::Degraded {
            last_check: 0,
            failures: 1,
            last_error: "test".to_string(),
        };
        assert!(!degraded.is_healthy());
        assert!(degraded.is_degraded());
        assert!(!degraded.is_unhealthy());

        let unhealthy = HealthStatus::Unhealthy {
            since: 0,
            failures: 3,
            last_error: "test".to_string(),
        };
        assert!(!unhealthy.is_healthy());
        assert!(!unhealthy.is_degraded());
        assert!(unhealthy.is_unhealthy());
    }
}
