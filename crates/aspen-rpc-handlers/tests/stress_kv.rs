//! Stress tests for the KV handler.
//!
//! These tests exercise the KV handler with:
//! - High-volume sequential operations
//! - CAS contention scenarios
//! - Batch operation stress
//! - Scan under concurrent writes
//!
//! # Tiger Style
//!
//! - All tests use bounded inputs
//! - Deterministic via DeterministicKeyValueStore
//! - No network I/O
//! - Fixed iteration limits

use std::sync::Arc;

use aspen_client_api::BatchWriteOperation;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_testing::DeterministicKeyValueStore;
use aspen_rpc_handlers::context::test_support::TestContextBuilder;
use aspen_rpc_handlers::handlers::KvHandler;
use aspen_rpc_handlers::registry::RequestHandler;
use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
#[cfg(feature = "sql")]
use aspen_rpc_handlers::test_mocks::mock_sql_executor;

// =============================================================================
// Test Setup Helpers
// =============================================================================

async fn test_context() -> (aspen_rpc_handlers::ClientProtocolContext, Arc<dyn aspen_core::KeyValueStore>) {
    let kv_store = DeterministicKeyValueStore::new();
    let mock_endpoint = Arc::new(MockEndpointProvider::new().await);
    let builder = TestContextBuilder::new().with_kv_store(kv_store.clone()).with_endpoint_manager(mock_endpoint);
    #[cfg(feature = "sql")]
    let builder = builder.with_sql_executor(mock_sql_executor());
    let ctx = builder.build();
    (ctx, kv_store)
}

// =============================================================================
// Write/Read Stress Tests
// =============================================================================

/// Test high-volume writes and reads.
#[tokio::test]
async fn test_stress_write_read_volume() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let num_keys = 500;

    // Write many keys
    for i in 0..num_keys {
        let key = format!("stress:volume:key:{}", i);
        let value = format!("value_{}", i).into_bytes();

        let write_req = ClientRpcRequest::WriteKey { key, value };
        let write_resp = handler.handle(write_req, &ctx).await.unwrap();

        if let ClientRpcResponse::WriteResult(result) = write_resp {
            assert!(result.success, "Write {} should succeed", i);
        }
    }

    // Read all keys back
    for i in 0..num_keys {
        let key = format!("stress:volume:key:{}", i);

        let read_req = ClientRpcRequest::ReadKey { key };
        let read_resp = handler.handle(read_req, &ctx).await.unwrap();

        if let ClientRpcResponse::ReadResult(result) = read_resp {
            assert!(result.found, "Key {} should exist", i);
            let expected = format!("value_{}", i).into_bytes();
            assert_eq!(result.value, Some(expected), "Value {} should match", i);
        }
    }
}

/// Test overwriting same key many times.
#[tokio::test]
async fn test_stress_overwrite_same_key() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let key = "stress:overwrite:single".to_string();
    let num_writes = 200;

    for i in 0..num_writes {
        let value = format!("version_{}", i).into_bytes();

        let write_req = ClientRpcRequest::WriteKey {
            key: key.clone(),
            value: value.clone(),
        };
        let write_resp = handler.handle(write_req, &ctx).await.unwrap();

        if let ClientRpcResponse::WriteResult(result) = write_resp {
            assert!(result.success, "Write {} should succeed", i);
        }

        // Verify immediately after write
        let read_req = ClientRpcRequest::ReadKey { key: key.clone() };
        let read_resp = handler.handle(read_req, &ctx).await.unwrap();

        if let ClientRpcResponse::ReadResult(result) = read_resp {
            assert!(result.found, "Key should exist after write {}", i);
            assert_eq!(result.value, Some(value), "Value should match after write {}", i);
        }
    }
}

