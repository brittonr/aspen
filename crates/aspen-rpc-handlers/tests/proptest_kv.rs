//! Property-based tests for the KV handler.
//!
//! Tests cover invariants for:
//! - Read-after-write consistency
//! - CAS atomicity
//! - Batch write atomicity
//! - Scan pagination completeness
//! - Delete idempotency
//! - Reserved prefix rejection
//!
//! # Tiger Style
//!
//! - All tests use bounded inputs from generators
//! - Deterministic via DeterministicKeyValueStore
//! - No network I/O

use std::sync::Arc;

use aspen_client_api::BatchWriteOperation;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::DeterministicKeyValueStore;
use aspen_rpc_handlers::context::test_support::TestContextBuilder;
use aspen_rpc_handlers::handlers::KvHandler;
use aspen_rpc_handlers::registry::RequestHandler;
use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
#[cfg(feature = "sql")]
use aspen_rpc_handlers::test_mocks::mock_sql_executor;
use proptest::prelude::*;

// =============================================================================
// Test Setup Helpers
// =============================================================================

/// Create a test context with fresh KV store.
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
// KV Generators
// =============================================================================

/// Generate a valid KV key (avoiding reserved prefixes).
fn valid_kv_key() -> impl Strategy<Value = String> {
    prop_oneof![
        "app:[a-z0-9]{1,20}",
        "data:[a-z0-9]{1,20}",
        "[a-z]{3,30}",
        "key_[0-9]{1,5}",
    ]
}

/// Generate a valid KV value.
/// Note: The KV handler uses String::from_utf8_lossy, so we only generate valid UTF-8 values
/// to avoid lossy conversion artifacts in round-trip testing.
fn kv_value() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Small string values
        "[a-zA-Z0-9 ]{1,100}".prop_map(|s| s.into_bytes()),
        // Medium string values
        "[a-zA-Z0-9 ]{100,500}".prop_map(|s| s.into_bytes()),
        // Numeric values
        "[0-9]{1,50}".prop_map(|s| s.into_bytes()),
        // JSON-like values
        "\\{\"key\": \"[a-z]{1,20}\"\\}".prop_map(|s| s.into_bytes()),
    ]
}

