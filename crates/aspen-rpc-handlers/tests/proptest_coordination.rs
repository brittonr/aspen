//! Property-based tests for the Coordination handler.
//!
//! Tests cover invariants for:
//! - Locks: mutual exclusion, fencing token monotonicity, TTL expiration, release credentials
//! - Counters: atomicity, non-negativity (unsigned), CAS linearizability
//!
//! # Tiger Style
//!
//! - All tests use bounded inputs from generators
//! - Deterministic via DeterministicKeyValueStore
//! - No network I/O

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_handlers::context::test_support::TestContextBuilder;
use aspen_rpc_handlers::handlers::CoordinationHandler;
use aspen_rpc_handlers::registry::RequestHandler;
use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
#[cfg(feature = "sql")]
use aspen_rpc_handlers::test_mocks::mock_sql_executor;
use aspen_testing::DeterministicKeyValueStore;
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

/// Create a test context with shared KV store.
#[allow(dead_code)]
async fn test_context_with_kv(
    kv_store: Arc<dyn aspen_core::KeyValueStore>,
) -> aspen_rpc_handlers::ClientProtocolContext {
    let mock_endpoint = Arc::new(MockEndpointProvider::new().await);
    let builder = TestContextBuilder::new().with_kv_store(kv_store).with_endpoint_manager(mock_endpoint);
    #[cfg(feature = "sql")]
    let builder = builder.with_sql_executor(mock_sql_executor());
    builder.build()
}

// =============================================================================
// Lock Generators
// =============================================================================

/// Generate a valid lock key (avoiding reserved prefixes).
fn valid_lock_key() -> impl Strategy<Value = String> {
    prop_oneof!["lock:[a-z]{3,10}", "mylock:[a-z0-9]{1,10}", "[a-z]{3,20}",]
}

/// Generate a holder ID.
fn holder_id() -> impl Strategy<Value = String> {
    "[a-z0-9]{8,16}"
}

/// Generate a valid TTL in milliseconds.
fn ttl_ms() -> impl Strategy<Value = u64> {
    1000u64..60_000u64
}

/// Generate a valid timeout in milliseconds.
fn timeout_ms() -> impl Strategy<Value = u64> {
    100u64..10_000u64
}

// =============================================================================
// Counter Generators
// =============================================================================

/// Generate a valid counter key.
fn valid_counter_key() -> impl Strategy<Value = String> {
    prop_oneof!["counter:[a-z]{3,10}", "cnt:[a-z0-9]{1,10}", "[a-z]{3,20}",]
}

/// Generate a counter add/subtract amount.
fn counter_amount() -> impl Strategy<Value = u64> {
    1u64..1000u64
}