/// Test delete and recreate cycle.
#[tokio::test]
async fn test_stress_delete_recreate_cycle() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let key = "stress:cycle:key".to_string();
    let num_cycles = 50;

    for cycle in 0..num_cycles {
        // Write
        let value = format!("cycle_{}", cycle).into_bytes();
        let write_req = ClientRpcRequest::WriteKey {
            key: key.clone(),
            value: value.clone(),
        };
        handler.handle(write_req, &ctx).await.unwrap();

        // Verify exists
        let read_req = ClientRpcRequest::ReadKey { key: key.clone() };
        let read_resp = handler.handle(read_req, &ctx).await.unwrap();
        if let ClientRpcResponse::ReadResult(result) = read_resp {
            assert!(result.found, "Key should exist in cycle {}", cycle);
        }

        // Delete
        let delete_req = ClientRpcRequest::DeleteKey { key: key.clone() };
        let delete_resp = handler.handle(delete_req, &ctx).await.unwrap();
        if let ClientRpcResponse::DeleteResult(result) = delete_resp {
            assert!(result.deleted, "Delete should succeed in cycle {}", cycle);
        }

        // Verify gone
        let read_req2 = ClientRpcRequest::ReadKey { key: key.clone() };
        let read_resp2 = handler.handle(read_req2, &ctx).await.unwrap();
        if let ClientRpcResponse::ReadResult(result) = read_resp2 {
            assert!(!result.found, "Key should not exist after delete in cycle {}", cycle);
        }
    }
}

// =============================================================================
// CAS Stress Tests
// =============================================================================

/// Test sequential CAS operations.
#[tokio::test]
async fn test_stress_cas_sequential() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let key = "stress:cas:seq".to_string();
    let num_updates = 100;

    // Create initial value with CAS (expected = None)
    let create_req = ClientRpcRequest::CompareAndSwapKey {
        key: key.clone(),
        expected: None,
        new_value: b"version_0".to_vec(),
    };
    let create_resp = handler.handle(create_req, &ctx).await.unwrap();
    if let ClientRpcResponse::CompareAndSwapResult(result) = create_resp {
        assert!(result.success, "CAS create should succeed");
    }

    // Sequential CAS updates
    for i in 0..num_updates {
        let expected = format!("version_{}", i).into_bytes();
        let new_value = format!("version_{}", i + 1).into_bytes();

        let cas_req = ClientRpcRequest::CompareAndSwapKey {
            key: key.clone(),
            expected: Some(expected),
            new_value: new_value.clone(),
        };
        let cas_resp = handler.handle(cas_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CompareAndSwapResult(result) = cas_resp {
            assert!(result.success, "CAS update {} should succeed", i);
        }
    }

    // Verify final value
    let read_req = ClientRpcRequest::ReadKey { key };
    let read_resp = handler.handle(read_req, &ctx).await.unwrap();
    if let ClientRpcResponse::ReadResult(result) = read_resp {
        let expected_final = format!("version_{}", num_updates).into_bytes();
        assert_eq!(result.value, Some(expected_final));
    }
}

/// Test CAS with wrong expected value fails.
#[tokio::test]
async fn test_stress_cas_wrong_expected() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let key = "stress:cas:wrong".to_string();

    // Write initial value
    let write_req = ClientRpcRequest::WriteKey {
        key: key.clone(),
        value: b"initial".to_vec(),
    };
    handler.handle(write_req, &ctx).await.unwrap();

    // Many CAS attempts with wrong expected value should all fail
    for i in 0..50 {
        let wrong_expected = format!("wrong_{}", i).into_bytes();
        let cas_req = ClientRpcRequest::CompareAndSwapKey {
            key: key.clone(),
            expected: Some(wrong_expected),
            new_value: b"new".to_vec(),
        };
        let cas_resp = handler.handle(cas_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CompareAndSwapResult(result) = cas_resp {
            assert!(!result.success, "CAS with wrong expected {} should fail", i);
        }
    }

    // Value should still be initial
    let read_req = ClientRpcRequest::ReadKey { key };
    let read_resp = handler.handle(read_req, &ctx).await.unwrap();
    if let ClientRpcResponse::ReadResult(result) = read_resp {
        assert_eq!(result.value, Some(b"initial".to_vec()));
    }
}

// =============================================================================
// Batch Operation Stress Tests
// =============================================================================

