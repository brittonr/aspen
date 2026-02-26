//! Network-related types for Raft communication.
//!
//! These types are shared across network-layer crates (aspen-raft-network, aspen-raft)
//! without requiring heavy dependencies like Iroh or openraft.

use serde::Deserialize;
use serde::Serialize;

/// Connection status for Raft heartbeat or Iroh transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connection is active and healthy.
    Connected,
    /// Connection is broken or unreachable.
    Disconnected,
}

/// Classification of node failures based on connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureType {
    /// Node is healthy and responsive.
    Healthy,
    /// Raft actor crashed but node is reachable (Iroh connected).
    ///
    /// Indicates local process problem, suitable for auto-restart.
    ActorCrash,
    /// Both Raft and Iroh connections failed.
    ///
    /// Indicates node-level failure, requires operator intervention.
    NodeCrash,
}

/// Severity level of detected clock drift.
///
/// Indicates how far the estimated clock offset is from ideal (0ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftSeverity {
    /// Drift within acceptable range (< warning threshold).
    Normal,
    /// Drift exceeds warning threshold but below alert.
    /// Indicates NTP may need attention.
    Warning,
    /// Drift exceeds alert threshold.
    /// Indicates significant NTP misconfiguration.
    Alert,
}

/// Health status of a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionHealth {
    /// Connection is healthy and operational.
    Healthy,
    /// Connection has experienced failures but may recover.
    Degraded {
        /// Number of consecutive failures observed.
        consecutive_failures: u32,
    },
    /// Connection has failed and should be replaced.
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ConnectionStatus Tests
    // =========================================================================

    #[test]
    fn test_connection_status_equality() {
        assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
        assert_eq!(ConnectionStatus::Disconnected, ConnectionStatus::Disconnected);
        assert_ne!(ConnectionStatus::Connected, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_connection_status_copy() {
        let status = ConnectionStatus::Connected;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_connection_status_debug() {
        let status = ConnectionStatus::Connected;
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("Connected"));
    }

    // =========================================================================
    // FailureType Tests
    // =========================================================================

    #[test]
    fn test_failure_type_equality() {
        assert_eq!(FailureType::Healthy, FailureType::Healthy);
        assert_eq!(FailureType::ActorCrash, FailureType::ActorCrash);
        assert_eq!(FailureType::NodeCrash, FailureType::NodeCrash);
        assert_ne!(FailureType::Healthy, FailureType::ActorCrash);
        assert_ne!(FailureType::ActorCrash, FailureType::NodeCrash);
    }

    #[test]
    fn test_failure_type_copy() {
        let failure = FailureType::ActorCrash;
        let copied = failure;
        assert_eq!(failure, copied);
    }

    #[test]
    fn test_failure_type_debug() {
        let failure = FailureType::NodeCrash;
        let debug_str = format!("{:?}", failure);
        assert!(debug_str.contains("NodeCrash"));
    }

    // =========================================================================
    // DriftSeverity Tests
    // =========================================================================

    #[test]
    fn test_drift_severity_equality() {
        assert_eq!(DriftSeverity::Normal, DriftSeverity::Normal);
        assert_eq!(DriftSeverity::Warning, DriftSeverity::Warning);
        assert_eq!(DriftSeverity::Alert, DriftSeverity::Alert);
        assert_ne!(DriftSeverity::Normal, DriftSeverity::Warning);
        assert_ne!(DriftSeverity::Warning, DriftSeverity::Alert);
    }

    #[test]
    fn test_drift_severity_copy() {
        let severity = DriftSeverity::Warning;
        let copied = severity;
        assert_eq!(severity, copied);
    }

    #[test]
    fn test_drift_severity_debug() {
        let severity = DriftSeverity::Alert;
        let debug_str = format!("{:?}", severity);
        assert!(debug_str.contains("Alert"));
    }

    // =========================================================================
    // ConnectionHealth Tests
    // =========================================================================

    #[test]
    fn test_connection_health_healthy() {
        let health = ConnectionHealth::Healthy;
        assert_eq!(health, ConnectionHealth::Healthy);
    }

    #[test]
    fn test_connection_health_degraded() {
        let health = ConnectionHealth::Degraded {
            consecutive_failures: 2,
        };
        assert!(matches!(health, ConnectionHealth::Degraded {
            consecutive_failures: 2
        }));
    }

    #[test]
    fn test_connection_health_failed() {
        let health = ConnectionHealth::Failed;
        assert_eq!(health, ConnectionHealth::Failed);
    }

    #[test]
    fn test_connection_health_copy() {
        let health = ConnectionHealth::Degraded {
            consecutive_failures: 3,
        };
        let copied = health;
        assert_eq!(health, copied);
    }

    #[test]
    fn test_connection_health_eq_different_variants() {
        assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Failed);
        assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Degraded {
            consecutive_failures: 1
        });
        assert_ne!(ConnectionHealth::Failed, ConnectionHealth::Degraded {
            consecutive_failures: 1
        });
    }

    #[test]
    fn test_connection_health_eq_same_degraded_different_counts() {
        let health1 = ConnectionHealth::Degraded {
            consecutive_failures: 1,
        };
        let health2 = ConnectionHealth::Degraded {
            consecutive_failures: 2,
        };
        assert_ne!(health1, health2);
    }

    #[test]
    fn test_connection_health_debug() {
        let health = ConnectionHealth::Degraded {
            consecutive_failures: 5,
        };
        let debug_str = format!("{:?}", health);
        assert!(debug_str.contains("Degraded"));
        assert!(debug_str.contains("5"));
    }

    // =========================================================================
    // Serde Roundtrip Tests
    // =========================================================================

    #[test]
    fn test_connection_status_serde_roundtrip() {
        for status in [ConnectionStatus::Connected, ConnectionStatus::Disconnected] {
            let json = serde_json::to_string(&status).expect("serialize");
            let deserialized: ConnectionStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_failure_type_serde_roundtrip() {
        for failure in [FailureType::Healthy, FailureType::ActorCrash, FailureType::NodeCrash] {
            let json = serde_json::to_string(&failure).expect("serialize");
            let deserialized: FailureType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(failure, deserialized);
        }
    }

    #[test]
    fn test_drift_severity_serde_roundtrip() {
        for severity in [DriftSeverity::Normal, DriftSeverity::Warning, DriftSeverity::Alert] {
            let json = serde_json::to_string(&severity).expect("serialize");
            let deserialized: DriftSeverity = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(severity, deserialized);
        }
    }

    #[test]
    fn test_connection_health_serde_roundtrip() {
        let values = [
            ConnectionHealth::Healthy,
            ConnectionHealth::Degraded {
                consecutive_failures: 0,
            },
            ConnectionHealth::Degraded {
                consecutive_failures: 5,
            },
            ConnectionHealth::Failed,
        ];
        for health in values {
            let json = serde_json::to_string(&health).expect("serialize");
            let deserialized: ConnectionHealth = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(health, deserialized);
        }
    }
}