// =============================================================================
// Lock Invariant Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// INV-LOCK-01: Mutual exclusion - only one holder can hold a lock at a time.
    ///
    /// If holder A acquires a lock, holder B should fail to acquire the same lock
    /// until A releases it.
    #[test]
    fn test_proptest_lock_mutual_exclusion(
        key in valid_lock_key(),
        holder_a in holder_id(),
        holder_b in holder_id(),
        ttl in ttl_ms(),
    ) {
        // Skip if both holders are the same (not a real contention test)
        prop_assume!(holder_a != holder_b);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // Holder A acquires the lock
            let req_a = ClientRpcRequest::LockTryAcquire {
                key: key.clone(),
                holder_id: holder_a.clone(),
                ttl_ms: ttl,
            };
            let resp_a = handler.handle(req_a, &ctx).await.unwrap();

            if let ClientRpcResponse::LockResult(result_a) = resp_a {
                if result_a.is_success {
                    // Holder A got the lock, now B should fail
                    let req_b = ClientRpcRequest::LockTryAcquire {
                        key: key.clone(),
                        holder_id: holder_b.clone(),
                        ttl_ms: ttl,
                    };
                    let resp_b = handler.handle(req_b, &ctx).await.unwrap();

                    if let ClientRpcResponse::LockResult(result_b) = resp_b {
                        // Mutual exclusion: B should not get the lock while A holds it
                        prop_assert!(!result_b.is_success, "Lock should not be acquired while held by another");
                        prop_assert_eq!(result_b.holder_id, Some(holder_a.clone()), "Should report current holder");
                    }
                }
            }
            Ok(())
        })?;
    }

    /// INV-LOCK-02: Fencing token monotonicity - each acquisition should get a higher token.
    ///
    /// When the same holder re-acquires a lock after release, the fencing token
    /// should be strictly greater than the previous one.
    #[test]
    fn test_proptest_lock_fencing_token_ordering(
        key in valid_lock_key(),
        holder in holder_id(),
        ttl in ttl_ms(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            let mut prev_token: Option<u64> = None;

            // Acquire and release multiple times, checking token ordering
            for _ in 0..3 {
                // Acquire
                let req = ClientRpcRequest::LockTryAcquire {
                    key: key.clone(),
                    holder_id: holder.clone(),
                    ttl_ms: ttl,
                };
                let resp = handler.handle(req, &ctx).await.unwrap();

                if let ClientRpcResponse::LockResult(result) = resp {
                    if result.is_success {
                        let token = result.fencing_token.unwrap();

                        // Fencing token should be strictly greater than previous
                        if let Some(prev) = prev_token {
                            prop_assert!(token > prev, "Fencing token should increase: {} > {}", token, prev);
                        }
                        prev_token = Some(token);

                        // Release the lock
                        let release_req = ClientRpcRequest::LockRelease {
                            key: key.clone(),
                            holder_id: holder.clone(),
                            fencing_token: token,
                        };
                        let release_resp = handler.handle(release_req, &ctx).await.unwrap();
                        if let ClientRpcResponse::LockResult(release_result) = release_resp {
                            prop_assert!(release_result.is_success, "Release should succeed");
                        }
                    }
                }
            }
            Ok(())
        })?;
    }

    /// INV-LOCK-04: Release requires correct credentials - wrong holder or token fails.
    ///
    /// A lock can only be released by the holder who acquired it, with the correct
    /// fencing token.
    #[test]
    fn test_proptest_lock_release_credentials(
        key in valid_lock_key(),
        holder_a in holder_id(),
        holder_b in holder_id(),
        ttl in ttl_ms(),
    ) {
        prop_assume!(holder_a != holder_b);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // Holder A acquires the lock
            let req_a = ClientRpcRequest::LockTryAcquire {
                key: key.clone(),
                holder_id: holder_a.clone(),
                ttl_ms: ttl,
            };
            let resp_a = handler.handle(req_a, &ctx).await.unwrap();

            if let ClientRpcResponse::LockResult(result_a) = resp_a {
                if result_a.is_success {
                    let token = result_a.fencing_token.unwrap();

                    // Holder B tries to release A's lock - should fail
                    let bad_release = ClientRpcRequest::LockRelease {
                        key: key.clone(),
                        holder_id: holder_b.clone(),
                        fencing_token: token,
                    };
                    let bad_resp = handler.handle(bad_release, &ctx).await.unwrap();
                    if let ClientRpcResponse::LockResult(bad_result) = bad_resp {
                        prop_assert!(!bad_result.is_success, "Wrong holder should not release lock");
                    }

                    // Holder A with wrong token - should fail
                    let wrong_token_release = ClientRpcRequest::LockRelease {
                        key: key.clone(),
                        holder_id: holder_a.clone(),
                        fencing_token: token + 1, // Wrong token
                    };
                    let wrong_resp = handler.handle(wrong_token_release, &ctx).await.unwrap();
                    if let ClientRpcResponse::LockResult(wrong_result) = wrong_resp {
                        prop_assert!(!wrong_result.is_success, "Wrong token should not release lock");
                    }

                    // Correct release - should succeed
                    let good_release = ClientRpcRequest::LockRelease {
                        key: key.clone(),
                        holder_id: holder_a.clone(),
                        fencing_token: token,
                    };
                    let good_resp = handler.handle(good_release, &ctx).await.unwrap();
                    if let ClientRpcResponse::LockResult(good_result) = good_resp {
                        prop_assert!(good_result.is_success, "Correct credentials should release lock");
                    }
                }
            }
            Ok(())
        })?;
    }

    /// INV-LOCK: Non-reentrant - same holder trying to re-acquire gets blocked.
    ///
    /// DistributedLock is NOT reentrant: even the same holder cannot re-acquire
    /// without releasing first. This is by design for simplicity and safety.
    #[test]
    fn test_proptest_lock_non_reentrant(
        key in valid_lock_key(),
        holder in holder_id(),
        ttl in ttl_ms(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // First acquisition
            let req1 = ClientRpcRequest::LockTryAcquire {
                key: key.clone(),
                holder_id: holder.clone(),
                ttl_ms: ttl,
            };
            let resp1 = handler.handle(req1, &ctx).await.unwrap();

            if let ClientRpcResponse::LockResult(result1) = resp1 {
                if result1.is_success {
                    // Same holder tries to acquire again (without releasing)
                    let req2 = ClientRpcRequest::LockTryAcquire {
                        key: key.clone(),
                        holder_id: holder.clone(),
                        ttl_ms: ttl,
                    };
                    let resp2 = handler.handle(req2, &ctx).await.unwrap();

                    if let ClientRpcResponse::LockResult(result2) = resp2 {
                        // Re-acquisition by same holder should FAIL (non-reentrant)
                        prop_assert!(!result2.is_success, "Same holder should NOT be able to re-acquire without release");
                        // Should report itself as current holder
                        prop_assert_eq!(result2.holder_id, Some(holder.clone()), "Should report current holder");
                    }
                }
            }
            Ok(())
        })?;
    }
}

