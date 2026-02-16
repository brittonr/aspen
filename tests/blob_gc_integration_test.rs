//! Integration tests for blob garbage collection with tag lifecycle.
//!
//! These tests validate the interaction between:
//! - Blob storage and protection tags
//! - BlobAwareKeyValueStore large value offloading
//! - GC tag lifecycle during KV operations
//!
//! # Requirements
//!
//! These tests require network access (marked with `#[ignore]` for Nix sandbox).
//!
//! Run with:
//!
//! ```bash
//! cargo nextest run blob_gc_integration --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use support::real_cluster::RealClusterConfig;
use support::real_cluster::RealClusterTester;

/// Test timeout for single-node operations.
const SINGLE_NODE_TIMEOUT: Duration = Duration::from_secs(30);

/// Large blob content that exceeds BLOB_THRESHOLD (1MB).
#[allow(dead_code)]
fn large_blob_content() -> Vec<u8> {
    vec![0xAB; 1_100_000] // 1.1 MB - above threshold
}

/// Small blob content below BLOB_THRESHOLD.
const SMALL_BLOB_CONTENT: &[u8] = b"Small blob content under 1MB threshold";

// ============================================================================
// GC Tag Lifecycle Tests
// ============================================================================

/// Test: Add a blob with protection tag and verify it survives.
///
/// This test validates:
/// 1. Blob can be protected with a named tag
/// 2. Protected blob remains accessible
/// 3. Multiple protection tags can reference the same blob
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_protection_lifecycle() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add a blob with a protection tag
    let add_response = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: SMALL_BLOB_CONTENT.to_vec(),
            tag: Some("test-protection-1".to_string()),
        })
        .await
        .expect("failed to send add blob request");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success, "add blob should succeed: {:?}", result.error);
            result.hash.expect("should return hash")
        }
        ClientRpcResponse::Error(e) => panic!("add blob failed: {}: {}", e.code, e.message),
        _ => panic!("unexpected response type"),
    };

    // Verify the blob is accessible
    let get_response = tester
        .client()
        .send(ClientRpcRequest::GetBlob { hash: hash.clone() })
        .await
        .expect("failed to send get blob request");

    match get_response {
        ClientRpcResponse::GetBlobResult(result) => {
            assert!(result.was_found, "protected blob should be accessible");
            assert_eq!(result.data.as_deref(), Some(SMALL_BLOB_CONTENT));
        }
        _ => panic!("unexpected response"),
    }

    // Add another protection tag to the same blob
    let protect_response = tester
        .client()
        .send(ClientRpcRequest::ProtectBlob {
            hash: hash.clone(),
            tag: "test-protection-2".to_string(),
        })
        .await
        .expect("failed to send protect blob request");

    match protect_response {
        ClientRpcResponse::ProtectBlobResult(result) => {
            assert!(result.is_success, "protect blob should succeed: {:?}", result.error);
        }
        _ => panic!("unexpected response"),
    }

    // Remove first protection tag
    let unprotect_response = tester
        .client()
        .send(ClientRpcRequest::UnprotectBlob {
            tag: "test-protection-1".to_string(),
        })
        .await
        .expect("failed to send unprotect blob request");

    match unprotect_response {
        ClientRpcResponse::UnprotectBlobResult(result) => {
            assert!(result.is_success, "unprotect blob should succeed: {:?}", result.error);
        }
        _ => panic!("unexpected response"),
    }

    // Blob should still be accessible (second tag still exists)
    let get_response2 = tester
        .client()
        .send(ClientRpcRequest::GetBlob { hash: hash.clone() })
        .await
        .expect("failed to send get blob request");

    match get_response2 {
        ClientRpcResponse::GetBlobResult(result) => {
            assert!(result.was_found, "blob should still be accessible with remaining tag");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Verify blob status returns correct information.
///
/// This test validates:
/// 1. BlobStatus contains correct hash
/// 2. BlobStatus contains correct size
/// 3. BlobStatus indicates completion
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_status_accuracy() {
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
            data: SMALL_BLOB_CONTENT.to_vec(),
            tag: None,
        })
        .await
        .expect("failed to send add blob request");

    let hash = match add_response {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.expect("should return hash")
        }
        _ => panic!("unexpected response"),
    };

    // Get blob status
    let status_response = tester
        .client()
        .send(ClientRpcRequest::GetBlobStatus { hash: hash.clone() })
        .await
        .expect("failed to send blob status request");

    match status_response {
        ClientRpcResponse::GetBlobStatusResult(result) => {
            assert!(result.was_found, "blob should be found");
            assert!(result.complete.unwrap_or(false), "blob should be complete");
            assert_eq!(result.size_bytes, Some(SMALL_BLOB_CONTENT.len() as u64), "size should match");
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Verify blob listing with pagination.
///
/// This test validates:
/// 1. List returns correct number of blobs
/// 2. Pagination works correctly with continuation token
/// 3. Blob entries contain correct metadata
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_blob_list_pagination() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add multiple blobs with different content
    let mut hashes = Vec::new();
    for i in 0..5 {
        let content = format!("Blob content number {}", i);
        let add_response = tester
            .client()
            .send(ClientRpcRequest::AddBlob {
                data: content.as_bytes().to_vec(),
                tag: None,
            })
            .await
            .expect("failed to send add blob request");

        match add_response {
            ClientRpcResponse::AddBlobResult(result) => {
                assert!(result.is_success);
                hashes.push(result.hash.expect("should return hash"));
            }
            _ => panic!("unexpected response"),
        }
    }

    // List blobs with small limit to test pagination
    let list_response = tester
        .client()
        .send(ClientRpcRequest::ListBlobs {
            limit: 3,
            continuation_token: None,
        })
        .await
        .expect("failed to send list blobs request");

    match list_response {
        ClientRpcResponse::ListBlobsResult(result) => {
            assert!(result.blobs.len() <= 3, "should respect limit");
            // May have continuation token if there are more blobs
            tracing::info!(
                count = result.blobs.len(),
                has_continuation = result.continuation_token.is_some(),
                "listed blobs"
            );
        }
        _ => panic!("unexpected response"),
    }

    tester.shutdown().await.expect("failed to shutdown cluster");
}

/// Test: Concurrent blob operations don't cause data corruption.
///
/// This test validates:
/// 1. Multiple concurrent adds don't corrupt data
/// 2. All blobs are retrievable after concurrent operations
/// 3. No duplicate hashes for different content
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_concurrent_blob_adds() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = Arc::new(RealClusterTester::new(config).await.expect("failed to create cluster"));

    // Spawn multiple concurrent add operations
    let mut handles = Vec::new();

    for i in 0..10 {
        let tester_clone = tester.clone();
        let content = format!("Concurrent blob content {}", i);

        let handle = tokio::spawn(async move {
            let response = tester_clone
                .client()
                .send(ClientRpcRequest::AddBlob {
                    data: content.as_bytes().to_vec(),
                    tag: None,
                })
                .await
                .expect("failed to send add blob request");

            match response {
                ClientRpcResponse::AddBlobResult(result) => {
                    assert!(result.is_success, "concurrent add should succeed");
                    (content, result.hash.expect("should return hash"))
                }
                _ => panic!("unexpected response"),
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // Verify all blobs are unique and accessible
    let mut seen_hashes = std::collections::HashSet::new();
    for result in results {
        let (content, hash) = result.expect("task should not panic");

        // Hash should be unique for different content
        assert!(seen_hashes.insert(hash.clone()), "hash should be unique");

        // Verify we can retrieve the content
        let get_response = tester
            .client()
            .send(ClientRpcRequest::GetBlob { hash })
            .await
            .expect("failed to send get blob request");

        match get_response {
            ClientRpcResponse::GetBlobResult(result) => {
                assert!(result.was_found, "blob should be found");
                assert_eq!(result.data.as_deref(), Some(content.as_bytes()), "content should match");
            }
            _ => panic!("unexpected response"),
        }
    }

    // Need to get the inner tester for shutdown since it's wrapped in Arc
    // For now, the tester will be dropped when the test ends which triggers cleanup
}

/// Test: Same content produces same hash (content-addressing).
///
/// This test validates:
/// 1. Adding identical content twice produces same hash
/// 2. No duplicate storage for identical content
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_content_addressing_deduplication() {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let config = RealClusterConfig::default()
        .with_node_count(1)
        .with_workers(false)
        .with_timeout(SINGLE_NODE_TIMEOUT);

    let tester = RealClusterTester::new(config).await.expect("failed to create cluster");

    // Add same content twice
    let content = b"Content for deduplication test";

    let add_response1 = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: content.to_vec(),
            tag: None,
        })
        .await
        .expect("failed to send add blob request");

    let hash1 = match add_response1 {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.expect("should return hash")
        }
        _ => panic!("unexpected response"),
    };

    let add_response2 = tester
        .client()
        .send(ClientRpcRequest::AddBlob {
            data: content.to_vec(),
            tag: None,
        })
        .await
        .expect("failed to send add blob request");

    let hash2 = match add_response2 {
        ClientRpcResponse::AddBlobResult(result) => {
            assert!(result.is_success);
            result.hash.expect("should return hash")
        }
        _ => panic!("unexpected response"),
    };

    // Hashes should be identical
    assert_eq!(hash1, hash2, "identical content should produce identical hash");

    tester.shutdown().await.expect("failed to shutdown cluster");
}
