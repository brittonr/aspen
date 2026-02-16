//! Stress tests for the Coordination handler.
//!
//! These tests exercise the coordination handler with:
//! - Sequential operations at high volume
//! - Lock contention scenarios
//! - Counter contention scenarios
//! - Fencing token verification
//!
//! # Tiger Style
//!
//! - All tests use bounded inputs
//! - Deterministic via DeterministicKeyValueStore
//! - No network I/O
//! - Fixed iteration limits

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
// Lock Stress Tests
// =============================================================================

/// Test that multiple lock acquisitions on same key are correctly serialized.
/// This exercises the lock mutual exclusion invariant under contention.
#[tokio::test]
async fn test_stress_lock_contention() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let lock_key = "stress:lock:contention".to_string();
    let num_iterations = 50;

    for i in 0..num_iterations {
        let holder_id = format!("holder_{}", i);

        // Acquire lock
        let acquire_req = ClientRpcRequest::LockTryAcquire {
            key: lock_key.clone(),
            holder_id: holder_id.clone(),
            ttl_ms: 5000,
        };
        let acquire_resp = handler.handle(acquire_req, &ctx).await.unwrap();

        if let ClientRpcResponse::LockResult(result) = acquire_resp {
            assert!(result.is_success, "Lock should be acquired on iteration {}", i);
            let fencing_token = result.fencing_token.expect("Should have fencing token");

            // Verify fencing token is valid
            assert!(fencing_token > 0, "Fencing token should be > 0");

            // Release lock
            let release_req = ClientRpcRequest::LockRelease {
                key: lock_key.clone(),
                holder_id,
                fencing_token,
            };
            let release_resp = handler.handle(release_req, &ctx).await.unwrap();

            if let ClientRpcResponse::LockResult(result) = release_resp {
                assert!(result.is_success, "Lock should be released on iteration {}", i);
            }
        }
    }
}

/// Test fencing token monotonicity across multiple acquisitions.
#[tokio::test]
async fn test_stress_fencing_token_monotonicity() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let lock_key = "stress:lock:fencing".to_string();
    let num_iterations = 100;
    let mut last_token: u64 = 0;

    for i in 0..num_iterations {
        let holder_id = format!("holder_{}", i);

        // Acquire lock
        let acquire_req = ClientRpcRequest::LockTryAcquire {
            key: lock_key.clone(),
            holder_id: holder_id.clone(),
            ttl_ms: 5000,
        };
        let acquire_resp = handler.handle(acquire_req, &ctx).await.unwrap();

        if let ClientRpcResponse::LockResult(result) = acquire_resp {
            assert!(result.is_success);
            let current_token = result.fencing_token.expect("Should have fencing token");

            // Verify strict monotonicity
            assert!(
                current_token > last_token,
                "Fencing token {} should be > previous token {} on iteration {}",
                current_token,
                last_token,
                i
            );
            last_token = current_token;

            // Release lock
            let release_req = ClientRpcRequest::LockRelease {
                key: lock_key.clone(),
                holder_id,
                fencing_token: current_token,
            };
            handler.handle(release_req, &ctx).await.unwrap();
        }
    }
}

/// Test that expired locks can be acquired by new holders.
#[tokio::test]
async fn test_stress_lock_expiry_reacquisition() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let lock_key = "stress:lock:expiry".to_string();

    // Acquire lock with short TTL
    let acquire_req = ClientRpcRequest::LockTryAcquire {
        key: lock_key.clone(),
        holder_id: "holder_1".to_string(),
        ttl_ms: 100, // Very short TTL
    };
    let acquire_resp = handler.handle(acquire_req, &ctx).await.unwrap();

    if let ClientRpcResponse::LockResult(result) = acquire_resp {
        assert!(result.is_success);
    }

    // Wait for expiration
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Another holder should be able to acquire
    let acquire_req_2 = ClientRpcRequest::LockTryAcquire {
        key: lock_key.clone(),
        holder_id: "holder_2".to_string(),
        ttl_ms: 5000,
    };
    let acquire_resp_2 = handler.handle(acquire_req_2, &ctx).await.unwrap();

    if let ClientRpcResponse::LockResult(result) = acquire_resp_2 {
        assert!(result.is_success, "New holder should acquire lock after TTL expiry");
    }
}

// =============================================================================
// Counter Stress Tests
// =============================================================================

/// Test counter increment under high load.
#[tokio::test]
async fn test_stress_counter_increments() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let counter_key = "stress:counter:inc".to_string();
    let num_increments = 200;

    for i in 0..num_increments {
        let inc_req = ClientRpcRequest::CounterIncrement {
            key: counter_key.clone(),
        };
        let inc_resp = handler.handle(inc_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CounterResult(result) = inc_resp {
            assert!(result.is_success, "Increment {} should succeed", i);
            assert_eq!(result.value, Some((i + 1) as u64), "Counter value should be {} after increment {}", i + 1, i);
        }
    }

    // Verify final value via CounterGet
    let get_req = ClientRpcRequest::CounterGet { key: counter_key };
    let get_resp = handler.handle(get_req, &ctx).await.unwrap();

    if let ClientRpcResponse::CounterResult(result) = get_resp {
        assert!(result.is_success);
        assert_eq!(result.value, Some(num_increments as u64));
    }
}