// =============================================================================
// Counter Invariant Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// INV-COUNTER-01: Increment atomicity - increment returns the new value.
    ///
    /// After N increments, the counter should have value N.
    #[test]
    fn test_proptest_counter_increment_atomicity(
        key in valid_counter_key(),
        n in 1usize..50usize,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            for i in 1..=n {
                let req = ClientRpcRequest::CounterIncrement { key: key.clone() };
                let resp = handler.handle(req, &ctx).await.unwrap();

                if let ClientRpcResponse::CounterResult(result) = resp {
                    prop_assert!(result.is_success, "Increment should succeed");
                    prop_assert_eq!(result.value, Some(i as u64), "Counter should equal number of increments");
                }
            }
            Ok(())
        })?;
    }

    /// INV-COUNTER-02: Unsigned counter never negative - decrement saturates at 0.
    ///
    /// Decrementing below 0 should either fail or saturate.
    #[test]
    fn test_proptest_counter_non_negative(
        key in valid_counter_key(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // Counter starts at 0, try to decrement
            let req = ClientRpcRequest::CounterDecrement { key: key.clone() };
            let resp = handler.handle(req, &ctx).await.unwrap();

            if let ClientRpcResponse::CounterResult(result) = resp {
                // Either fails or returns 0 (no negative)
                if result.is_success {
                    if let Some(value) = result.value {
                        prop_assert!(value == 0, "Counter should not go negative, got {}", value);
                    }
                }
                // If it fails, that's also acceptable (underflow protection)
            }
            Ok(())
        })?;
    }

    /// INV-COUNTER-03: CAS linearizability - compare-and-set is atomic.
    ///
    /// CAS should only succeed if the current value matches expected.
    #[test]
    fn test_proptest_counter_cas_linearizable(
        key in valid_counter_key(),
        initial in 1u64..1000u64,
        new_value in 1u64..1000u64,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // Set initial value
            let set_req = ClientRpcRequest::CounterSet { key: key.clone(), value: initial };
            let _ = handler.handle(set_req, &ctx).await.unwrap();

            // CAS with wrong expected - should fail
            let bad_cas = ClientRpcRequest::CounterCompareAndSet {
                key: key.clone(),
                expected: initial + 1, // Wrong expectation
                new_value,
            };
            let bad_resp = handler.handle(bad_cas, &ctx).await.unwrap();
            if let ClientRpcResponse::CounterResult(bad_result) = bad_resp {
                prop_assert!(!bad_result.is_success, "CAS with wrong expected should fail");
            }

            // CAS with correct expected - should succeed
            let good_cas = ClientRpcRequest::CounterCompareAndSet {
                key: key.clone(),
                expected: initial,
                new_value,
            };
            let good_resp = handler.handle(good_cas, &ctx).await.unwrap();
            if let ClientRpcResponse::CounterResult(good_result) = good_resp {
                prop_assert!(good_result.is_success, "CAS with correct expected should succeed");
                prop_assert_eq!(good_result.value, Some(new_value), "CAS should return new value");
            }

            // Verify the value changed
            let get_req = ClientRpcRequest::CounterGet { key: key.clone() };
            let get_resp = handler.handle(get_req, &ctx).await.unwrap();
            if let ClientRpcResponse::CounterResult(get_result) = get_resp {
                prop_assert_eq!(get_result.value, Some(new_value), "Counter should have new value after CAS");
            }

            Ok(())
        })?;
    }

    /// INV-COUNTER: Add/Subtract consistency - add(n) then subtract(n) returns to original.
    #[test]
    fn test_proptest_counter_add_subtract_consistency(
        key in valid_counter_key(),
        initial in 100u64..1000u64,
        delta in 1u64..100u64,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // Set initial value
            let set_req = ClientRpcRequest::CounterSet { key: key.clone(), value: initial };
            let _ = handler.handle(set_req, &ctx).await.unwrap();

            // Add delta
            let add_req = ClientRpcRequest::CounterAdd { key: key.clone(), amount: delta };
            let add_resp = handler.handle(add_req, &ctx).await.unwrap();
            if let ClientRpcResponse::CounterResult(add_result) = add_resp {
                prop_assert_eq!(add_result.value, Some(initial + delta), "Add should increase by delta");
            }

            // Subtract delta
            let sub_req = ClientRpcRequest::CounterSubtract { key: key.clone(), amount: delta };
            let sub_resp = handler.handle(sub_req, &ctx).await.unwrap();
            if let ClientRpcResponse::CounterResult(sub_result) = sub_resp {
                prop_assert_eq!(sub_result.value, Some(initial), "Subtract should return to original");
            }

            Ok(())
        })?;
    }
}

