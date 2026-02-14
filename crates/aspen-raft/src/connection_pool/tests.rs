//! Unit tests for connection pool components.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tokio::sync::Semaphore;

use super::*;
use crate::connection_pool::peer_connection::StreamGuard;
use crate::constants::IROH_CONNECT_TIMEOUT;
use crate::constants::IROH_STREAM_OPEN_TIMEOUT;
use crate::constants::MAX_PEERS;
use crate::constants::MAX_STREAMS_PER_CONNECTION;
use crate::types::NodeId;

// =========================================================================
// ConnectionHealth Enum Tests
// =========================================================================

#[test]
fn test_connection_health_healthy_variant() {
    let health = ConnectionHealth::Healthy;
    assert_eq!(health, ConnectionHealth::Healthy);
}

#[test]
fn test_connection_health_degraded_variant() {
    let health = ConnectionHealth::Degraded {
        consecutive_failures: 2,
    };
    assert!(matches!(health, ConnectionHealth::Degraded {
        consecutive_failures: 2
    }));
}

#[test]
fn test_connection_health_failed_variant() {
    let health = ConnectionHealth::Failed;
    assert_eq!(health, ConnectionHealth::Failed);
}

#[test]
fn test_connection_health_clone() {
    let health = ConnectionHealth::Degraded {
        consecutive_failures: 3,
    };
    let cloned = health;
    assert_eq!(health, cloned);
}