/// Test counter decrement behavior.
///
/// Note: Unsigned counters decrement from a positive value toward 0.
/// The first 10 decrements should succeed. Further decrements may fail
/// due to saturation semantics (once at 0, decrementing returns an error).
#[tokio::test]
async fn test_stress_counter_decrements() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let counter_key = "stress:counter:dec".to_string();

    // First set counter to 10 via increments
    for _ in 0..10 {
        let inc_req = ClientRpcRequest::CounterIncrement {
            key: counter_key.clone(),
        };
        handler.handle(inc_req, &ctx).await.unwrap();
    }

    // Decrement 10 times (should reach 0)
    for i in 0..10 {
        let dec_req = ClientRpcRequest::CounterDecrement {
            key: counter_key.clone(),
        };
        let dec_resp = handler.handle(dec_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CounterResult(result) = dec_resp {
            assert!(result.is_success, "Decrement {} should succeed", i);
            let expected = 10 - (i as u64 + 1);
            assert_eq!(result.value, Some(expected), "Counter should be {} after decrement {}", expected, i);
        }
    }

    // Verify counter is at 0
    let get_req = ClientRpcRequest::CounterGet { key: counter_key };
    let get_resp = handler.handle(get_req, &ctx).await.unwrap();

    if let ClientRpcResponse::CounterResult(result) = get_resp {
        assert!(result.is_success);
        assert_eq!(result.value, Some(0));
    }
}

/// Test signed counter positive and negative values.
#[tokio::test]
async fn test_stress_signed_counter_operations() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let counter_key = "stress:signed:ops".to_string();

    // Start from a positive value using large increment
    let add_req = ClientRpcRequest::SignedCounterAdd {
        key: counter_key.clone(),
        amount: 100,
    };
    let add_resp = handler.handle(add_req, &ctx).await.unwrap();

    if let ClientRpcResponse::SignedCounterResult(result) = add_resp {
        assert!(result.is_success, "Initial add should succeed");
        assert_eq!(result.value, Some(100));
    }

    // Go directly to negative with a large negative delta
    let sub_req = ClientRpcRequest::SignedCounterAdd {
        key: counter_key.clone(),
        amount: -150, // 100 - 150 = -50
    };
    let sub_resp = handler.handle(sub_req, &ctx).await.unwrap();

    if let ClientRpcResponse::SignedCounterResult(result) = sub_resp {
        assert!(result.is_success, "Subtract should succeed");
        assert_eq!(result.value, Some(-50), "Value should be -50");
    }

    // Add back to positive
    let add_req2 = ClientRpcRequest::SignedCounterAdd {
        key: counter_key.clone(),
        amount: 75, // -50 + 75 = 25
    };
    let add_resp2 = handler.handle(add_req2, &ctx).await.unwrap();

    if let ClientRpcResponse::SignedCounterResult(result) = add_resp2 {
        assert!(result.is_success, "Add back should succeed");
        assert_eq!(result.value, Some(25), "Value should be 25");
    }
}

/// Test counter add with various amounts.
#[tokio::test]
async fn test_stress_counter_add_variations() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let counter_key = "stress:counter:add".to_string();
    let mut expected: u64 = 0;

    // Add various positive values (unsigned counter only allows positive adds)
    let amounts: Vec<u64> = vec![1, 5, 10, 100, 42];

    for amount in amounts {
        let add_req = ClientRpcRequest::CounterAdd {
            key: counter_key.clone(),
            amount,
        };
        let add_resp = handler.handle(add_req, &ctx).await.unwrap();

        expected += amount;

        if let ClientRpcResponse::CounterResult(result) = add_resp {
            assert!(result.is_success);
            assert_eq!(result.value, Some(expected));
        }
    }
}

// =============================================================================
// Multiple Keys Stress Tests
// =============================================================================

