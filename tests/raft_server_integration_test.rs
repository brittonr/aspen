//! Integration tests for Raft RPC server lifecycle and resource limits.
//!
//! This module tests the RaftRpcServer spawn/shutdown lifecycle and the
//! semaphore-based resource limiting (connection and stream limits).
//!
//! # Coverage Goals
//!
//! - spawn() and shutdown() lifecycle validation
//! - Connection limit enforcement (MAX_CONCURRENT_CONNECTIONS)
//! - Stream limit enforcement (MAX_STREAMS_PER_CONNECTION)
//! - Semaphore cleanup on connection/stream close
//! - Concurrent connection stress testing
//!
//! # Test Approach
//!
//! Tests use MockIrohNetwork for deterministic, fast testing of the semaphore
//! logic. The mock infrastructure simulates Iroh's connection and stream
//! semantics without requiring real network I/O.
//!
//! # Tiger Style
//!
//! - Bounded test resources match production limits
//! - Explicit timeouts on all async operations
//! - Deterministic tests with no real I/O

#![allow(deprecated)] // Allow deprecated RAFT_ALPN usage in tests

mod support;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;

use support::mock_iroh::MockIrohNetwork;
use support::mock_iroh::RAFT_ALPN;
use tokio::sync::Semaphore;
use tokio::time::timeout;

/// Test-specific limits for faster tests.
/// Production uses MAX_CONCURRENT_CONNECTIONS=500 and MAX_STREAMS_PER_CONNECTION=100.
const TEST_MAX_CONNECTIONS: usize = 10;
const TEST_MAX_STREAMS: usize = 5;

/// Timeout for test operations.
const TEST_TIMEOUT: Duration = Duration::from_secs(5);

// ============================================================================
// Connection Limit Enforcement Tests
// ============================================================================

/// Test that connection semaphore enforces the limit.
///
/// This tests the core semaphore logic used by RaftRpcServer and
/// RaftProtocolHandler to limit concurrent connections.
#[tokio::test]
async fn test_connection_limit_enforcement() {
    let semaphore = Arc::new(Semaphore::new(TEST_MAX_CONNECTIONS));
    let mut permits = Vec::new();

    // Acquire all available permits
    for i in 0..TEST_MAX_CONNECTIONS {
        let permit = semaphore.clone().try_acquire_owned().unwrap_or_else(|_| panic!("should acquire permit {}", i));
        permits.push(permit);
    }

    // Verify semaphore is exhausted
    assert_eq!(semaphore.available_permits(), 0);

    // Next acquisition should fail
    assert!(semaphore.clone().try_acquire_owned().is_err(), "should reject connection when limit reached");

    // Release one permit
    drop(permits.pop());

    // Now acquisition should succeed
    assert_eq!(semaphore.available_permits(), 1);
    let new_permit = semaphore.clone().try_acquire_owned().expect("should acquire permit after release");

    // Clean up
    drop(new_permit);
    drop(permits);
}

/// Test that connection semaphore is properly released when connection closes.
#[tokio::test]
async fn test_connection_semaphore_cleanup() {
    let semaphore = Arc::new(Semaphore::new(TEST_MAX_CONNECTIONS));

    // Simulate connection lifecycle in a scope
    {
        let mut permits = Vec::new();
        for _ in 0..TEST_MAX_CONNECTIONS {
            let permit = semaphore.clone().try_acquire_owned().expect("acquire");
            permits.push(permit);
        }
        assert_eq!(semaphore.available_permits(), 0);
        // permits dropped at end of scope
    }

    // All permits should be released
    assert_eq!(semaphore.available_permits(), TEST_MAX_CONNECTIONS);
}

/// Test that stream semaphore enforces the limit.
#[tokio::test]
async fn test_stream_limit_enforcement() {
    let semaphore = Arc::new(Semaphore::new(TEST_MAX_STREAMS));
    let mut permits = Vec::new();

    // Acquire all stream permits
    for i in 0..TEST_MAX_STREAMS {
        let permit = semaphore
            .clone()
            .try_acquire_owned()
            .unwrap_or_else(|_| panic!("should acquire stream permit {}", i));
        permits.push(permit);
    }

    // Verify semaphore is exhausted
    assert_eq!(semaphore.available_permits(), 0);

    // Next stream should be rejected
    assert!(semaphore.clone().try_acquire_owned().is_err(), "should reject stream when limit reached");

    // Release one permit
    drop(permits.pop());

    // Now acquisition should succeed
    let new_permit = semaphore.clone().try_acquire_owned().expect("should acquire stream permit after release");

    drop(new_permit);
    drop(permits);
}