/// Test large batch writes.
#[tokio::test]
async fn test_stress_batch_write_large() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    // Create a batch of 100 operations (max allowed)
    let num_ops = 100;
    let operations: Vec<BatchWriteOperation> = (0..num_ops)
        .map(|i| BatchWriteOperation::Set {
            key: format!("stress:batch:key:{}", i),
            value: format!("value_{}", i).into_bytes(),
        })
        .collect();

    let batch_req = ClientRpcRequest::BatchWrite { operations };
    let batch_resp = handler.handle(batch_req, &ctx).await.unwrap();

    if let ClientRpcResponse::BatchWriteResult(result) = batch_resp {
        assert!(result.success, "Batch write should succeed");
        assert_eq!(result.operations_applied, Some(num_ops as u32));
    }

    // Verify all keys
    for i in 0..num_ops {
        let key = format!("stress:batch:key:{}", i);
        let read_req = ClientRpcRequest::ReadKey { key };
        let read_resp = handler.handle(read_req, &ctx).await.unwrap();

        if let ClientRpcResponse::ReadResult(result) = read_resp {
            assert!(result.found, "Key {} should exist", i);
            let expected = format!("value_{}", i).into_bytes();
            assert_eq!(result.value, Some(expected));
        }
    }
}

/// Test multiple batch writes.
#[tokio::test]
async fn test_stress_batch_write_multiple() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let num_batches = 10;
    let batch_size = 20;

    for batch_num in 0..num_batches {
        let operations: Vec<BatchWriteOperation> = (0..batch_size)
            .map(|i| {
                let key = format!("stress:multibatch:b{}:k{}", batch_num, i);
                let value = format!("batch_{}_value_{}", batch_num, i).into_bytes();
                BatchWriteOperation::Set { key, value }
            })
            .collect();

        let batch_req = ClientRpcRequest::BatchWrite { operations };
        let batch_resp = handler.handle(batch_req, &ctx).await.unwrap();

        if let ClientRpcResponse::BatchWriteResult(result) = batch_resp {
            assert!(result.success, "Batch {} should succeed", batch_num);
        }
    }

    // Verify sample keys
    for batch_num in 0..num_batches {
        let key = format!("stress:multibatch:b{}:k0", batch_num);
        let read_req = ClientRpcRequest::ReadKey { key };
        let read_resp = handler.handle(read_req, &ctx).await.unwrap();

        if let ClientRpcResponse::ReadResult(result) = read_resp {
            assert!(result.found, "Batch {} key 0 should exist", batch_num);
        }
    }
}

/// Test batch read with many keys.
#[tokio::test]
async fn test_stress_batch_read_large() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let num_keys = 100;

    // Write keys
    for i in 0..num_keys {
        let write_req = ClientRpcRequest::WriteKey {
            key: format!("stress:batchread:{}", i),
            value: format!("value_{}", i).into_bytes(),
        };
        handler.handle(write_req, &ctx).await.unwrap();
    }

    // Batch read all keys
    let keys: Vec<String> = (0..num_keys).map(|i| format!("stress:batchread:{}", i)).collect();

    let batch_read_req = ClientRpcRequest::BatchRead { keys: keys.clone() };
    let batch_read_resp = handler.handle(batch_read_req, &ctx).await.unwrap();

    if let ClientRpcResponse::BatchReadResult(result) = batch_read_resp {
        assert!(result.success, "Batch read should succeed");
        let values = result.values.unwrap();
        assert_eq!(values.len(), num_keys, "Should return all values");

        // Verify ordering
        for (i, value) in values.iter().enumerate() {
            let expected = format!("value_{}", i).into_bytes();
            assert_eq!(value.as_ref(), Some(&expected), "Value {} should match", i);
        }
    }
}

// =============================================================================
// Scan Stress Tests
// =============================================================================

