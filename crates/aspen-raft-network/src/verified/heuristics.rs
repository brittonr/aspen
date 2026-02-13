//! Pure heuristic functions for network operations.
//!
//! This module contains pure functions for clock drift detection,
//! connection pooling, and node failure detection.
//!
//! All functions are deterministic and side-effect free, making them ideal for:
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Fuzzing for edge case discovery
//!
//! # Tiger Style
//!
//! - All calculations bounded by explicit limits
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes

use std::time::Duration;

use aspen_raft_types::network::ConnectionHealth;
use aspen_raft_types::network::ConnectionStatus;
use aspen_raft_types::network::DriftSeverity;
use aspen_raft_types::network::FailureType;

// ============================================================================
// Clock Drift Detection Pure Functions
// ============================================================================

/// Calculate NTP-style clock offset between two nodes.
///
/// Uses the 4-timestamp NTP algorithm to estimate clock offset while
/// accounting for network latency. The formula is:
///
/// ```text
/// offset = ((t2 - t1) + (t3 - t4)) / 2
/// ```
///
/// Where:
/// - t1: Client sends request
/// - t2: Server receives request
/// - t3: Server sends response
/// - t4: Client receives response
///
/// # Returns
///
/// A tuple of (offset_ms, rtt_ms) where:
/// - offset_ms: Estimated clock offset in milliseconds (positive = peer ahead)
/// - rtt_ms: Round-trip time in milliseconds
///
/// # Example
///
/// ```
/// use aspen_raft_network::verified::calculate_ntp_clock_offset;
///
/// let (offset, rtt) = calculate_ntp_clock_offset(1000, 1100, 1150, 1200);
/// // offset = ((1100-1000) + (1150-1200)) / 2 = (100 + (-50)) / 2 = 25ms
/// assert_eq!(offset, 25);
/// ```
#[inline]
pub fn calculate_ntp_clock_offset(
    client_send_ms: u64,
    server_recv_ms: u64,
    server_send_ms: u64,
    client_recv_ms: u64,
) -> (i64, i64) {
    // Calculate clock offset using NTP formula
    let t2_minus_t1 = server_recv_ms as i64 - client_send_ms as i64;
    let t3_minus_t4 = server_send_ms as i64 - client_recv_ms as i64;
    let offset_ms = (t2_minus_t1 + t3_minus_t4) / 2;

    // Calculate RTT: (t4 - t1) - (t3 - t2)
    let rtt_ms = (client_recv_ms as i64 - client_send_ms as i64) - (server_send_ms as i64 - server_recv_ms as i64);

    (offset_ms, rtt_ms)
}

/// Classify drift severity based on absolute offset magnitude.
///
/// # Arguments
///
/// * `ewma_offset_ms` - Smoothed offset in milliseconds (can be negative)
/// * `warning_threshold_ms` - Threshold for Warning severity
/// * `alert_threshold_ms` - Threshold for Alert severity
///
/// # Returns
///
/// `DriftSeverity::Normal`, `DriftSeverity::Warning`, or `DriftSeverity::Alert`
#[inline]
pub fn classify_drift_severity(
    ewma_offset_ms: f64,
    warning_threshold_ms: u64,
    alert_threshold_ms: u64,
) -> DriftSeverity {
    let abs_offset = ewma_offset_ms.abs() as u64;
    if abs_offset >= alert_threshold_ms {
        DriftSeverity::Alert
    } else if abs_offset >= warning_threshold_ms {
        DriftSeverity::Warning
    } else {
        DriftSeverity::Normal
    }
}

/// Update exponentially weighted moving average (EWMA).
///
/// Formula: `new_avg = alpha * new_value + (1 - alpha) * old_avg`
///
/// # Arguments
///
/// * `new_value` - The new measurement
/// * `old_avg` - The previous EWMA value
/// * `alpha` - Smoothing factor (0.0 to 1.0, higher = more weight on new values)
///
/// # Example
///
/// ```
/// use aspen_raft_network::verified::compute_ewma;
///
/// let old_avg = 0.0;
/// let new_value = 100.0;
/// let alpha = 0.1;
/// let result = compute_ewma(new_value, old_avg, alpha);
/// // result = 0.1 * 100 + 0.9 * 0 = 10.0
/// assert!((result - 10.0).abs() < 0.001);
/// ```
#[inline]
pub fn compute_ewma(new_value: f64, old_avg: f64, alpha: f64) -> f64 {
    alpha * new_value + (1.0 - alpha) * old_avg
}

// ============================================================================
// Connection Pool Pure Functions
// ============================================================================

