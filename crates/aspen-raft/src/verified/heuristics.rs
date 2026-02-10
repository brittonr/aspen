//! Pure heuristic functions for Raft operations.
//!
//! This module contains pure functions for TTL calculation, clock drift detection,
//! supervisor logic, connection pooling, and node failure detection.
//!
//! All functions are deterministic and side-effect free, making them ideal for:
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Fuzzing for edge case discovery
//!
//! # Tiger Style
//!
//! - All calculations bounded by explicit limits from constants.rs
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes

use std::time::Duration;

use crate::clock_drift_detection::DriftSeverity;
use crate::connection_pool::ConnectionHealth;
use crate::node_failure_detection::ConnectionStatus;
use crate::node_failure_detection::FailureType;

// ============================================================================
// TTL Calculation Pure Functions
// ============================================================================

/// Calculate absolute expiration timestamp from relative TTL.
///
/// Converts a TTL in seconds to an absolute expiration time in milliseconds
/// since Unix epoch. Uses saturating arithmetic to prevent overflow.
///
/// # Arguments
///
/// * `now_ms` - Current timestamp in milliseconds since Unix epoch
/// * `ttl_seconds` - Time-to-live in seconds
///
/// # Returns
///
/// Absolute expiration timestamp in milliseconds. On overflow, returns `u64::MAX`.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::calculate_expires_at_ms;
///
/// let now_ms = 1704067200000; // Jan 1, 2024 00:00:00 UTC
/// let ttl_seconds = 3600;     // 1 hour
/// let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
/// assert_eq!(expires_at, now_ms + 3600 * 1000);
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic to prevent overflow
/// - Returns deterministic result for any valid input
/// - No panics for any input combination
#[inline]
pub fn calculate_expires_at_ms(now_ms: u64, ttl_seconds: u32) -> u64 {
    let ttl_ms = (ttl_seconds as u64).saturating_mul(1000);
    now_ms.saturating_add(ttl_ms)
}

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
/// use aspen_raft::pure::calculate_ntp_clock_offset;
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
/// use aspen_raft::pure::compute_ewma;
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
// Supervisor Pure Functions
// ============================================================================

/// Calculate backoff duration for restart attempts.
///
/// Uses a lookup table approach with capping at the maximum index.
///
/// # Arguments
///
/// * `restart_count` - Number of restarts attempted
/// * `backoff_durations` - Array of backoff durations indexed by attempt count
///
/// # Returns
///
/// The backoff duration for this restart attempt, capped at the last array element.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use aspen_raft::pure::calculate_backoff_duration;
///
/// let durations = [Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(10)];
/// assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(1));
/// assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(5));
/// assert_eq!(calculate_backoff_duration(2, &durations), Duration::from_secs(10));
/// assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(10)); // capped
/// ```
#[inline]
pub fn calculate_backoff_duration(restart_count: usize, backoff_durations: &[Duration]) -> Duration {
    let idx = restart_count.min(backoff_durations.len().saturating_sub(1));
    backoff_durations[idx]
}

/// Determine if a restart should be allowed based on recent restart count.
///
/// # Arguments
///
/// * `recent_restarts` - Number of restarts within the sliding window
/// * `max_restarts` - Maximum allowed restarts before giving up
///
/// # Returns
///
/// `true` if restart is allowed, `false` if circuit breaker should trip
#[inline]
pub fn should_allow_restart(recent_restarts: usize, max_restarts: u32) -> bool {
    recent_restarts < max_restarts as usize
}

// ============================================================================
// Connection Pool Pure Functions
// ============================================================================

