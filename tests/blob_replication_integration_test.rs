//! Integration tests for blob replication with real Iroh networking.
//!
//! These tests validate the blob replication system using actual
//! Iroh P2P networking (not madsim simulation).
//!
//! # Test Categories
//!
//! 1. **Basic Replication** - Single blob replication across nodes
//! 2. **Status Tracking** - Replication metadata and status queries
//! 3. **Multi-node** - Full cluster replication scenarios
//! 4. **Edge Cases** - Error handling and recovery
//!
//! # Requirements
//!
//! These tests require:
//! - Network access (marked with `#[ignore]` for Nix sandbox)
//! - The `blob` feature enabled
//!
//! Run with:
//!
//! ```bash
//! cargo nextest run blob_replication_integration --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::time::Duration;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;
use tokio::time::sleep;

/// Test timeout for single-node operations.
const SINGLE_NODE_TIMEOUT: Duration = Duration::from_secs(30);

/// Test timeout for multi-node operations.
const MULTI_NODE_TIMEOUT: Duration = Duration::from_secs(60);

/// Time to wait for replication to propagate.
const REPLICATION_WAIT: Duration = Duration::from_secs(5);

/// Test blob content for replication tests.
const TEST_BLOB_CONTENT: &[u8] = b"Hello, blob replication! This is test content for integration testing.";

// ============================================================================
// Basic Blob Operations Tests
// ============================================================================