/// Transition connection health state based on stream operation result.
///
/// Implements a state machine:
/// - Healthy + success -> Healthy
/// - Healthy + failure -> Degraded(1)
/// - Degraded(n) + success -> Healthy
/// - Degraded(n) + failure -> Degraded(n+1) or Failed if n >= max_retries
/// - Failed + any -> Failed (terminal)
///
/// # Arguments
///
/// * `current` - Current health state
/// * `operation_succeeded` - Whether the stream operation succeeded
/// * `max_retries` - Maximum consecutive failures before marking as Failed
///
/// # Returns
///
/// The new health state after the transition.
#[inline]
pub fn transition_connection_health(
    current: ConnectionHealth,
    operation_succeeded: bool,
    max_retries: u32,
) -> ConnectionHealth {
    if operation_succeeded {
        // Success always resets to Healthy (except from Failed)
        match current {
            ConnectionHealth::Failed => ConnectionHealth::Failed,
            _ => ConnectionHealth::Healthy,
        }
    } else {
        // Failure transitions through state machine
        match current {
            ConnectionHealth::Healthy => ConnectionHealth::Degraded {
                consecutive_failures: 1,
            },
            ConnectionHealth::Degraded { consecutive_failures } => {
                if consecutive_failures >= max_retries {
                    ConnectionHealth::Failed
                } else {
                    ConnectionHealth::Degraded {
                        consecutive_failures: consecutive_failures + 1,
                    }
                }
            }
            ConnectionHealth::Failed => ConnectionHealth::Failed,
        }
    }
}

/// Calculate exponential backoff for connection retry.
///
/// Formula: `base_ms * 2^(attempt - 1)`
///
/// # Arguments
///
/// * `attempt` - Retry attempt number (1-based)
/// * `base_ms` - Base backoff in milliseconds
///
/// # Returns
///
/// Backoff duration. For attempt=1 returns base_ms, doubles each subsequent attempt.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use aspen_raft_network::verified::calculate_connection_retry_backoff;
///
/// assert_eq!(calculate_connection_retry_backoff(1, 100), Duration::from_millis(100));
/// assert_eq!(calculate_connection_retry_backoff(2, 100), Duration::from_millis(200));
/// assert_eq!(calculate_connection_retry_backoff(3, 100), Duration::from_millis(400));
/// ```
#[inline]
pub fn calculate_connection_retry_backoff(attempt: u32, base_ms: u64) -> Duration {
    Duration::from_millis(base_ms.saturating_mul(1u64 << (attempt.saturating_sub(1))))
}

// ============================================================================
// Node Failure Detection Pure Functions
// ============================================================================

/// Classify node failure type based on Raft and Iroh connection statuses.
///
/// Truth table:
/// | Raft        | Iroh         | Result     |
/// |-------------|--------------|------------|
/// | Connected   | *            | Healthy    |
/// | Disconnected| Connected    | ActorCrash |
/// | Disconnected| Disconnected | NodeCrash  |
///
/// # Arguments
///
/// * `raft_heartbeat` - Raft heartbeat connection status
/// * `iroh_connection` - Iroh P2P transport status
///
/// # Returns
///
/// The classified failure type.
#[inline]
pub fn classify_node_failure(raft_heartbeat: ConnectionStatus, iroh_connection: ConnectionStatus) -> FailureType {
    match (raft_heartbeat, iroh_connection) {
        (ConnectionStatus::Connected, _) => FailureType::Healthy,
        (ConnectionStatus::Disconnected, ConnectionStatus::Connected) => FailureType::ActorCrash,
        (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected) => FailureType::NodeCrash,
    }
}

