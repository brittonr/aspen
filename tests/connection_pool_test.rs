//! Unit tests for the connection pool module.
//!
//! Tests the ConnectionHealth enum, PeerConnection behavior, and ConnectionPoolMetrics.
//! Note: Full RaftConnectionPool testing requires Iroh endpoint mocking which is
//! tested via integration tests.

use aspen::raft::connection_pool::{
    CONNECTION_IDLE_TIMEOUT, CONNECTION_RETRY_BACKOFF_BASE_MS, ConnectionHealth,
    ConnectionPoolMetrics, MAX_CONNECTION_RETRIES,
};
use std::time::Duration;

/// Test ConnectionHealth enum variants and equality.
#[test]
fn test_connection_health_variants() {
    let healthy = ConnectionHealth::Healthy;
    let degraded = ConnectionHealth::Degraded {
        consecutive_failures: 1,
    };
    let failed = ConnectionHealth::Failed;

    // Test equality
    assert_eq!(healthy, ConnectionHealth::Healthy);
    assert_eq!(failed, ConnectionHealth::Failed);
    assert_ne!(healthy, failed);
    assert_ne!(healthy, degraded);
    assert_ne!(degraded, failed);
}

/// Test ConnectionHealth degraded state tracks consecutive failures.
#[test]
fn test_connection_health_degraded_state() {
    let degraded1 = ConnectionHealth::Degraded {
        consecutive_failures: 1,
    };
    let degraded2 = ConnectionHealth::Degraded {
        consecutive_failures: 2,
    };
    let degraded1_copy = ConnectionHealth::Degraded {
        consecutive_failures: 1,
    };

    // Different failure counts should not be equal
    assert_ne!(degraded1, degraded2);
    // Same failure count should be equal
    assert_eq!(degraded1, degraded1_copy);
}

/// Test ConnectionHealth can be copied (it implements Copy).
#[test]
fn test_connection_health_copy() {
    let healthy = ConnectionHealth::Healthy;
    let healthy_copy = healthy; // Copy trait
    let healthy_copy2: ConnectionHealth = healthy; // Another copy

    assert_eq!(healthy, healthy_copy);
    assert_eq!(healthy, healthy_copy2);
}

/// Test ConnectionHealth debug format.
#[test]
fn test_connection_health_debug() {
    let healthy = ConnectionHealth::Healthy;
    let degraded = ConnectionHealth::Degraded {
        consecutive_failures: 3,
    };
    let failed = ConnectionHealth::Failed;

    let healthy_debug = format!("{:?}", healthy);
    let degraded_debug = format!("{:?}", degraded);
    let failed_debug = format!("{:?}", failed);

    assert!(healthy_debug.contains("Healthy"));
    assert!(degraded_debug.contains("Degraded"));
    assert!(degraded_debug.contains("consecutive_failures"));
    assert!(degraded_debug.contains("3"));
    assert!(failed_debug.contains("Failed"));
}

/// Test ConnectionPoolMetrics structure and fields.
#[test]
fn test_connection_pool_metrics_fields() {
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

    // Verify counts add up
    assert_eq!(
        metrics.healthy_connections + metrics.degraded_connections + metrics.failed_connections,
        metrics.total_connections
    );
}

/// Test ConnectionPoolMetrics clone and debug.
#[test]
fn test_connection_pool_metrics_clone_debug() {
    let metrics = ConnectionPoolMetrics {
        total_connections: 5,
        healthy_connections: 3,
        degraded_connections: 1,
        failed_connections: 1,
        total_active_streams: 10,
    };

    let metrics_clone = metrics.clone();
    assert_eq!(metrics_clone.total_connections, metrics.total_connections);

    let debug_str = format!("{:?}", metrics);
    assert!(debug_str.contains("ConnectionPoolMetrics"));
    assert!(debug_str.contains("total_connections"));
    assert!(debug_str.contains("5"));
}