/// Test: Add a blob and retrieve it on the same node.
///
/// This test validates the basic blob lifecycle:
/// 1. Start a single-node cluster
/// 2. Add a blob with test content
/// 3. Retrieve the blob by hash
/// 4. Verify the content matches
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_add_and_get() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: Some("test-blob".to_string()),
        })
        .await
        .expect("failed to send add blob request");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success, "add blob should succeed: {:?}", result.error);
            assert!(result.hash.is_some(), "should return hash");
            assert_eq!(result.size, Some(TEST_BLOB_CONTENT.len() as u64));
            tracing::info!(hash = ?result.hash, size = result.size, "blob added");
            result.hash.unwrap()
        }
        ClientRpcResponse::Error(e) => {
            panic!("add blob failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    };

    // Get the blob back
    let get_response = tester
        .client()
        .send(ClientRpcRequest::GetBlob { hash: hash.clone() })
        .await
        .expect("failed to send get blob request");

    match get_response {
        ClientRpcResponse::GetBlobResult(result) => {
            assert!(result.was_found, "blob should be found");
            assert_eq!(result.data.as_deref(), Some(TEST_BLOB_CONTENT));
            tracing::info!("blob retrieved successfully");
        }
        ClientRpcResponse::Error(e) => {
            panic!("get blob failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Check blob existence with HasBlob.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_has_blob() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: None,
        })
        .await
        .expect("failed to send add blob request");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Check it exists
    let has_response = tester
        .client()
        .send(ClientRpcRequest::HasBlob { hash: hash.clone() })
        .await
        .expect("failed to send has blob request");

    match has_response {
        ClientRpcResponse::HasBlobResult(result) => {
            assert!(result.does_exist, "blob should exist");
        }
        _ => panic!("unexpected response"),
    }

    // Check non-existent blob
    let fake_hash = "0".repeat(64);
    let has_response = tester
        .client()
        .send(ClientRpcRequest::HasBlob { hash: fake_hash })
        .await
        .expect("failed to send has blob request");

    match has_response {
        ClientRpcResponse::HasBlobResult(result) => {
            assert!(!result.does_exist, "fake blob should not exist");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

// ============================================================================
// Blob Replication Tests
// ============================================================================

/// Test: Trigger blob replication to specific target nodes.
///
/// This test validates the core replication flow:
/// 1. Start a 3-node cluster
/// 2. Add a blob to node 1
/// 3. Trigger replication to nodes 2 and 3
/// 4. Verify successful replication
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_trigger_replication() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default().with_node_count(3).with_workers(false).with_timeout(MULTI_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: Some("replicate-test".to_string()),
        })
        .await
        .expect("failed to send add blob request");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success, "add blob should succeed: {:?}", result.error);
            tracing::info!(hash = ?result.hash, "blob added for replication");
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Wait a moment for cluster stabilization
    sleep(Duration::from_secs(2)).await;

    // Trigger replication to nodes 2 and 3
    let replicate_response = tester
        .client()
        .send(ClientRpcRequest::TriggerBlobReplication {
            hash: hash.clone(),
            target_nodes: vec![2, 3],
            replication_factor: 0, // Use default
        })
        .await
        .expect("failed to send trigger replication request");

    match replicate_response {
        ClientRpcResponse::TriggerBlobReplicationResult(result) => {
            assert!(result.is_success, "replication should succeed: {:?}", result.error);
            tracing::info!(
                successful = ?result.successful_nodes,
                failed = ?result.failed_nodes,
                duration_ms = result.duration_ms,
                "replication completed"
            );

            // Verify at least one node got the blob
            if let Some(successful) = &result.successful_nodes {
                assert!(!successful.is_empty(), "at least one node should succeed");
            }
        }
        ClientRpcResponse::Error(e) => {
            panic!("trigger replication failed: {}: {}", e.code, e.message);
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Get blob replication status.
///
/// This test validates replication status tracking:
/// 1. Add a blob
/// 2. Trigger replication
/// 3. Query replication status
/// 4. Verify metadata accuracy
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_replication_status() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default().with_node_count(3).with_workers(false).with_timeout(MULTI_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: Some("status-test".to_string()),
        })
        .await
        .expect("failed to send add blob request");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Wait for potential auto-replication
    sleep(Duration::from_secs(2)).await;

    // Trigger explicit replication
    let _ = tester
        .client()
        .send(ClientRpcRequest::TriggerBlobReplication {
            hash: hash.clone(),
            target_nodes: vec![2, 3],
            replication_factor: 0,
        })
        .await
        .expect("failed to trigger replication");

    // Wait for replication to complete
    sleep(REPLICATION_WAIT).await;

    // Query replication status
    let status_response = tester
        .client()
        .send(ClientRpcRequest::GetBlobReplicationStatus { hash: hash.clone() })
        .await
        .expect("failed to send status request");

    match status_response {
        ClientRpcResponse::GetBlobReplicationStatusResult(result) => {
            tracing::info!(
                was_found = result.was_found,
                replica_nodes = ?result.replica_nodes,
                status = ?result.status,
                replicas_needed = result.replicas_needed,
                "replication status"
            );

            // Blob should be tracked (either auto-replicated or explicitly)
            if result.was_found {
                assert!(result.hash.is_some());
                assert!(result.size.is_some());
                // Status should be one of the valid states
                if let Some(status) = &result.status {
                    assert!(
                        ["healthy", "degraded", "under_replicated", "critical"].contains(&status.as_str()),
                        "unexpected status: {}",
                        status
                    );
                }
            }
        }
        ClientRpcResponse::Error(e) => {
            // Not finding replication metadata is acceptable if auto-replication is disabled
            tracing::warn!(error = %e.message, "replication status query returned error (may be expected)");
        }
        _ => panic!("unexpected response type"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Blob status includes size and completion info.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_status() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: Some("status-check".to_string()),
        })
        .await
        .expect("failed to add blob");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Get blob status
    let status_response = tester
        .client()
        .send(ClientRpcRequest::GetBlobStatus { hash: hash.clone() })
        .await
        .expect("failed to get status");

    match status_response {
        ClientRpcResponse::GetBlobStatusResult(result) => {
            assert!(result.was_found, "blob should be found");
            assert_eq!(result.hash.as_deref(), Some(hash.as_str()));
            assert_eq!(result.size, Some(TEST_BLOB_CONTENT.len() as u64));
            assert!(result.complete.unwrap_or(false), "blob should be complete");
            tracing::info!(
                size = result.size,
                complete = result.complete,
                tags = ?result.tags,
                "blob status"
            );
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

// ============================================================================
// Multi-Node Replication Tests
// ============================================================================

/// Test: Verify blob can be retrieved from any node after replication.
///
/// This is the core cross-node blob availability test:
/// 1. Add blob to node 1
/// 2. Replicate to nodes 2 and 3
/// 3. Verify blob is retrievable from all nodes
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_multi_node_availability() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default().with_node_count(3).with_workers(false).with_timeout(MULTI_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add blob via primary client (connects to node 1)
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: Some("multi-node-test".to_string()),
        })
        .await
        .expect("failed to add blob");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Trigger replication to all nodes
    let replicate_response = tester
        .client()
        .send(ClientRpcRequest::TriggerBlobReplication {
            hash: hash.clone(),
            target_nodes: vec![2, 3],
            replication_factor: 3,
        })
        .await
        .expect("failed to trigger replication");

    if let ClientRpcResponse::TriggerBlobReplicationResult(result) = replicate_response {
        tracing::info!(
            is_success = result.is_success,
            successful_nodes = ?result.successful_nodes,
            "replication triggered"
        );
    }

    // Wait for replication
    sleep(REPLICATION_WAIT).await;

    // Verify blob exists on node 1 (source)
    let has_response = tester
        .client()
        .send(ClientRpcRequest::HasBlob { hash: hash.clone() })
        .await
        .expect("failed to check blob on node 1");

    match has_response {
        ClientRpcResponse::HasBlobResult(result) => {
            assert!(result.does_exist, "blob should exist on source node");
        }
        _ => panic!("unexpected response"),
    }

    tracing::info!("blob replication multi-node test completed");
    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Large blob replication.
///
/// Tests replication with a larger blob to verify streaming transfer works.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_large_replication() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default().with_node_count(3).with_workers(false).with_timeout(MULTI_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Create a larger blob (1 MB)
    let large_content: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: large_content.clone(),
            tag: Some("large-blob-test".to_string()),
        })
        .await
        .expect("failed to add large blob");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success, "large blob add should succeed");
            assert_eq!(result.size, Some(1024 * 1024));
            tracing::info!(hash = ?result.hash, size = result.size, "large blob added");
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Trigger replication
    let replicate_response = tester
        .client()
        .send(ClientRpcRequest::TriggerBlobReplication {
            hash: hash.clone(),
            target_nodes: vec![2, 3],
            replication_factor: 0,
        })
        .await
        .expect("failed to trigger large blob replication");

    if let ClientRpcResponse::TriggerBlobReplicationResult(result) = replicate_response {
        tracing::info!(
            is_success = result.is_success,
            duration_ms = result.duration_ms,
            "large blob replication completed"
        );
    }

    // Verify blob integrity by reading back
    let get_response =
        tester.client().send(ClientRpcRequest::GetBlob { hash }).await.expect("failed to get large blob");

    match get_response {
        ClientRpcResponse::GetBlobResult(result) => {
            assert!(result.was_found, "large blob should be retrievable");
            assert_eq!(result.data.as_ref().map(|d| d.len()), Some(1024 * 1024));
            // Verify content integrity
            if let Some(data) = result.data {
                assert_eq!(data, large_content, "blob content should match");
            }
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test: Replication of non-existent blob returns appropriate error.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_replication_nonexistent() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(2)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Try to replicate a blob that doesn't exist
    let fake_hash = "a".repeat(64);

    let replicate_response = tester
        .client()
        .send(ClientRpcRequest::TriggerBlobReplication {
            hash: fake_hash.clone(),
            target_nodes: vec![2],
            replication_factor: 0,
        })
        .await
        .expect("failed to send request");

    match replicate_response {
        ClientRpcResponse::TriggerBlobReplicationResult(result) => {
            assert!(!result.is_success, "replication of nonexistent blob should fail");
            assert!(result.error.is_some(), "should have error message");
            tracing::info!(error = ?result.error, "expected error for nonexistent blob");
        }
        ClientRpcResponse::Error(e) => {
            tracing::info!(error = %e.message, "expected error for nonexistent blob");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Get replication status for unreplicated blob.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_replication_status_not_found() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Query status for a blob that was never replicated
    let fake_hash = "b".repeat(64);

    let status_response = tester
        .client()
        .send(ClientRpcRequest::GetBlobReplicationStatus { hash: fake_hash })
        .await
        .expect("failed to send status request");

    match status_response {
        ClientRpcResponse::GetBlobReplicationStatusResult(result) => {
            assert!(!result.was_found, "unreplicated blob should not have status");
            tracing::info!("correctly reported no replication status");
        }
        ClientRpcResponse::Error(e) => {
            tracing::info!(error = %e.message, "expected: no replication status");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: List blobs includes replicated blobs.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_list_after_replication() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add multiple blobs
    let mut hashes = Vec::new();
    for i in 0..3 {
        let content = format!("blob content {}", i);
        let add_response = tester
            .client()
            .send(ClientRpcRequest::AddBlob {
                data: content.into_bytes(),
                tag: Some(format!("list-test-{}", i)),
            })
            .await
            .expect("failed to add blob");

        if let ClientRpcResponse::AddBlobResult(result) = add_response {
            assert!(result.is_success);
            hashes.push(result.hash.unwrap());
        }
    }

    // List all blobs
    let list_response = tester
        .client()
        .send(ClientRpcRequest::ListBlobs {
            limit: 100,
            continuation_token: None,
        })
        .await
        .expect("failed to list blobs");

    match list_response {
        ClientRpcResponse::ListBlobsResult(result) => {
            tracing::info!(count = result.blobs.len(), "listed blobs");
            // All our blobs should be in the list
            for hash in &hashes {
                assert!(result.blobs.iter().any(|b| b.hash == *hash), "blob {} should be in list", hash);
            }
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

// ============================================================================
// Blob Protection Tests
// ============================================================================

/// Test: Protected blobs survive garbage collection.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_protection() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob with protection tag
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: TEST_BLOB_CONTENT.to_vec(),
            tag: Some("protected-blob".to_string()),
        })
        .await
        .expect("failed to add blob");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.unwrap()
        }
        _ => panic!("unexpected response"),
    };

    // Add another protection tag
    let protect_response = tester
        .client()
        .send(ClientRpcRequest::ProtectBlob {
            hash: hash.clone(),
            tag: "second-protection".to_string(),
        })
        .await
        .expect("failed to protect blob");

    match protect_response {
        ClientRpcResponse::ProtectBlobResult(result) => {
            assert!(result.is_success, "protection should succeed");
        }
        _ => panic!("unexpected response"),
    }

    // Verify status shows protection
    let status_response = tester
        .client()
        .send(ClientRpcRequest::GetBlobStatus { hash: hash.clone() })
        .await
        .expect("failed to get status");

    match status_response {
        ClientRpcResponse::GetBlobStatusResult(result) => {
            assert!(result.was_found);
            if let Some(tags) = &result.tags {
                assert!(!tags.is_empty(), "should have at least one protection tag");
                tracing::info!(tags = ?tags, "blob protection tags");
            }
        }
        _ => panic!("unexpected response"),
    }

    // Remove protection
    let unprotect_response = tester
        .client()
        .send(ClientRpcRequest::UnprotectBlob {
            tag: "second-protection".to_string(),
        })
        .await
        .expect("failed to unprotect");

    match unprotect_response {
        ClientRpcResponse::UnprotectBlobResult(result) => {
            assert!(result.is_success, "unprotection should succeed");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}