/// Transition connection health state based on stream operation result.
///
/// Implements a state machine:
/// - Healthy + success → Healthy
/// - Healthy + failure → Degraded(1)
/// - Degraded(n) + success → Healthy
/// - Degraded(n) + failure → Degraded(n+1) or Failed if n >= max_retries
/// - Failed + any → Failed (terminal)
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
/// use aspen_raft::pure::calculate_connection_retry_backoff;
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
    // TTL Calculation Tests
    // ========================================================================

    #[test]
    fn test_ttl_calculation_basic() {
        let now_ms = 1704067200000; // Jan 1, 2024 00:00:00 UTC
        let ttl_seconds = 3600; // 1 hour
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, now_ms + 3600 * 1000);
    }

    #[test]
    fn test_ttl_calculation_zero_ttl() {
        let now_ms = 1704067200000;
        let ttl_seconds = 0;
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, now_ms); // Expires immediately
    }

    #[test]
    fn test_ttl_calculation_one_second() {
        let now_ms = 1000;
        let ttl_seconds = 1;
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, 2000);
    }

    #[test]
    fn test_ttl_calculation_large_ttl() {
        let now_ms = 0;
        let ttl_seconds = u32::MAX; // ~136 years
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        // u32::MAX = 4294967295, * 1000 = 4294967295000
        assert_eq!(expires_at, 4294967295000);
    }

    #[test]
    fn test_ttl_calculation_overflow_protection() {
        // Near max now_ms with large TTL should saturate, not overflow
        let now_ms = u64::MAX - 1000;
        let ttl_seconds = u32::MAX;
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, u64::MAX); // Saturates to max
    }

    #[test]
    fn test_ttl_calculation_max_now_ms() {
        let now_ms = u64::MAX;
        let ttl_seconds = 1;
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(expires_at, u64::MAX); // Saturates, doesn't wrap
    }

    #[test]
    fn test_ttl_calculation_realistic_30_days() {
        let now_ms = 1704067200000; // Jan 1, 2024
        let ttl_seconds = 30 * 24 * 60 * 60; // 30 days
        let expires_at = calculate_expires_at_ms(now_ms, ttl_seconds);
        let expected = now_ms + (30_u64 * 24 * 60 * 60 * 1000);
        assert_eq!(expires_at, expected);
    }

    #[test]
    fn test_ttl_calculation_deterministic() {
        // Same inputs should always produce same output
        let now_ms = 1704067200000;
        let ttl_seconds = 3600;
        let result1 = calculate_expires_at_ms(now_ms, ttl_seconds);
        let result2 = calculate_expires_at_ms(now_ms, ttl_seconds);
        assert_eq!(result1, result2);
    }

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
    fn test_ntp_offset_server_ahead() {
        // Server clock 1000ms ahead, symmetric 50ms network delay
        // Client sends at 1000, server receives at 2050, server sends at 2100, client receives at 1150
        let (offset, rtt) = calculate_ntp_clock_offset(1000, 2050, 2100, 1150);
        // offset = ((2050-1000) + (2100-1150)) / 2 = (1050 + 950) / 2 = 1000ms
        assert_eq!(offset, 1000);
        // rtt = (1150-1000) - (2100-2050) = 150 - 50 = 100ms
        assert_eq!(rtt, 100);
    }

    #[test]
    fn test_ntp_offset_high_rtt() {
        // High RTT scenario (500ms round trip)
        let (offset, rtt) = calculate_ntp_clock_offset(0, 250, 300, 500);
        // offset = ((250-0) + (300-500)) / 2 = (250 + (-200)) / 2 = 25ms
        assert_eq!(offset, 25);
        // rtt = (500-0) - (300-250) = 500 - 50 = 450ms
        assert_eq!(rtt, 450);
    }

    #[test]
    fn test_ntp_offset_asymmetric_delay() {
        // Asymmetric delay: fast outbound (50ms), slow return (150ms)
        // No clock drift
        let (offset, rtt) = calculate_ntp_clock_offset(1000, 1050, 1100, 1250);
        // offset = ((1050-1000) + (1100-1250)) / 2 = (50 + (-150)) / 2 = -50ms
        // This shows asymmetric delays introduce measurement error
        assert_eq!(offset, -50);
        // rtt = (1250-1000) - (1100-1050) = 250 - 50 = 200ms
        assert_eq!(rtt, 200);
    }

    #[test]
    fn test_ntp_offset_large_timestamps() {
        // Large timestamp values (near year 2100)
        let base = 4_000_000_000_000_u64; // ~2096 in ms since epoch
        let (offset, rtt) = calculate_ntp_clock_offset(base, base + 100, base + 150, base + 200);
        assert_eq!(offset, 25);
        assert_eq!(rtt, 150);
    }

    #[test]
    fn test_ntp_offset_zero_server_processing_time() {
        // Server processes instantly (server_send == server_recv)
        let (offset, rtt) = calculate_ntp_clock_offset(1000, 1100, 1100, 1200);
        // offset = ((1100-1000) + (1100-1200)) / 2 = (100 + (-100)) / 2 = 0
        assert_eq!(offset, 0);
        // rtt = (1200-1000) - (1100-1100) = 200 - 0 = 200ms
        assert_eq!(rtt, 200);
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
    fn test_classify_drift_boundary_at_warning() {
        // Exactly at warning threshold
        assert_eq!(classify_drift_severity(100.0, 100, 500), DriftSeverity::Warning);
        assert_eq!(classify_drift_severity(-100.0, 100, 500), DriftSeverity::Warning);
    }

    #[test]
    fn test_classify_drift_boundary_below_warning() {
        // Just below warning threshold
        assert_eq!(classify_drift_severity(99.0, 100, 500), DriftSeverity::Normal);
        assert_eq!(classify_drift_severity(-99.0, 100, 500), DriftSeverity::Normal);
    }

    #[test]
    fn test_classify_drift_boundary_at_alert() {
        // Exactly at alert threshold
        assert_eq!(classify_drift_severity(500.0, 100, 500), DriftSeverity::Alert);
        assert_eq!(classify_drift_severity(-500.0, 100, 500), DriftSeverity::Alert);
    }

    #[test]
    fn test_classify_drift_boundary_below_alert() {
        // Just below alert threshold
        assert_eq!(classify_drift_severity(499.0, 100, 500), DriftSeverity::Warning);
        assert_eq!(classify_drift_severity(-499.0, 100, 500), DriftSeverity::Warning);
    }

    #[test]
    fn test_classify_drift_zero() {
        assert_eq!(classify_drift_severity(0.0, 100, 500), DriftSeverity::Normal);
    }

    #[test]
    fn test_classify_drift_large_values() {
        // Very large drift values
        assert_eq!(classify_drift_severity(1_000_000.0, 100, 500), DriftSeverity::Alert);
        assert_eq!(classify_drift_severity(-1_000_000.0, 100, 500), DriftSeverity::Alert);
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

    #[test]
    fn test_ewma_no_weight() {
        let result = compute_ewma(100.0, 50.0, 0.0);
        assert!((result - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_half_weight() {
        // Alpha = 0.5 should average the values
        let result = compute_ewma(100.0, 0.0, 0.5);
        assert!((result - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_convergence() {
        // Simulate convergence over multiple iterations
        let mut avg = 0.0;
        let alpha = 0.1;
        let target = 100.0;

        for _ in 0..50 {
            avg = compute_ewma(target, avg, alpha);
        }
        // After many iterations, should be close to target
        assert!((avg - target).abs() < 1.0);
    }

    #[test]
    fn test_ewma_negative_values() {
        let result = compute_ewma(-50.0, 50.0, 0.5);
        assert!((result - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_same_values() {
        // When new and old are same, result should be same regardless of alpha
        let result = compute_ewma(42.0, 42.0, 0.7);
        assert!((result - 42.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_small_alpha() {
        // Very small alpha should barely change
        let result = compute_ewma(100.0, 50.0, 0.01);
        // 0.01 * 100 + 0.99 * 50 = 1 + 49.5 = 50.5
        assert!((result - 50.5).abs() < 0.001);
    }

    // ========================================================================
    // Supervisor Tests
    // ========================================================================

    #[test]
    fn test_backoff_first_attempt() {
        let durations = [Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(10)];
        assert_eq!(calculate_backoff_duration(0, &durations), Duration::from_secs(1));
    }

    #[test]
    fn test_backoff_progression() {
        let durations = [Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(10)];
        assert_eq!(calculate_backoff_duration(1, &durations), Duration::from_secs(5));
        assert_eq!(calculate_backoff_duration(2, &durations), Duration::from_secs(10));
    }

    #[test]
    fn test_backoff_capped() {
        let durations = [Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(10)];
        assert_eq!(calculate_backoff_duration(100, &durations), Duration::from_secs(10));
        assert_eq!(calculate_backoff_duration(usize::MAX, &durations), Duration::from_secs(10));
    }

    #[test]
    fn test_should_restart_within_limit() {
        assert!(should_allow_restart(0, 3));
        assert!(should_allow_restart(1, 3));
        assert!(should_allow_restart(2, 3));
    }

    #[test]
    fn test_should_restart_at_limit() {
        assert!(!should_allow_restart(3, 3));
        assert!(!should_allow_restart(4, 3));
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
    fn test_health_degraded_failure_not_max() {
        assert_eq!(
            transition_connection_health(
                ConnectionHealth::Degraded {
                    consecutive_failures: 1
                },
                false,
                3
            ),
            ConnectionHealth::Degraded {
                consecutive_failures: 2
            }
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

    #[test]
    fn test_retry_backoff_saturating() {
        // Should not overflow
        let result = calculate_connection_retry_backoff(63, 100);
        assert!(result >= Duration::from_millis(100));
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

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    // ========================================================================
    // TTL Calculation Property Tests
    // ========================================================================

    #[test]
    fn prop_ttl_calculation_never_underflows() {
        check!().with_type::<(u64, u32)>().for_each(|(now_ms, ttl_seconds)| {
            let result = calculate_expires_at_ms(*now_ms, *ttl_seconds);
            // Result should always be >= now_ms (no underflow)
            assert!(result >= *now_ms, "Expires at {} should be >= now_ms {}", result, now_ms);
        });
    }

    #[test]
    fn prop_ttl_calculation_monotonic_in_ttl() {
        check!().with_type::<(u64, u32, u32)>().for_each(|(now_ms, ttl1, ttl2)| {
            let result1 = calculate_expires_at_ms(*now_ms, *ttl1);
            let result2 = calculate_expires_at_ms(*now_ms, *ttl2);
            // Larger TTL should result in larger or equal expiration
            if *ttl1 <= *ttl2 {
                assert!(result1 <= result2, "TTL {} should give result <= TTL {}", ttl1, ttl2);
            }
        });
    }

    #[test]
    fn prop_ttl_calculation_no_panic() {
        // Just verify no panics for any input combination
        check!().with_type::<(u64, u32)>().for_each(|(now_ms, ttl_seconds)| {
            let _ = calculate_expires_at_ms(*now_ms, *ttl_seconds);
        });
    }

    // ========================================================================
    // Clock Drift Property Tests
    // ========================================================================

    #[test]
    fn prop_ntp_offset_no_panic() {
        // Verify no panics for any input combination
        check!().with_type::<(u64, u64, u64, u64)>().for_each(|(t1, t2, t3, t4)| {
            let _ = calculate_ntp_clock_offset(*t1, *t2, *t3, *t4);
        });
    }

    #[test]
    fn prop_classify_drift_severity_ordering() {
        // Higher offsets should never result in lower severity
        check!()
            .with_type::<(f64, u64, u64)>()
            .filter(|(_, warning, alert)| *warning > 0 && *alert > *warning)
            .for_each(|(offset, warning, alert)| {
                if !offset.is_finite() {
                    return;
                }
                let severity = classify_drift_severity(*offset, *warning, *alert);
                let double_offset = offset * 2.0;
                if double_offset.is_finite() {
                    let higher_severity = classify_drift_severity(double_offset, *warning, *alert);
                    // Higher offset should not result in lower severity
                    match severity {
                        DriftSeverity::Alert => {} // Can stay Alert
                        DriftSeverity::Warning => {
                            assert!(
                                matches!(higher_severity, DriftSeverity::Warning | DriftSeverity::Alert),
                                "Double offset should not decrease severity"
                            );
                        }
                        DriftSeverity::Normal => {} // Anything is valid for higher
                    }
                }
            });
    }

    #[test]
    fn prop_classify_drift_abs_symmetric() {
        // Positive and negative should give same result
        check!().with_type::<(f64, u64, u64)>().filter(|(_, w, a)| *w > 0 && *a > *w).for_each(
            |(offset, warning, alert)| {
                if !offset.is_finite() || *offset == 0.0 {
                    return;
                }
                let pos_severity = classify_drift_severity(offset.abs(), *warning, *alert);
                let neg_severity = classify_drift_severity(-offset.abs(), *warning, *alert);
                assert_eq!(pos_severity, neg_severity, "Positive and negative offset should give same severity");
            },
        );
    }

    // ========================================================================
    // EWMA Property Tests
    // ========================================================================

    #[test]
    fn prop_ewma_bounded() {
        check!().with_type::<(f64, f64, f64)>().for_each(|(new_value, old_avg, alpha)| {
            // Clamp alpha to valid range
            let alpha = alpha.clamp(0.0, 1.0);

            // Skip NaN/Inf inputs
            if !new_value.is_finite() || !old_avg.is_finite() {
                return;
            }

            let result = compute_ewma(*new_value, *old_avg, alpha);

            // EWMA should be bounded between inputs when alpha is in [0, 1]
            if alpha >= 0.0 && alpha <= 1.0 {
                let min_val = new_value.min(*old_avg);
                let max_val = new_value.max(*old_avg);
                assert!(
                    result >= min_val && result <= max_val,
                    "EWMA {} not between {} and {}",
                    result,
                    min_val,
                    max_val
                );
            }
        });
    }

    #[test]
    fn prop_backoff_never_exceeds_max() {
        let durations = [Duration::from_secs(1), Duration::from_secs(5), Duration::from_secs(10)];
        let max_duration = Duration::from_secs(10);

        check!().with_type::<usize>().for_each(|restart_count| {
            let result = calculate_backoff_duration(*restart_count, &durations);
            assert!(result <= max_duration);
        });
    }

    #[test]
    fn prop_health_failed_is_terminal() {
        check!().with_type::<(bool, u32)>().for_each(|(succeeded, max_retries)| {
            let result = transition_connection_health(ConnectionHealth::Failed, *succeeded, *max_retries);
            assert_eq!(result, ConnectionHealth::Failed);
        });
    }

    #[test]
    fn prop_retry_backoff_monotonic() {
        check!().with_type::<(u32, u64)>().filter(|(_, base)| *base > 0 && *base < 10000).for_each(
            |(attempt, base_ms)| {
                if *attempt > 0 && *attempt < 30 {
                    let current = calculate_connection_retry_backoff(*attempt, *base_ms);
                    let next = calculate_connection_retry_backoff(attempt.saturating_add(1), *base_ms);
                    assert!(next >= current, "Backoff should be monotonically increasing");
                }
            },
        );
    }
}