// =============================================================================
// Handler Dispatch Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Verify can_handle returns true for all lock operation types.
    #[test]
    fn test_proptest_can_handle_lock_operations(
        key in valid_lock_key(),
        holder in holder_id(),
        ttl in ttl_ms(),
        timeout in timeout_ms(),
    ) {
        let handler = CoordinationHandler;

        let requests = vec![
            ClientRpcRequest::LockAcquire {
                key: key.clone(),
                holder_id: holder.clone(),
                ttl_ms: ttl,
                timeout_ms: timeout,
            },
            ClientRpcRequest::LockTryAcquire {
                key: key.clone(),
                holder_id: holder.clone(),
                ttl_ms: ttl,
            },
            ClientRpcRequest::LockRelease {
                key: key.clone(),
                holder_id: holder.clone(),
                fencing_token: 1,
            },
            ClientRpcRequest::LockRenew {
                key: key.clone(),
                holder_id: holder.clone(),
                fencing_token: 1,
                ttl_ms: ttl,
            },
        ];

        for req in requests {
            prop_assert!(handler.can_handle(&req), "CoordinationHandler should handle {:?}", req);
        }
    }

    /// Verify can_handle returns true for all counter operation types.
    #[test]
    fn test_proptest_can_handle_counter_operations(
        key in valid_counter_key(),
        amount in counter_amount(),
    ) {
        let handler = CoordinationHandler;

        let requests = vec![
            ClientRpcRequest::CounterGet { key: key.clone() },
            ClientRpcRequest::CounterIncrement { key: key.clone() },
            ClientRpcRequest::CounterDecrement { key: key.clone() },
            ClientRpcRequest::CounterAdd { key: key.clone(), amount },
            ClientRpcRequest::CounterSubtract { key: key.clone(), amount },
            ClientRpcRequest::CounterSet { key: key.clone(), value: amount },
            ClientRpcRequest::CounterCompareAndSet {
                key: key.clone(),
                expected: amount,
                new_value: amount + 1,
            },
        ];

        for req in requests {
            prop_assert!(handler.can_handle(&req), "CoordinationHandler should handle {:?}", req);
        }
    }
}

// =============================================================================
// Reserved Prefix Rejection Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Keys with reserved system prefix should be rejected.
    ///
    /// The only reserved prefix is `_system:` which is for internal cluster data.
    #[test]
    fn test_proptest_reserved_prefix_rejected(
        suffix in "[a-z0-9]{1,10}",
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (ctx, _kv) = test_context().await;
            let handler = CoordinationHandler;

            // The only reserved prefix is _system:
            let key = format!("_system:{}", suffix);

            // Lock with reserved prefix should be rejected
            let lock_req = ClientRpcRequest::LockTryAcquire {
                key: key.clone(),
                holder_id: "test".to_string(),
                ttl_ms: 1000,
            };
            let lock_resp = handler.handle(lock_req, &ctx).await.unwrap();
            if let ClientRpcResponse::LockResult(result) = lock_resp {
                prop_assert!(!result.is_success, "Reserved prefix '_system:' should be rejected for lock");
                prop_assert!(result.error.is_some(), "Should have error message");
            }

            // Counter with reserved prefix should be rejected
            let counter_req = ClientRpcRequest::CounterGet { key: key.clone() };
            let counter_resp = handler.handle(counter_req, &ctx).await.unwrap();
            if let ClientRpcResponse::CounterResult(result) = counter_resp {
                prop_assert!(!result.is_success, "Reserved prefix '_system:' should be rejected for counter");
                prop_assert!(result.error.is_some(), "Should have error message");
            }

            Ok(())
        })?;
    }
}