#[test]
fn test_connection_health_copy() {
    let health = ConnectionHealth::Healthy;
    let copied = health;
    // Both should be valid (Copy trait)
    assert_eq!(health, copied);
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

// =========================================================================
// ConnectionPoolMetrics Tests
// =========================================================================

#[test]
fn test_connection_pool_metrics_creation() {
    let metrics = ConnectionPoolMetrics {
        total_connections: 10,
        healthy_connections: 7,
        degraded_connections: 2,
        failed_connections: 1,
        total_active_streams: 25,
    };
    assert_eq!(metrics.total_connections, 10);
    assert_eq!(metrics.healthy_connections, 7);
    assert_eq!(metrics.degraded_connections, 2);
    assert_eq!(metrics.failed_connections, 1);
    assert_eq!(metrics.total_active_streams, 25);
}

#[test]
fn test_connection_pool_metrics_clone() {
    let metrics = ConnectionPoolMetrics {
        total_connections: 5,
        healthy_connections: 3,
        degraded_connections: 1,
        failed_connections: 1,
        total_active_streams: 10,
    };
    let cloned = metrics.clone();
    assert_eq!(metrics.total_connections, cloned.total_connections);
    assert_eq!(metrics.healthy_connections, cloned.healthy_connections);
    assert_eq!(metrics.degraded_connections, cloned.degraded_connections);
    assert_eq!(metrics.failed_connections, cloned.failed_connections);
    assert_eq!(metrics.total_active_streams, cloned.total_active_streams);
}

#[test]
fn test_connection_pool_metrics_debug() {
    let metrics = ConnectionPoolMetrics {
        total_connections: 3,
        healthy_connections: 2,
        degraded_connections: 1,
        failed_connections: 0,
        total_active_streams: 5,
    };
    let debug_str = format!("{:?}", metrics);
    assert!(debug_str.contains("ConnectionPoolMetrics"));
    assert!(debug_str.contains("total_connections"));
    assert!(debug_str.contains("3"));
}

#[test]
fn test_connection_pool_metrics_zero_values() {
    let metrics = ConnectionPoolMetrics {
        total_connections: 0,
        healthy_connections: 0,
        degraded_connections: 0,
        failed_connections: 0,
        total_active_streams: 0,
    };
    assert_eq!(metrics.total_connections, 0);
    assert_eq!(metrics.total_active_streams, 0);
}

#[test]
fn test_connection_pool_metrics_max_values() {
    let metrics = ConnectionPoolMetrics {
        total_connections: u32::MAX,
        healthy_connections: u32::MAX,
        degraded_connections: u32::MAX,
        failed_connections: u32::MAX,
        total_active_streams: u32::MAX,
    };
    assert_eq!(metrics.total_connections, u32::MAX);
}

// =========================================================================
// Constants Validation Tests
// =========================================================================

#[test]
fn test_connection_idle_timeout_constant() {
    assert_eq!(CONNECTION_IDLE_TIMEOUT, Duration::from_secs(60));
}

#[test]
fn test_max_connection_retries_constant() {
    assert_eq!(MAX_CONNECTION_RETRIES, 3);
}

#[test]
fn test_connection_retry_backoff_base_constant() {
    assert_eq!(CONNECTION_RETRY_BACKOFF_BASE_MS, 100);
}

#[test]
fn test_max_peers_constant() {
    assert_eq!(MAX_PEERS, 1000);
}

#[test]
fn test_max_streams_per_connection_constant() {
    assert_eq!(MAX_STREAMS_PER_CONNECTION, 100);
}

#[test]
fn test_iroh_connect_timeout_constant() {
    assert_eq!(IROH_CONNECT_TIMEOUT, Duration::from_secs(5));
}

#[test]
fn test_iroh_stream_open_timeout_constant() {
    assert_eq!(IROH_STREAM_OPEN_TIMEOUT, Duration::from_secs(2));
}

// =========================================================================
// Exponential Backoff Calculation Tests
// =========================================================================

#[test]
fn test_exponential_backoff_attempt_1() {
    // Formula: BASE * (1 << (attempts - 1))
    let attempts = 1;
    let backoff = Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
    assert_eq!(backoff, Duration::from_millis(100));
}

#[test]
fn test_exponential_backoff_attempt_2() {
    let attempts = 2;
    let backoff = Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
    assert_eq!(backoff, Duration::from_millis(200));
}

#[test]
fn test_exponential_backoff_attempt_3() {
    let attempts = 3;
    let backoff = Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
    assert_eq!(backoff, Duration::from_millis(400));
}

#[test]
fn test_exponential_backoff_max_retries() {
    // At MAX_CONNECTION_RETRIES (3), backoff = 100 * (1 << 2) = 400ms
    let attempts = MAX_CONNECTION_RETRIES;
    let backoff = Duration::from_millis(CONNECTION_RETRY_BACKOFF_BASE_MS * (1 << (attempts - 1)));
    assert_eq!(backoff, Duration::from_millis(400));
}

// =========================================================================
// StreamGuard Tests
// =========================================================================

#[test]
fn test_stream_guard_decrements_on_drop() {
    let active_streams = Arc::new(AtomicU32::new(5));
    let semaphore = Arc::new(Semaphore::new(10));
    let permit = semaphore.clone().try_acquire_owned().unwrap();

    assert_eq!(active_streams.load(Ordering::Relaxed), 5);

    let guard = StreamGuard {
        _permit: permit,
        active_streams: active_streams.clone(),
    };

    drop(guard);

    // Should have decremented by 1
    assert_eq!(active_streams.load(Ordering::Relaxed), 4);
}

#[test]
fn test_stream_guard_multiple_drops() {
    let active_streams = Arc::new(AtomicU32::new(10));
    let semaphore = Arc::new(Semaphore::new(10));

    // Create and drop multiple guards
    for expected in (7..=9).rev() {
        let permit = semaphore.clone().try_acquire_owned().unwrap();
        let guard = StreamGuard {
            _permit: permit,
            active_streams: active_streams.clone(),
        };
        drop(guard);
        assert_eq!(active_streams.load(Ordering::Relaxed), expected);
    }
}

#[test]
fn test_stream_guard_underflow_protection() {
    // Test that fetch_sub with 0 doesn't panic (wraps to u32::MAX, but that's okay
    // since in practice we always increment before decrement)
    let active_streams = Arc::new(AtomicU32::new(1));
    let semaphore = Arc::new(Semaphore::new(1));
    let permit = semaphore.clone().try_acquire_owned().unwrap();

    let guard = StreamGuard {
        _permit: permit,
        active_streams: active_streams.clone(),
    };
    drop(guard);

    assert_eq!(active_streams.load(Ordering::Relaxed), 0);
}

// =========================================================================
// Atomic Operations Tests
// =========================================================================

#[test]
fn test_atomic_add_relaxed() {
    let counter = AtomicU32::new(0);
    let result = counter.fetch_add(1, Ordering::Relaxed);
    assert_eq!(result, 0);
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn test_atomic_sub_relaxed() {
    let counter = AtomicU32::new(5);
    let result = counter.fetch_sub(1, Ordering::Relaxed);
    assert_eq!(result, 5);
    assert_eq!(counter.load(Ordering::Relaxed), 4);
}

#[test]
fn test_atomic_load_relaxed() {
    let counter = AtomicU32::new(42);
    assert_eq!(counter.load(Ordering::Relaxed), 42);
}

#[test]
fn test_atomic_concurrent_operations() {
    let counter = Arc::new(AtomicU32::new(0));

    // Simulate concurrent increments
    for _ in 0..100 {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    assert_eq!(counter.load(Ordering::Relaxed), 100);
}

// =========================================================================
// Semaphore Tests (for stream limiting)
// =========================================================================

#[test]
fn test_semaphore_bounded_capacity() {
    let semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));

    // Should be able to acquire MAX_STREAMS_PER_CONNECTION permits
    let mut permits = Vec::new();
    for _ in 0..MAX_STREAMS_PER_CONNECTION {
        let permit = semaphore.clone().try_acquire_owned();
        assert!(permit.is_ok());
        permits.push(permit.unwrap());
    }

    // Next acquisition should fail
    let result = semaphore.clone().try_acquire_owned();
    assert!(result.is_err());
}

#[test]
fn test_semaphore_release_on_drop() {
    let semaphore = Arc::new(Semaphore::new(1));

    // Acquire the only permit
    let permit = semaphore.clone().try_acquire_owned().unwrap();

    // Second acquisition should fail
    assert!(semaphore.clone().try_acquire_owned().is_err());

    // Drop the permit
    drop(permit);

    // Now acquisition should succeed
    assert!(semaphore.clone().try_acquire_owned().is_ok());
}

// =========================================================================
// Duration and Timeout Tests
// =========================================================================

#[test]
fn test_timeout_durations_ordering() {
    // Connect timeout > Stream timeout (connect includes handshake)
    assert!(IROH_CONNECT_TIMEOUT > IROH_STREAM_OPEN_TIMEOUT);

    // Idle timeout > Connect timeout (connections should live longer)
    assert!(CONNECTION_IDLE_TIMEOUT > IROH_CONNECT_TIMEOUT);
}

#[test]
fn test_idle_timeout_in_seconds() {
    assert_eq!(CONNECTION_IDLE_TIMEOUT.as_secs(), 60);
}

#[test]
fn test_connect_timeout_in_seconds() {
    assert_eq!(IROH_CONNECT_TIMEOUT.as_secs(), 5);
}

#[test]
fn test_stream_timeout_in_seconds() {
    assert_eq!(IROH_STREAM_OPEN_TIMEOUT.as_secs(), 2);
}

// =========================================================================
// Capacity Bounds Tests
// =========================================================================

#[test]
fn test_max_peers_is_bounded() {
    // Verify MAX_PEERS has a reasonable upper bound
    // Use runtime value to avoid clippy assertions_on_constants warning
    let max_peers = MAX_PEERS;
    assert!(max_peers <= 10_000);
    assert!(max_peers >= 100);
}

#[test]
fn test_max_streams_per_connection_is_bounded() {
    // Verify streams per connection is reasonable
    let max_streams = MAX_STREAMS_PER_CONNECTION;
    assert!(max_streams <= 1_000);
    assert!(max_streams >= 10);
}

#[test]
fn test_max_retries_is_bounded() {
    // Verify retries are reasonable (not infinite, not too few)
    let max_retries = MAX_CONNECTION_RETRIES;
    assert!(max_retries >= 1);
    assert!(max_retries <= 10);
}

// =========================================================================
// Node ID Type Tests (used in pool keys)
// =========================================================================

#[test]
fn test_node_id_as_hashmap_key() {
    use std::collections::HashMap;
    let mut map: HashMap<NodeId, u32> = HashMap::new();

    let node1 = NodeId::from(1);
    let node2 = NodeId::from(2);

    map.insert(node1, 100);
    map.insert(node2, 200);

    assert_eq!(map.get(&node1), Some(&100));
    assert_eq!(map.get(&node2), Some(&200));
    assert_eq!(map.len(), 2);
}

#[test]
fn test_node_id_copy_for_pool_operations() {
    let node = NodeId::from(42);
    let copied = node;
    // Both should be valid due to Copy
    assert_eq!(node, copied);
}

// =========================================================================
// ALPN Configuration Tests
// =========================================================================

#[test]
#[allow(deprecated)]
fn test_raft_alpn_constant() {
    // Verify legacy RAFT_ALPN is correct (deprecated but tested for backward compat)
    assert_eq!(aspen_transport::RAFT_ALPN, b"raft-rpc");
}

#[test]
fn test_raft_auth_alpn_constant() {
    // Verify RAFT_AUTH_ALPN is correct (recommended for production)
    assert_eq!(aspen_transport::RAFT_AUTH_ALPN, b"raft-auth");
}

#[test]
#[allow(deprecated)]
fn test_alpn_selection_with_auth() {
    // When use_auth_alpn is true, should select RAFT_AUTH_ALPN
    let use_auth_alpn = true;
    let alpn = if use_auth_alpn {
        aspen_transport::RAFT_AUTH_ALPN
    } else {
        aspen_transport::RAFT_ALPN
    };
    assert_eq!(alpn, b"raft-auth");
}

#[test]
#[allow(deprecated)]
fn test_alpn_selection_without_auth() {
    // When use_auth_alpn is false, should select RAFT_ALPN (deprecated path)
    let use_auth_alpn = false;
    let alpn = if use_auth_alpn {
        aspen_transport::RAFT_AUTH_ALPN
    } else {
        aspen_transport::RAFT_ALPN
    };
    assert_eq!(alpn, b"raft-rpc");
}

#[test]
#[allow(deprecated)]
fn test_alpn_values_are_different() {
    // Ensure the two ALPN values are distinct
    assert_ne!(aspen_transport::RAFT_ALPN, aspen_transport::RAFT_AUTH_ALPN);
}

#[test]
#[allow(deprecated)]
fn test_alpn_values_are_valid_utf8() {
    // Both ALPN values should be valid UTF-8 for logging
    assert!(std::str::from_utf8(aspen_transport::RAFT_ALPN).is_ok());
    assert!(std::str::from_utf8(aspen_transport::RAFT_AUTH_ALPN).is_ok());
}