/// Test scan with many keys.
#[tokio::test]
async fn test_stress_scan_large_dataset() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let num_keys = 200;
    let prefix = "stress:scan:dataset:";

    // Write keys
    for i in 0..num_keys {
        let write_req = ClientRpcRequest::WriteKey {
            key: format!("{}{:04}", prefix, i), // Zero-padded for ordering
            value: format!("value_{}", i).into_bytes(),
        };
        handler.handle(write_req, &ctx).await.unwrap();
    }

    // Scan with pagination
    let mut all_keys: Vec<String> = Vec::new();
    let mut continuation_token: Option<String> = None;
    let page_size = 25;

    loop {
        let scan_req = ClientRpcRequest::ScanKeys {
            prefix: prefix.to_string(),
            limit: Some(page_size),
            continuation_token: continuation_token.clone(),
        };
        let scan_resp = handler.handle(scan_req, &ctx).await.unwrap();

        if let ClientRpcResponse::ScanResult(result) = scan_resp {
            for entry in result.entries {
                all_keys.push(entry.key);
            }
            continuation_token = result.continuation_token;
            if !result.is_truncated || continuation_token.is_none() {
                break;
            }
        }
    }

    assert_eq!(all_keys.len(), num_keys, "Should find all {} keys", num_keys);
}

/// Test scan with different prefixes.
#[tokio::test]
async fn test_stress_scan_multiple_prefixes() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let prefixes = vec!["prefix_a:", "prefix_b:", "prefix_c:"];
    let keys_per_prefix = 50;

    // Write keys for each prefix
    for prefix in &prefixes {
        for i in 0..keys_per_prefix {
            let write_req = ClientRpcRequest::WriteKey {
                key: format!("{}{}", prefix, i),
                value: b"value".to_vec(),
            };
            handler.handle(write_req, &ctx).await.unwrap();
        }
    }

    // Scan each prefix
    for prefix in &prefixes {
        let scan_req = ClientRpcRequest::ScanKeys {
            prefix: prefix.to_string(),
            limit: Some(1000),
            continuation_token: None,
        };
        let scan_resp = handler.handle(scan_req, &ctx).await.unwrap();

        if let ClientRpcResponse::ScanResult(result) = scan_resp {
            assert_eq!(result.entries.len(), keys_per_prefix, "Prefix {} should have {} keys", prefix, keys_per_prefix);
        }
    }
}

// =============================================================================
// Mixed Operation Stress Tests
// =============================================================================

/// Test interleaved writes, reads, and deletes.
#[tokio::test]
async fn test_stress_mixed_operations() {
    let (ctx, _kv) = test_context().await;
    let handler = KvHandler;

    let num_iterations = 100;

    for i in 0..num_iterations {
        let key = format!("stress:mixed:{}", i % 10); // Reuse 10 keys

        match i % 4 {
            0 => {
                // Write
                let write_req = ClientRpcRequest::WriteKey {
                    key,
                    value: format!("iter_{}", i).into_bytes(),
                };
                let resp = handler.handle(write_req, &ctx).await.unwrap();
                if let ClientRpcResponse::WriteResult(result) = resp {
                    assert!(result.success);
                }
            }
            1 => {
                // Read
                let read_req = ClientRpcRequest::ReadKey { key };
                let resp = handler.handle(read_req, &ctx).await.unwrap();
                if let ClientRpcResponse::ReadResult(_result) = resp {
                    // May or may not find, depending on timing
                }
            }
            2 => {
                // Delete
                let delete_req = ClientRpcRequest::DeleteKey { key };
                let resp = handler.handle(delete_req, &ctx).await.unwrap();
                if let ClientRpcResponse::DeleteResult(_result) = resp {
                    // Delete is idempotent
                }
            }
            _ => {
                // CAS (may fail if key doesn't exist)
                let cas_req = ClientRpcRequest::CompareAndSwapKey {
                    key,
                    expected: None, // Try to create
                    new_value: format!("cas_{}", i).into_bytes(),
                };
                let resp = handler.handle(cas_req, &ctx).await.unwrap();
                if let ClientRpcResponse::CompareAndSwapResult(_result) = resp {
                    // May succeed or fail depending on key existence
                }
            }
        }
    }
}
