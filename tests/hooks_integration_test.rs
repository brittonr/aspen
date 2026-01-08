//! Integration tests for hook system with real Iroh networking.
//!
//! These tests validate the hook service RPC operations using actual
//! Iroh P2P networking (not madsim simulation).
//!
//! # Test Categories
//!
//! 1. **Hook List Tests** - List configured handlers
//! 2. **Hook Metrics Tests** - Query execution metrics
//! 3. **Hook Trigger Tests** - Manual event triggering
//!
//! # Requirements
//!
//! These tests require network access and are marked with `#[ignore]` for
//! tests that require networking. Run with:
//!
//! ```bash
//! cargo nextest run hooks_integration --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::time::Duration;

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;

/// Test timeout for single-node operations.
const SINGLE_NODE_TIMEOUT: Duration = Duration::from_secs(30);

/// Test: List hooks on a single node with no handlers configured.
///
/// This test validates the basic hook list operation:
/// 1. Start a single-node cluster
/// 2. Query HookList RPC
/// 3. Verify response contains expected structure
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_hook_list_no_handlers() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Query hook list
    let response = tester.client().send(ClientRpcRequest::HookList).await.expect("failed to send HookList request");

    match response {
        ClientRpcResponse::HookListResult(result) => {
            // Hook service should exist (even if disabled)
            tracing::info!(enabled = result.enabled, handler_count = result.handlers.len(), "hook list received");

            // Default configuration has no handlers
            assert!(result.handlers.is_empty(), "default config should have no handlers");
        }
        ClientRpcResponse::Error(e) => {
            panic!("hook list failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Get hook metrics on a node with no handlers.
///
/// This test validates the metrics query operation:
/// 1. Start a single-node cluster
/// 2. Query HookGetMetrics RPC
/// 3. Verify response contains expected structure
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_hook_metrics_no_handlers() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Query hook metrics
    let response = tester
        .client()
        .send(ClientRpcRequest::HookGetMetrics { handler_name: None })
        .await
        .expect("failed to send HookGetMetrics request");

    match response {
        ClientRpcResponse::HookMetricsResult(result) => {
            tracing::info!(
                enabled = result.enabled,
                total_events = result.total_events_processed,
                handler_count = result.handlers.len(),
                "hook metrics received"
            );

            // No events should have been processed with no handlers
            assert!(result.handlers.is_empty(), "should have no handler metrics");
        }
        ClientRpcResponse::Error(e) => {
            panic!("hook metrics failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Trigger hook event with no handlers configured.
///
/// This test validates the manual trigger operation:
/// 1. Start a single-node cluster
/// 2. Trigger a write_committed event
/// 3. Verify response indicates success (0 handlers dispatched)
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_hook_trigger_no_handlers() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Trigger a hook event
    let response = tester
        .client()
        .send(ClientRpcRequest::HookTrigger {
            event_type: "write_committed".to_string(),
            payload: serde_json::json!({"key": "test", "value": "hello"}),
        })
        .await
        .expect("failed to send HookTrigger request");

    match response {
        ClientRpcResponse::HookTriggerResult(result) => {
            tracing::info!(
                success = result.success,
                dispatched_count = result.dispatched_count,
                error = ?result.error,
                "hook trigger result"
            );

            // With hooks disabled or no handlers, should succeed with 0 dispatches
            // OR return an error if the service is not available
            if let Some(error) = &result.error {
                tracing::info!(error = %error, "hook service not available (expected in some configs)");
            } else {
                assert_eq!(result.dispatched_count, 0, "should dispatch to 0 handlers with no config");
                assert!(result.handler_failures.is_empty(), "should have no handler failures");
            }
        }
        ClientRpcResponse::Error(e) => {
            panic!("hook trigger failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Trigger hook event with invalid event type.
///
/// This test validates error handling for invalid event types:
/// 1. Start a single-node cluster
/// 2. Trigger an invalid event type
/// 3. Verify response contains an error
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_hook_trigger_invalid_event_type() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Trigger an invalid event type
    let response = tester
        .client()
        .send(ClientRpcRequest::HookTrigger {
            event_type: "invalid_event_type".to_string(),
            payload: serde_json::json!({}),
        })
        .await
        .expect("failed to send HookTrigger request");

    match response {
        ClientRpcResponse::HookTriggerResult(result) => {
            tracing::info!(success = result.success, error = ?result.error, "hook trigger result for invalid type");

            // Should fail with an error about unknown event type
            assert!(!result.success, "should fail for invalid event type");
            assert!(result.error.is_some(), "should have an error message");
            let error = result.error.unwrap();
            assert!(error.contains("unknown event type"), "error should mention unknown event type: {}", error);
        }
        ClientRpcResponse::Error(e) => {
            panic!("hook trigger RPC failed unexpectedly: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: Hook metrics with filter by handler name.
///
/// This test validates the handler name filter:
/// 1. Start a single-node cluster
/// 2. Query metrics with a non-existent handler name
/// 3. Verify empty result
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_hook_metrics_with_filter() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Query hook metrics with filter
    let response = tester
        .client()
        .send(ClientRpcRequest::HookGetMetrics {
            handler_name: Some("non_existent_handler".to_string()),
        })
        .await
        .expect("failed to send HookGetMetrics request");

    match response {
        ClientRpcResponse::HookMetricsResult(result) => {
            tracing::info!(handler_count = result.handlers.len(), "hook metrics with filter received");

            // Should have no handlers matching the filter
            assert!(result.handlers.is_empty(), "should have no matching handlers");
        }
        ClientRpcResponse::Error(e) => {
            panic!("hook metrics failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("shutdown failed");
}

/// Test: All supported hook event types.
///
/// This test validates that all event types are recognized:
/// 1. Start a single-node cluster
/// 2. Trigger each supported event type
/// 3. Verify all are accepted (no "unknown event type" errors)
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_hook_trigger_all_event_types() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    let event_types = [
        "write_committed",
        "delete_committed",
        "membership_changed",
        "leader_elected",
        "snapshot_created",
    ];

    for event_type in &event_types {
        let response = tester
            .client()
            .send(ClientRpcRequest::HookTrigger {
                event_type: event_type.to_string(),
                payload: serde_json::json!({}),
            })
            .await
            .expect(&format!("failed to send HookTrigger for {}", event_type));

        match response {
            ClientRpcResponse::HookTriggerResult(result) => {
                tracing::info!(event_type = %event_type, success = result.success, error = ?result.error, "trigger result");

                // Should not have "unknown event type" error
                if let Some(error) = &result.error {
                    assert!(
                        !error.contains("unknown event type"),
                        "event type '{}' should be recognized, got error: {}",
                        event_type,
                        error
                    );
                }
            }
            ClientRpcResponse::Error(e) => {
                panic!("hook trigger RPC failed for {}: {}: {}", event_type, e.code, e.message);
            }
            _ => panic!("unexpected response type for {}", event_type),
        }
    }

    tester.shutdown().await.expect("shutdown failed");
}