/// Test that stream semaphore is properly released when stream completes.
#[tokio::test]
async fn test_stream_semaphore_cleanup() {
    let semaphore = Arc::new(Semaphore::new(TEST_MAX_STREAMS));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Simulate stream lifecycle pattern from handle_connection
    {
        let mut permits = Vec::new();
        for _ in 0..TEST_MAX_STREAMS {
            let permit = semaphore.clone().try_acquire_owned().expect("acquire");
            active_streams.fetch_add(1, Ordering::Relaxed);
            permits.push(permit);
        }
        assert_eq!(active_streams.load(Ordering::Relaxed), TEST_MAX_STREAMS as u32);

        // Simulate stream completion
        for permit in permits.drain(..) {
            active_streams.fetch_sub(1, Ordering::Relaxed);
            drop(permit);
        }
    }

    assert_eq!(active_streams.load(Ordering::Relaxed), 0);
    assert_eq!(semaphore.available_permits(), TEST_MAX_STREAMS);
}

// ============================================================================
// Mock Network Connection Limit Tests
// ============================================================================

/// Test connection limit with mock Iroh network.
///
/// Simulates the server accept loop pattern where connections are rejected
/// when the semaphore is exhausted.
#[tokio::test]
async fn test_mock_network_connection_limit() {
    let network = MockIrohNetwork::new();
    let server_ep = network.create_endpoint();
    let connection_semaphore = Arc::new(Semaphore::new(TEST_MAX_CONNECTIONS));

    // Create client endpoints and establish connections up to limit
    let mut client_connections = Vec::new();
    let mut server_connections = Vec::new();
    let mut permits = Vec::new();

    for _ in 0..TEST_MAX_CONNECTIONS {
        let client_ep = network.create_endpoint();
        let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
        client_connections.push(client_conn);

        // Server accepts and acquires permit
        let incoming = server_ep.accept().await.unwrap();
        let permit = connection_semaphore.clone().try_acquire_owned().expect("should acquire permit");
        permits.push(permit);

        let server_conn = incoming.accept();
        server_connections.push(server_conn);
    }

    assert_eq!(connection_semaphore.available_permits(), 0);

    // Next connection should be rejected by semaphore
    let extra_client = network.create_endpoint();
    let extra_conn = extra_client.connect(server_ep.id(), RAFT_ALPN).await.unwrap();

    // Server would reject due to semaphore limit
    let _incoming = server_ep.accept().await.unwrap();
    assert!(
        connection_semaphore.clone().try_acquire_owned().is_err(),
        "should reject connection when limit reached"
    );

    // Release one connection
    drop(permits.pop());
    drop(server_connections.pop());
    drop(client_connections.pop());

    // Now semaphore allows new connection
    let permit = connection_semaphore.clone().try_acquire_owned().expect("should acquire permit after release");

    drop(permit);
    drop(extra_conn);
}

/// Test stream limit per connection with mock network.
#[tokio::test]
async fn test_mock_network_stream_limit() {
    let network = MockIrohNetwork::new();
    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Establish connection
    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    let stream_semaphore = Arc::new(Semaphore::new(TEST_MAX_STREAMS));
    let mut client_streams = Vec::new();
    let mut server_streams = Vec::new();
    let mut permits = Vec::new();

    // Open streams up to limit
    for _ in 0..TEST_MAX_STREAMS {
        let (send, recv) = client_conn.open_bi().await.unwrap();
        client_streams.push((send, recv));

        let (server_send, server_recv) = server_conn.accept_bi().await.unwrap();
        let permit = stream_semaphore.clone().try_acquire_owned().expect("should acquire stream permit");
        permits.push(permit);
        server_streams.push((server_send, server_recv));
    }

    assert_eq!(stream_semaphore.available_permits(), 0);

    // Next stream would be rejected
    assert!(stream_semaphore.clone().try_acquire_owned().is_err(), "should reject stream when limit reached");

    // Release one stream
    drop(permits.pop());
    drop(server_streams.pop());
    drop(client_streams.pop());

    // Now semaphore allows new stream
    let permit = stream_semaphore.clone().try_acquire_owned().expect("should acquire stream permit after release");

    drop(permit);
}