// =============================================================================
// Read-Write Invariant Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// INV-KV-01: Read-after-write consistency.
    ///
    /// A key that was just written should be readable with the same value.
    #[test]
    fn test_proptest_kv_read_after_write(
        key in valid_kv_key(),
        value in kv_value(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Write the key
            let write_req = ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: value.clone(),
            };
            let write_resp = handler.handle(write_req, &ctx).await.unwrap();

            if let ClientRpcResponse::WriteResult(result) = write_resp {
                prop_assert!(result.success, "Write should succeed");
            }

            // Read the key back
            let read_req = ClientRpcRequest::ReadKey { key: key.clone() };
            let read_resp = handler.handle(read_req, &ctx).await.unwrap();

            if let ClientRpcResponse::ReadResult(result) = read_resp {
                prop_assert!(result.found, "Key should be found after write");
                prop_assert_eq!(result.value, Some(value), "Value should match what was written");
            }

            Ok(())
        })?;
    }

    /// INV-KV-05: Delete idempotency.
    ///
    /// Deleting a key should succeed, and deleting it again should also succeed.
    #[test]
    fn test_proptest_kv_delete_idempotent(
        key in valid_kv_key(),
        value in kv_value(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Write the key first
            let write_req = ClientRpcRequest::WriteKey {
                key: key.clone(),
                value,
            };
            let _ = handler.handle(write_req, &ctx).await.unwrap();

            // First delete
            let delete_req = ClientRpcRequest::DeleteKey { key: key.clone() };
            let delete_resp1 = handler.handle(delete_req, &ctx).await.unwrap();

            if let ClientRpcResponse::DeleteResult(result1) = delete_resp1 {
                prop_assert!(result1.deleted, "First delete should succeed");
            }

            // Second delete (idempotent)
            let delete_req2 = ClientRpcRequest::DeleteKey { key: key.clone() };
            let delete_resp2 = handler.handle(delete_req2, &ctx).await.unwrap();

            if let ClientRpcResponse::DeleteResult(result2) = delete_resp2 {
                // Delete of non-existent key should also succeed (idempotent)
                prop_assert!(result2.deleted, "Second delete should also succeed (idempotent)");
            }

            // Verify key is gone
            let read_req = ClientRpcRequest::ReadKey { key };
            let read_resp = handler.handle(read_req, &ctx).await.unwrap();

            if let ClientRpcResponse::ReadResult(result) = read_resp {
                prop_assert!(!result.found, "Key should not exist after delete");
            }

            Ok(())
        })?;
    }

    /// INV-KV-02: CAS atomicity.
    ///
    /// Compare-and-swap should only succeed if current value matches expected.
    #[test]
    fn test_proptest_kv_cas_atomicity(
        key in valid_kv_key(),
        initial_value in kv_value(),
        new_value in kv_value(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Write initial value
            let write_req = ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: initial_value.clone(),
            };
            let _ = handler.handle(write_req, &ctx).await.unwrap();

            // CAS with wrong expected value - should fail
            let wrong_expected = vec![0u8; initial_value.len() + 1]; // Different value
            let bad_cas = ClientRpcRequest::CompareAndSwapKey {
                key: key.clone(),
                expected: Some(wrong_expected),
                new_value: new_value.clone(),
            };
            let bad_resp = handler.handle(bad_cas, &ctx).await.unwrap();

            if let ClientRpcResponse::CompareAndSwapResult(bad_result) = bad_resp {
                prop_assert!(!bad_result.success, "CAS with wrong expected should fail");
            }

            // CAS with correct expected value - should succeed
            let good_cas = ClientRpcRequest::CompareAndSwapKey {
                key: key.clone(),
                expected: Some(initial_value),
                new_value: new_value.clone(),
            };
            let good_resp = handler.handle(good_cas, &ctx).await.unwrap();

            if let ClientRpcResponse::CompareAndSwapResult(good_result) = good_resp {
                prop_assert!(good_result.success, "CAS with correct expected should succeed");
            }

            // Verify value changed
            let read_req = ClientRpcRequest::ReadKey { key };
            let read_resp = handler.handle(read_req, &ctx).await.unwrap();

            if let ClientRpcResponse::ReadResult(result) = read_resp {
                prop_assert_eq!(result.value, Some(new_value), "Value should be updated after CAS");
            }

            Ok(())
        })?;
    }

    /// INV-KV: CAS for create (expected = None).
    ///
    /// CAS with expected=None should only succeed if key doesn't exist.
    #[test]
    fn test_proptest_kv_cas_create(
        key in valid_kv_key(),
        value in kv_value(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // CAS create on non-existent key - should succeed
            let create_cas = ClientRpcRequest::CompareAndSwapKey {
                key: key.clone(),
                expected: None,
                new_value: value.clone(),
            };
            let create_resp = handler.handle(create_cas, &ctx).await.unwrap();

            if let ClientRpcResponse::CompareAndSwapResult(result) = create_resp {
                prop_assert!(result.success, "CAS create on new key should succeed");
            }

            // CAS create again - should fail (key now exists)
            let create_cas2 = ClientRpcRequest::CompareAndSwapKey {
                key: key.clone(),
                expected: None,
                new_value: value.clone(),
            };
            let create_resp2 = handler.handle(create_cas2, &ctx).await.unwrap();

            if let ClientRpcResponse::CompareAndSwapResult(result2) = create_resp2 {
                prop_assert!(!result2.success, "CAS create on existing key should fail");
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Batch Operation Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// INV-KV-03: Batch write atomicity.
    ///
    /// All operations in a batch should be applied together.
    #[test]
    fn test_proptest_kv_batch_atomicity(
        keys in prop::collection::vec(valid_kv_key(), 1..10),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Create batch operations (unique keys)
            let unique_keys: Vec<String> = keys.into_iter()
                .enumerate()
                .map(|(i, k)| format!("{}_{}", k, i))
                .collect();

            let operations: Vec<BatchWriteOperation> = unique_keys.iter()
                .map(|k| BatchWriteOperation::Set {
                    key: k.clone(),
                    value: format!("value_for_{}", k).into_bytes(),
                })
                .collect();

            // Execute batch write
            let batch_req = ClientRpcRequest::BatchWrite {
                operations: operations.clone(),
            };
            let batch_resp = handler.handle(batch_req, &ctx).await.unwrap();

            if let ClientRpcResponse::BatchWriteResult(result) = batch_resp {
                prop_assert!(result.success, "Batch write should succeed");
                prop_assert_eq!(
                    result.operations_applied,
                    Some(operations.len() as u32),
                    "All operations should be applied"
                );
            }

            // Verify all keys were written
            for key in &unique_keys {
                let read_req = ClientRpcRequest::ReadKey { key: key.clone() };
                let read_resp = handler.handle(read_req, &ctx).await.unwrap();

                if let ClientRpcResponse::ReadResult(result) = read_resp {
                    prop_assert!(result.found, "Key '{}' should exist after batch", key);
                    let expected_value = format!("value_for_{}", key).into_bytes();
                    prop_assert_eq!(result.value, Some(expected_value), "Value should match for key '{}'", key);
                }
            }

            Ok(())
        })?;
    }

    /// INV-KV: Batch read returns values in order.
    #[test]
    fn test_proptest_kv_batch_read_ordering(
        n in 1usize..10usize,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Write keys with ordered values
            let keys: Vec<String> = (0..n).map(|i| format!("key_{}", i)).collect();
            for (i, key) in keys.iter().enumerate() {
                let write_req = ClientRpcRequest::WriteKey {
                    key: key.clone(),
                    value: format!("value_{}", i).into_bytes(),
                };
                let _ = handler.handle(write_req, &ctx).await.unwrap();
            }

            // Batch read
            let batch_req = ClientRpcRequest::BatchRead { keys: keys.clone() };
            let batch_resp = handler.handle(batch_req, &ctx).await.unwrap();

            if let ClientRpcResponse::BatchReadResult(result) = batch_resp {
                prop_assert!(result.success, "Batch read should succeed");
                let values = result.values.unwrap();
                prop_assert_eq!(values.len(), n, "Should return all values");

                // Verify ordering
                for (i, value) in values.iter().enumerate() {
                    let expected = format!("value_{}", i).into_bytes();
                    prop_assert_eq!(value.as_ref(), Some(&expected), "Value at index {} should match", i);
                }
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Scan Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// INV-KV-04: Scan pagination completeness.
    ///
    /// Multiple scan calls with continuation tokens should return all matching keys.
    #[test]
    fn test_proptest_kv_scan_completeness(
        n in 5usize..20usize,
        page_size in 2u32..5u32,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Write keys with common prefix
            let prefix = "scan_test:";
            for i in 0..n {
                let write_req = ClientRpcRequest::WriteKey {
                    key: format!("{}{}", prefix, i),
                    value: format!("value_{}", i).into_bytes(),
                };
                let _ = handler.handle(write_req, &ctx).await.unwrap();
            }

            // Scan with pagination
            let mut all_keys: Vec<String> = Vec::new();
            let mut continuation_token: Option<String> = None;
            let mut iterations = 0;

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

                iterations += 1;
                if iterations > 100 {
                    panic!("Too many scan iterations");
                }
            }

            // Verify we got all keys
            prop_assert_eq!(all_keys.len(), n, "Should find all {} keys", n);

            Ok(())
        })?;
    }

    /// Scan with no matching keys returns empty.
    #[test]
    fn test_proptest_kv_scan_no_match(
        prefix in "[xyz]{5,10}:",  // Unlikely to match anything
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // Write some keys with different prefix
            for i in 0..5 {
                let write_req = ClientRpcRequest::WriteKey {
                    key: format!("other:{}", i),
                    value: b"value".to_vec(),
                };
                let _ = handler.handle(write_req, &ctx).await.unwrap();
            }

            // Scan with non-matching prefix
            let scan_req = ClientRpcRequest::ScanKeys {
                prefix,
                limit: Some(100),
                continuation_token: None,
            };
            let scan_resp = handler.handle(scan_req, &ctx).await.unwrap();

            if let ClientRpcResponse::ScanResult(result) = scan_resp {
                prop_assert!(result.entries.is_empty(), "Should find no matching keys");
                prop_assert_eq!(result.count, 0, "Count should be 0");
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Handler Dispatch Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Verify can_handle returns true for all KV operation types.
    #[test]
    fn test_proptest_can_handle_kv_operations(
        key in valid_kv_key(),
        value in kv_value(),
    ) {
        let handler = KvHandler;

        let requests = vec![
            ClientRpcRequest::ReadKey { key: key.clone() },
            ClientRpcRequest::WriteKey { key: key.clone(), value: value.clone() },
            ClientRpcRequest::DeleteKey { key: key.clone() },
            ClientRpcRequest::CompareAndSwapKey {
                key: key.clone(),
                expected: Some(value.clone()),
                new_value: value.clone(),
            },
            ClientRpcRequest::CompareAndDeleteKey {
                key: key.clone(),
                expected: value.clone(),
            },
            ClientRpcRequest::ScanKeys {
                prefix: "test:".to_string(),
                limit: Some(100),
                continuation_token: None,
            },
            ClientRpcRequest::BatchRead { keys: vec![key.clone()] },
            ClientRpcRequest::BatchWrite {
                operations: vec![BatchWriteOperation::Set { key: key.clone(), value }],
            },
        ];

        for req in requests {
            prop_assert!(handler.can_handle(&req), "KvHandler should handle {:?}", req);
        }
    }
}

// =============================================================================
// Reserved Prefix Rejection Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Keys with reserved system prefix should be rejected for writes.
    #[test]
    fn test_proptest_kv_reserved_prefix_rejected(
        suffix in "[a-z0-9]{1,10}",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = KvHandler;

            // The only reserved prefix is _system:
            let key = format!("_system:{}", suffix);

            // Write with reserved prefix should fail
            let write_req = ClientRpcRequest::WriteKey {
                key: key.clone(),
                value: b"test".to_vec(),
            };
            let write_resp = handler.handle(write_req, &ctx).await.unwrap();
            if let ClientRpcResponse::WriteResult(result) = write_resp {
                prop_assert!(!result.success, "Write with reserved prefix should fail");
                prop_assert!(result.error.is_some(), "Should have error message");
            }

            // Delete with reserved prefix should fail
            let delete_req = ClientRpcRequest::DeleteKey { key: key.clone() };
            let delete_resp = handler.handle(delete_req, &ctx).await.unwrap();
            if let ClientRpcResponse::DeleteResult(result) = delete_resp {
                prop_assert!(!result.deleted, "Delete with reserved prefix should fail");
                prop_assert!(result.error.is_some(), "Should have error message");
            }

            // CAS with reserved prefix should fail
            let cas_req = ClientRpcRequest::CompareAndSwapKey {
                key: key.clone(),
                expected: None,
                new_value: b"test".to_vec(),
            };
            let cas_resp = handler.handle(cas_req, &ctx).await.unwrap();
            if let ClientRpcResponse::CompareAndSwapResult(result) = cas_resp {
                prop_assert!(!result.success, "CAS with reserved prefix should fail");
                prop_assert!(result.error.is_some(), "Should have error message");
            }

            // Note: Read is allowed for system keys (for observability)

            Ok(())
        })?;
    }
}