/// Test operations across many different keys.
#[tokio::test]
async fn test_stress_many_keys() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let num_keys = 50;

    // Create many counters (start from 1 to avoid edge cases with 0)
    for i in 0..num_keys {
        let key = format!("stress:many:counter:{}", i);
        let set_req = ClientRpcRequest::CounterSet {
            key: key.clone(),
            value: (i + 1) as u64, // Start from 1
        };
        handler.handle(set_req, &ctx).await.unwrap();
    }

    // Read all counters and verify values
    for i in 0..num_keys {
        let key = format!("stress:many:counter:{}", i);
        let get_req = ClientRpcRequest::CounterGet { key };
        let get_resp = handler.handle(get_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CounterResult(result) = get_resp {
            assert!(result.is_success, "Counter {} should exist", i);
            assert_eq!(result.value, Some((i + 1) as u64), "Counter {} should have value {}", i, i + 1);
        }
    }

    // Update all counters to double their value
    for i in 0..num_keys {
        let key = format!("stress:many:counter:{}", i);
        let set_req = ClientRpcRequest::CounterSet {
            key: key.clone(),
            value: ((i + 1) * 2) as u64,
        };
        let set_resp = handler.handle(set_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CounterResult(result) = set_resp {
            assert!(result.is_success, "Counter {} should be updated", i);
        }
    }

    // Verify all updated
    for i in 0..num_keys {
        let key = format!("stress:many:counter:{}", i);
        let get_req = ClientRpcRequest::CounterGet { key };
        let get_resp = handler.handle(get_req, &ctx).await.unwrap();

        if let ClientRpcResponse::CounterResult(result) = get_resp {
            assert!(result.is_success);
            assert_eq!(result.value, Some(((i + 1) * 2) as u64), "Counter {} should be doubled", i);
        }
    }
}

// =============================================================================
// Rate Limiter Stress Tests
// =============================================================================

/// Test rate limiter under sustained load.
#[tokio::test]
async fn test_stress_rate_limiter() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let limiter_key = "stress:ratelimiter:test".to_string();

    // Try to acquire tokens - initially should work until depleted
    let mut allowed_count = 0;
    for _ in 0..20 {
        let acquire_req = ClientRpcRequest::RateLimiterTryAcquire {
            key: limiter_key.clone(),
            tokens: 1,
            capacity: 10,
            refill_rate: 1.0,
        };
        let acquire_resp = handler.handle(acquire_req, &ctx).await.unwrap();

        if let ClientRpcResponse::RateLimiterResult(result) = acquire_resp {
            if result.is_success {
                allowed_count += 1;
            }
        }
    }

    // Should have allowed some tokens (up to initial capacity)
    assert!(allowed_count > 0, "Rate limiter should allow some tokens initially");
    assert!(allowed_count <= 10, "Rate limiter should not exceed capacity");

    // Check available permits
    let status_req = ClientRpcRequest::RateLimiterAvailable {
        key: limiter_key.clone(),
        capacity: 10,
        refill_rate: 1.0,
    };
    let status_resp = handler.handle(status_req, &ctx).await.unwrap();

    if let ClientRpcResponse::RateLimiterResult(result) = status_resp {
        // After consuming tokens, tokens_remaining should be low
        if let Some(remaining) = result.tokens_remaining {
            assert!(remaining <= 10, "Available tokens should be bounded by capacity");
        }
    }
}

// =============================================================================
// Lock Release Edge Cases
// =============================================================================

/// Test that releasing with wrong fencing token fails.
#[tokio::test]
async fn test_stress_lock_wrong_token_release() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let lock_key = "stress:lock:wrongtoken".to_string();
    let holder_id = "holder".to_string();

    // Acquire lock
    let acquire_req = ClientRpcRequest::LockTryAcquire {
        key: lock_key.clone(),
        holder_id: holder_id.clone(),
        ttl_ms: 5000,
    };
    let acquire_resp = handler.handle(acquire_req, &ctx).await.unwrap();

    if let ClientRpcResponse::LockResult(result) = acquire_resp {
        assert!(result.is_success);
        let correct_token = result.fencing_token.expect("Should have token");

        // Try to release with wrong token
        let wrong_release = ClientRpcRequest::LockRelease {
            key: lock_key.clone(),
            holder_id: holder_id.clone(),
            fencing_token: correct_token + 100, // Wrong token
        };
        let wrong_resp = handler.handle(wrong_release, &ctx).await.unwrap();

        if let ClientRpcResponse::LockResult(result) = wrong_resp {
            assert!(!result.is_success, "Wrong token should not release lock");
        }

        // Lock should still be held
        let check_req = ClientRpcRequest::LockTryAcquire {
            key: lock_key.clone(),
            holder_id: "other_holder".to_string(),
            ttl_ms: 5000,
        };
        let check_resp = handler.handle(check_req, &ctx).await.unwrap();

        if let ClientRpcResponse::LockResult(result) = check_resp {
            assert!(!result.is_success, "Lock should still be held after failed release");
        }
    }
}

/// Test that releasing with wrong holder fails.
#[tokio::test]
async fn test_stress_lock_wrong_holder_release() {
    let (ctx, _kv) = test_context().await;
    let handler = CoordinationHandler;

    let lock_key = "stress:lock:wrongholder".to_string();
    let holder_id = "holder".to_string();

    // Acquire lock
    let acquire_req = ClientRpcRequest::LockTryAcquire {
        key: lock_key.clone(),
        holder_id: holder_id.clone(),
        ttl_ms: 5000,
    };
    let acquire_resp = handler.handle(acquire_req, &ctx).await.unwrap();

    if let ClientRpcResponse::LockResult(result) = acquire_resp {
        assert!(result.is_success);
        let token = result.fencing_token.expect("Should have token");

        // Try to release with wrong holder
        let wrong_release = ClientRpcRequest::LockRelease {
            key: lock_key.clone(),
            holder_id: "wrong_holder".to_string(),
            fencing_token: token,
        };
        let wrong_resp = handler.handle(wrong_release, &ctx).await.unwrap();

        if let ClientRpcResponse::LockResult(result) = wrong_resp {
            assert!(!result.is_success, "Wrong holder should not release lock");
        }
    }
}