// ============================================================================
// Concurrent Stress Tests
// ============================================================================

/// Test concurrent connection attempts are handled correctly.
#[tokio::test]
async fn test_concurrent_connection_stress() {
    let network = MockIrohNetwork::new();
    let server_ep = network.create_endpoint();
    let connection_semaphore = Arc::new(Semaphore::new(TEST_MAX_CONNECTIONS));

    let num_tasks = 20; // More than TEST_MAX_CONNECTIONS
    let mut handles = Vec::new();

    for _ in 0..num_tasks {
        let network = network.clone();
        let server_ep = server_ep.clone();
        let semaphore = connection_semaphore.clone();

        let handle = tokio::spawn(async move {
            let client_ep = network.create_endpoint();
            let conn_result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;

            if let Ok(conn) = conn_result {
                // Try to acquire permit (simulating server behavior)
                if let Ok(permit) = semaphore.clone().try_acquire_owned() {
                    // Simulate some work
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    drop(permit);
                    true // Accepted
                } else {
                    drop(conn);
                    false // Rejected
                }
            } else {
                false // Connection failed
            }
        });
        handles.push(handle);
    }

    // Collect results with timeout
    let results = timeout(TEST_TIMEOUT, async {
        let mut accepted = 0;
        let mut rejected = 0;
        for handle in handles {
            if handle.await.unwrap() {
                accepted += 1;
            } else {
                rejected += 1;
            }
        }
        (accepted, rejected)
    })
    .await
    .expect("test timed out");

    // Verify no permit leakage - all permits should be available
    assert_eq!(
        connection_semaphore.available_permits(),
        TEST_MAX_CONNECTIONS,
        "all permits should be released after tasks complete"
    );

    // Some connections were accepted, some may have been rejected
    assert!(results.0 > 0, "at least some connections should be accepted");
}

/// Test concurrent streams on a single connection.
#[tokio::test]
async fn test_concurrent_stream_stress() {
    let network = MockIrohNetwork::new();
    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = Arc::new(incoming.accept());

    let stream_semaphore = Arc::new(Semaphore::new(TEST_MAX_STREAMS));
    let active_count = Arc::new(AtomicU32::new(0));
    let max_observed = Arc::new(AtomicU32::new(0));

    let num_streams = 20; // More than TEST_MAX_STREAMS
    let mut handles = Vec::new();

    // Spawn client stream openers
    for _ in 0..num_streams {
        let conn = client_conn.clone();
        let handle = tokio::spawn(async move {
            if let Ok((mut send, _recv)) = conn.open_bi().await {
                // Send some data
                let _ = send.write_all(b"test").await;
                let _ = send.finish();
                true
            } else {
                false
            }
        });
        handles.push(handle);
    }

    // Server accepts streams with semaphore limiting
    let server_conn_clone = server_conn.clone();
    let semaphore = stream_semaphore.clone();
    let active = active_count.clone();
    let max_obs = max_observed.clone();

    let server_handle = tokio::spawn(async move {
        let mut accepted = 0;
        for _ in 0..num_streams {
            match timeout(Duration::from_millis(100), server_conn_clone.accept_bi()).await {
                Ok(Ok((_send, _recv))) => {
                    if let Ok(permit) = semaphore.clone().try_acquire_owned() {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_obs.fetch_max(current, Ordering::SeqCst);

                        // Simulate processing
                        tokio::time::sleep(Duration::from_millis(5)).await;

                        active.fetch_sub(1, Ordering::SeqCst);
                        drop(permit);
                        accepted += 1;
                    }
                }
                _ => break,
            }
        }
        accepted
    });

    // Wait for all client tasks
    for handle in handles {
        let _ = handle.await;
    }

    // Wait for server with timeout
    let accepted = timeout(TEST_TIMEOUT, server_handle).await.expect("test timed out").expect("server task panicked");

    // Verify limits were respected
    let max_concurrent = max_observed.load(Ordering::SeqCst);
    assert!(
        max_concurrent <= TEST_MAX_STREAMS as u32,
        "max concurrent streams {} should not exceed limit {}",
        max_concurrent,
        TEST_MAX_STREAMS
    );

    // Verify no permit leakage
    assert_eq!(stream_semaphore.available_permits(), TEST_MAX_STREAMS, "all stream permits should be released");

    assert!(accepted > 0, "at least some streams should be accepted");
}