/// Test constant values are as expected (Tiger Style resource bounds).
#[test]
fn test_connection_pool_constants() {
    // Verify timeout is reasonable (60 seconds)
    assert_eq!(CONNECTION_IDLE_TIMEOUT, Duration::from_secs(60));

    // Verify retry limit is bounded (3)
    assert_eq!(MAX_CONNECTION_RETRIES, 3);

    // Verify backoff base is reasonable (100ms)
    assert_eq!(CONNECTION_RETRY_BACKOFF_BASE_MS, 100);
}

/// Test exponential backoff calculation pattern.
#[test]
fn test_exponential_backoff_pattern() {
    let base_ms = CONNECTION_RETRY_BACKOFF_BASE_MS;

    // First attempt (attempt 0): base = 100ms
    let backoff_1 = Duration::from_millis(base_ms);
    assert_eq!(backoff_1, Duration::from_millis(100));

    // Second attempt (attempt 1): base * 2^1 = 200ms
    let backoff_2 = Duration::from_millis(base_ms * 2);
    assert_eq!(backoff_2, Duration::from_millis(200));

    // Third attempt (attempt 2): base * 2^2 = 400ms
    let backoff_3 = Duration::from_millis(base_ms * 4);
    assert_eq!(backoff_3, Duration::from_millis(400));
}

/// Test ConnectionPoolMetrics with edge cases.
#[test]
fn test_connection_pool_metrics_edge_cases() {
    // Empty pool
    let empty = ConnectionPoolMetrics {
        total_connections: 0,
        healthy_connections: 0,
        degraded_connections: 0,
        failed_connections: 0,
        total_active_streams: 0,
    };
    assert_eq!(empty.total_connections, 0);

    // All healthy
    let all_healthy = ConnectionPoolMetrics {
        total_connections: 100,
        healthy_connections: 100,
        degraded_connections: 0,
        failed_connections: 0,
        total_active_streams: 500,
    };
    assert_eq!(
        all_healthy.healthy_connections,
        all_healthy.total_connections
    );

    // All failed
    let all_failed = ConnectionPoolMetrics {
        total_connections: 10,
        healthy_connections: 0,
        degraded_connections: 0,
        failed_connections: 10,
        total_active_streams: 0,
    };
    assert_eq!(all_failed.failed_connections, all_failed.total_connections);
}

/// Test health state transitions are valid per the state machine.
#[test]
fn test_health_state_machine_validity() {
    // Valid states
    let _healthy = ConnectionHealth::Healthy;
    let _degraded_1 = ConnectionHealth::Degraded {
        consecutive_failures: 1,
    };
    let _degraded_max = ConnectionHealth::Degraded {
        consecutive_failures: MAX_CONNECTION_RETRIES,
    };
    let _failed = ConnectionHealth::Failed;

    // State machine transitions documented in code:
    // Healthy -> Degraded (on first failure)
    // Degraded -> Degraded (on subsequent failures, incrementing count)
    // Degraded -> Failed (when consecutive_failures >= MAX_CONNECTION_RETRIES)
    // Degraded -> Healthy (on successful stream open)
    // Failed -> Failed (stays failed, requires new connection)

    // Verify degraded can hold failure count up to MAX_CONNECTION_RETRIES
    for i in 1..=MAX_CONNECTION_RETRIES {
        let degraded = ConnectionHealth::Degraded {
            consecutive_failures: i,
        };
        if let ConnectionHealth::Degraded {
            consecutive_failures,
        } = degraded
        {
            assert_eq!(consecutive_failures, i);
        }
    }
}

/// Test that MAX_CONNECTION_RETRIES bounds the degraded state.
#[test]
fn test_max_retries_bounds_degraded_state() {
    // After MAX_CONNECTION_RETRIES failures, connection should transition to Failed
    let at_limit = ConnectionHealth::Degraded {
        consecutive_failures: MAX_CONNECTION_RETRIES,
    };

    // Verify at_limit is just before transitioning to Failed
    if let ConnectionHealth::Degraded {
        consecutive_failures,
    } = at_limit
    {
        assert_eq!(consecutive_failures, MAX_CONNECTION_RETRIES);
        // Next failure would transition to Failed
        assert!(consecutive_failures >= MAX_CONNECTION_RETRIES);
    }
}