/// Determine if the oldest unreachable node should be evicted.
///
/// Eviction is needed when:
/// 1. The map is at capacity (len >= max_nodes)
/// 2. The failing node is NOT already in the map
///
/// # Arguments
///
/// * `current_count` - Current number of tracked unreachable nodes
/// * `max_nodes` - Maximum allowed unreachable nodes
/// * `new_node_already_tracked` - Whether the new failing node is already being tracked
///
/// # Returns
///
/// `true` if the oldest entry should be evicted to make room
#[inline]
pub fn should_evict_oldest_unreachable(current_count: usize, max_nodes: usize, new_node_already_tracked: bool) -> bool {
    !new_node_already_tracked && current_count >= max_nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Clock Drift Tests
    // ========================================================================

    #[test]
    fn test_ntp_offset_symmetric() {
        // Symmetric network delay, server 50ms ahead
        let (offset, rtt) = calculate_ntp_clock_offset(1000, 1100, 1150, 1200);
        // offset = ((100) + (-50)) / 2 = 25ms
        assert_eq!(offset, 25);
        // rtt = (200) - (50) = 150ms
        assert_eq!(rtt, 150);
    }

    #[test]
    fn test_ntp_offset_no_delay() {
        // Zero network delay, no clock drift
        let (offset, rtt) = calculate_ntp_clock_offset(1000, 1000, 1000, 1000);
        assert_eq!(offset, 0);
        assert_eq!(rtt, 0);
    }

    #[test]
    fn test_ntp_offset_negative() {
        // Server clock behind client
        let (offset, _rtt) = calculate_ntp_clock_offset(1000, 900, 950, 1100);
        // offset = ((-100) + (-150)) / 2 = -125ms
        assert_eq!(offset, -125);
    }

    #[test]
    fn test_classify_drift_normal() {
        assert_eq!(classify_drift_severity(50.0, 100, 500), DriftSeverity::Normal);
        assert_eq!(classify_drift_severity(-50.0, 100, 500), DriftSeverity::Normal);
    }

    #[test]
    fn test_classify_drift_warning() {
        assert_eq!(classify_drift_severity(150.0, 100, 500), DriftSeverity::Warning);
        assert_eq!(classify_drift_severity(-150.0, 100, 500), DriftSeverity::Warning);
    }

    #[test]
    fn test_classify_drift_alert() {
        assert_eq!(classify_drift_severity(600.0, 100, 500), DriftSeverity::Alert);
        assert_eq!(classify_drift_severity(-600.0, 100, 500), DriftSeverity::Alert);
    }

    #[test]
    fn test_ewma_first_value() {
        let result = compute_ewma(100.0, 0.0, 0.1);
        assert!((result - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_full_weight() {
        let result = compute_ewma(100.0, 50.0, 1.0);
        assert!((result - 100.0).abs() < 0.001);
    }

    // ========================================================================
    // Connection Pool Tests
    // ========================================================================

    #[test]
    fn test_health_healthy_success() {
        assert_eq!(transition_connection_health(ConnectionHealth::Healthy, true, 3), ConnectionHealth::Healthy);
    }

    #[test]
    fn test_health_healthy_failure() {
        assert_eq!(transition_connection_health(ConnectionHealth::Healthy, false, 3), ConnectionHealth::Degraded {
            consecutive_failures: 1
        });
    }

    #[test]
    fn test_health_degraded_success() {
        assert_eq!(
            transition_connection_health(
                ConnectionHealth::Degraded {
                    consecutive_failures: 2
                },
                true,
                3
            ),
            ConnectionHealth::Healthy
        );
    }

    #[test]
    fn test_health_degraded_failure_at_max() {
        assert_eq!(
            transition_connection_health(
                ConnectionHealth::Degraded {
                    consecutive_failures: 3
                },
                false,
                3
            ),
            ConnectionHealth::Failed
        );
    }

    #[test]
    fn test_health_failed_is_terminal() {
        assert_eq!(transition_connection_health(ConnectionHealth::Failed, true, 3), ConnectionHealth::Failed);
        assert_eq!(transition_connection_health(ConnectionHealth::Failed, false, 3), ConnectionHealth::Failed);
    }

    #[test]
    fn test_retry_backoff_progression() {
        assert_eq!(calculate_connection_retry_backoff(1, 100), Duration::from_millis(100));
        assert_eq!(calculate_connection_retry_backoff(2, 100), Duration::from_millis(200));
        assert_eq!(calculate_connection_retry_backoff(3, 100), Duration::from_millis(400));
    }

    // ========================================================================
    // Node Failure Detection Tests
    // ========================================================================

    #[test]
    fn test_classify_healthy() {
        assert_eq!(
            classify_node_failure(ConnectionStatus::Connected, ConnectionStatus::Connected),
            FailureType::Healthy
        );
        assert_eq!(
            classify_node_failure(ConnectionStatus::Connected, ConnectionStatus::Disconnected),
            FailureType::Healthy
        );
    }

    #[test]
    fn test_classify_actor_crash() {
        assert_eq!(
            classify_node_failure(ConnectionStatus::Disconnected, ConnectionStatus::Connected),
            FailureType::ActorCrash
        );
    }

    #[test]
    fn test_classify_node_crash() {
        assert_eq!(
            classify_node_failure(ConnectionStatus::Disconnected, ConnectionStatus::Disconnected),
            FailureType::NodeCrash
        );
    }

    #[test]
    fn test_should_evict_when_full_and_new() {
        assert!(should_evict_oldest_unreachable(1000, 1000, false));
    }

    #[test]
    fn test_should_not_evict_when_not_full() {
        assert!(!should_evict_oldest_unreachable(999, 1000, false));
    }

    #[test]
    fn test_should_not_evict_when_already_tracked() {
        assert!(!should_evict_oldest_unreachable(1000, 1000, true));
    }
}