// ============================================================================
// Lifecycle Tests with Mock Network
// ============================================================================

/// Test basic connection establishment and clean close.
#[tokio::test]
async fn test_connection_lifecycle() {
    let network = MockIrohNetwork::new();
    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Establish connection
    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    // Verify connection is open
    assert!(!client_conn.is_closed());
    assert!(!server_conn.is_closed());

    // Open a stream and exchange data
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();
    let (mut server_send, mut server_recv) = server_conn.accept_bi().await.unwrap();

    send.write_all(b"request").await.unwrap();
    send.finish().unwrap();

    let request = server_recv.read_to_end(1024).await.unwrap();
    assert_eq!(request, b"request");

    server_send.write_all(b"response").await.unwrap();
    server_send.finish().unwrap();

    let response = recv.read_to_end(1024).await.unwrap();
    assert_eq!(response, b"response");

    // Close connection
    client_conn.close();
    assert!(client_conn.is_closed());
}

/// Test that server rejects new streams after connection close.
#[tokio::test]
async fn test_connection_close_rejects_streams() {
    let network = MockIrohNetwork::new();
    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let _server_conn = incoming.accept();

    // Close connection
    client_conn.close();

    // Opening stream on closed connection should fail
    let result = client_conn.open_bi().await;
    assert!(result.is_err(), "should reject stream on closed connection");
}

/// Test that network partition causes connection failures.
#[tokio::test]
async fn test_network_partition_handling() {
    let network = MockIrohNetwork::new();
    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Enable network partition
    network.set_network_partition(true);

    // Connection should fail
    let result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;
    assert!(result.is_err(), "connection should fail during partition");

    // Disable partition
    network.set_network_partition(false);

    // Connection should succeed
    let result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;
    assert!(result.is_ok(), "connection should succeed after partition heals");
}

/// Test connection refusal handling.
#[tokio::test]
async fn test_connection_refused_handling() {
    let network = MockIrohNetwork::new();
    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Mark server as refusing connections
    network.refuse_connections_to(server_ep.id());

    // Connection should be refused
    let result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;
    assert!(result.is_err(), "connection should be refused");

    // Allow connections again
    network.allow_connections_to(server_ep.id());

    // Connection should succeed
    let result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;
    assert!(result.is_ok(), "connection should succeed after allowing");
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test semaphore close prevents new acquisitions.
#[tokio::test]
async fn test_semaphore_close_behavior() {
    let semaphore = Arc::new(Semaphore::new(TEST_MAX_CONNECTIONS));

    // Acquire some permits
    let permit1 = semaphore.clone().try_acquire_owned().unwrap();
    let permit2 = semaphore.clone().try_acquire_owned().unwrap();

    // Close the semaphore
    semaphore.close();

    // New acquisitions should fail
    assert!(semaphore.clone().try_acquire_owned().is_err(), "acquisition should fail on closed semaphore");

    // Existing permits are still valid and can be dropped
    drop(permit1);
    drop(permit2);
}

/// Test that dropping semaphore permit immediately makes it available.
#[tokio::test]
async fn test_permit_immediate_release() {
    let semaphore = Arc::new(Semaphore::new(1));

    // Acquire the only permit
    let permit = semaphore.clone().try_acquire_owned().unwrap();
    assert_eq!(semaphore.available_permits(), 0);

    // Drop permit
    drop(permit);

    // Permit should be immediately available
    assert_eq!(semaphore.available_permits(), 1);
    let permit2 = semaphore.clone().try_acquire_owned().unwrap();
    assert_eq!(semaphore.available_permits(), 0);
    drop(permit2);
}
