//! Unit tests for Coordination RPC handler.
//!
//! These tests verify individual handler operations with controlled inputs
//! and deterministic in-memory storage. Tests are organized by coordination
//! primitive category.
//!
//! # Test Coverage Goals
//!
//! - Success paths for each operation type
//! - Error cases (invalid keys, wrong holders, CAS failures)
//! - Edge cases (zero timeouts, empty operations)
//!
//! # Tiger Style
//!
//! - All operations use bounded test values
//! - No real I/O - uses DeterministicKeyValueStore
//! - Deterministic results for reproducibility

#![cfg(feature = "testing")]

use std::sync::Arc;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::DeterministicKeyValueStore;
use aspen_core::KeyValueStore;
use aspen_rpc_handlers::context::ClientProtocolContext;
use aspen_rpc_handlers::context::test_support::TestContextBuilder;
use aspen_rpc_handlers::handlers::coordination::CoordinationHandler;
use aspen_rpc_handlers::registry::RequestHandler;
use aspen_rpc_handlers::test_mocks::MockEndpointProvider;
#[cfg(feature = "sql")]
use aspen_rpc_handlers::test_mocks::mock_sql_executor;

/// Helper to create a test context with shared KV store.
async fn test_context() -> (ClientProtocolContext, Arc<dyn KeyValueStore>) {
    let kv_store: Arc<dyn KeyValueStore> = DeterministicKeyValueStore::new();
    let endpoint = Arc::new(MockEndpointProvider::new().await);
    let mut builder = TestContextBuilder::new().with_kv_store(Arc::clone(&kv_store)).with_endpoint_manager(endpoint);

    #[cfg(feature = "sql")]
    {
        builder = builder.with_sql_executor(mock_sql_executor());
    }

    let ctx = builder.build();
    (ctx, kv_store)
}

// =============================================================================
// can_handle() Tests
// =============================================================================

mod can_handle_tests {
    use super::*;

    #[test]
    fn test_can_handle_lock_acquire() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::LockAcquire {
            key: "lock:test".to_string(),
            holder_id: "h1".to_string(),
            ttl_ms: 5000,
            timeout_ms: 1000,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_lock_try_acquire() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::LockTryAcquire {
            key: "lock:test".to_string(),
            holder_id: "h1".to_string(),
            ttl_ms: 5000,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_lock_release() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::LockRelease {
            key: "lock:test".to_string(),
            holder_id: "h1".to_string(),
            fencing_token: 12345,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_counter_get() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::CounterGet {
            key: "counter:test".to_string(),
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_counter_increment() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::CounterIncrement {
            key: "counter:test".to_string(),
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_sequence_next() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::SequenceNext {
            key: "seq:test".to_string(),
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_rate_limiter_try_acquire() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::RateLimiterTryAcquire {
            key: "ratelimit:test".to_string(),
            tokens: 1,
            capacity: 10,
            refill_rate: 1.0,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_barrier_enter() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::BarrierEnter {
            name: "barrier:test".to_string(),
            participant_id: "p1".to_string(),
            required_count: 3,
            timeout_ms: 5000,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_semaphore_acquire() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::SemaphoreAcquire {
            name: "sem:test".to_string(),
            holder_id: "h1".to_string(),
            permits: 1,
            capacity: 10,
            ttl_ms: 5000,
            timeout_ms: 1000,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_rwlock_acquire_read() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::RWLockAcquireRead {
            name: "rwlock:test".to_string(),
            holder_id: "h1".to_string(),
            ttl_ms: 5000,
            timeout_ms: 1000,
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_can_handle_queue_create() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::QueueCreate {
            queue_name: "queue:test".to_string(),
            default_visibility_timeout_ms: Some(30000),
            default_ttl_ms: Some(86400000),
            max_delivery_attempts: Some(5),
        };
        assert!(handler.can_handle(&request));
    }

    #[test]
    fn test_cannot_handle_kv_request() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::ReadKey {
            key: "some:key".to_string(),
        };
        assert!(!handler.can_handle(&request));
    }

    #[test]
    fn test_cannot_handle_cluster_request() {
        let handler = CoordinationHandler;
        let request = ClientRpcRequest::GetMetrics;
        assert!(!handler.can_handle(&request));
    }
}

// =============================================================================
// Counter Tests
// =============================================================================

mod counter_tests {
    use super::*;

    #[tokio::test]
    async fn test_counter_get_uninitialized() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::CounterGet {
            key: "counter:test".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(0)); // Uninitialized counter defaults to 0
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_increment() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Increment
        let request = ClientRpcRequest::CounterIncrement {
            key: "counter:inc".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(1));
            }
            other => panic!("unexpected response: {:?}", other),
        }

        // Increment again
        let request = ClientRpcRequest::CounterIncrement {
            key: "counter:inc".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(2));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_decrement() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Set initial value
        let set_req = ClientRpcRequest::CounterSet {
            key: "counter:dec".to_string(),
            value: 10,
        };
        handler.handle(set_req, &ctx).await.unwrap();

        // Decrement
        let request = ClientRpcRequest::CounterDecrement {
            key: "counter:dec".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(9));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_add() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::CounterAdd {
            key: "counter:add".to_string(),
            amount: 5,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(5));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_subtract() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Set initial value
        let set_req = ClientRpcRequest::CounterSet {
            key: "counter:sub".to_string(),
            value: 100,
        };
        handler.handle(set_req, &ctx).await.unwrap();

        let request = ClientRpcRequest::CounterSubtract {
            key: "counter:sub".to_string(),
            amount: 30,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(70));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_set() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::CounterSet {
            key: "counter:set".to_string(),
            value: 42,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(42));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_compare_and_set_success() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Set initial value
        let set_req = ClientRpcRequest::CounterSet {
            key: "counter:cas".to_string(),
            value: 10,
        };
        handler.handle(set_req, &ctx).await.unwrap();

        // CAS with correct expected value
        let request = ClientRpcRequest::CounterCompareAndSet {
            key: "counter:cas".to_string(),
            expected: 10,
            new_value: 20,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_counter_empty_key_behavior() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Empty key is still processed by the handler (validation at a higher level)
        let request = ClientRpcRequest::CounterGet { key: "".to_string() };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::CounterResult(result) => {
                // The handler processes the request; validation may happen elsewhere
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// Signed Counter Tests
// =============================================================================

mod signed_counter_tests {
    use super::*;

    #[tokio::test]
    async fn test_signed_counter_get_uninitialized() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::SignedCounterGet {
            key: "signed:test".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SignedCounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(0));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_signed_counter_add_positive() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::SignedCounterAdd {
            key: "signed:add".to_string(),
            amount: 100,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SignedCounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(100));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_signed_counter_add_negative() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // First add some positive value
        let add_req = ClientRpcRequest::SignedCounterAdd {
            key: "signed:neg".to_string(),
            amount: 50,
        };
        handler.handle(add_req, &ctx).await.unwrap();

        // Then subtract (add negative)
        let request = ClientRpcRequest::SignedCounterAdd {
            key: "signed:neg".to_string(),
            amount: -30,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SignedCounterResult(result) => {
                assert!(result.success);
                assert_eq!(result.value, Some(20));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// Sequence Tests
// =============================================================================

mod sequence_tests {
    use super::*;

    #[tokio::test]
    async fn test_sequence_next_returns_incrementing_values() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // First call - get initial value
        let request = ClientRpcRequest::SequenceNext {
            key: "seq:test".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        let first_value = match response {
            ClientRpcResponse::SequenceResult(result) => {
                assert!(result.success);
                result.value.unwrap()
            }
            other => panic!("unexpected response: {:?}", other),
        };

        // First value should be 1 (start of first batch)
        assert_eq!(first_value, 1);

        // Second call with same key returns an incrementing value
        // Note: Due to the batched implementation, each handler call creates a new
        // SequenceGenerator instance, so values increment by batch_size (100)
        let request = ClientRpcRequest::SequenceNext {
            key: "seq:test".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SequenceResult(result) => {
                assert!(result.success);
                // Next batch starts at 101 (after first batch of 100)
                let second_value = result.value.unwrap();
                assert!(second_value > first_value);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sequence_reserve() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Reserve 5 values
        let request = ClientRpcRequest::SequenceReserve {
            key: "seq:reserve".to_string(),
            count: 5,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SequenceResult(result) => {
                assert!(result.success);
                // Returns start of reserved range
                let start = result.value.unwrap();
                assert_eq!(start, 1); // First reservation starts at 1
            }
            other => panic!("unexpected response: {:?}", other),
        }

        // Next reserve on same key continues from where we left off
        let request = ClientRpcRequest::SequenceReserve {
            key: "seq:reserve".to_string(),
            count: 5,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SequenceResult(result) => {
                assert!(result.success);
                let start = result.value.unwrap();
                assert_eq!(start, 6); // Continues after first 5
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_sequence_current() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Current on uninitialized sequence returns start value (1)
        let request = ClientRpcRequest::SequenceCurrent {
            key: "seq:current_new".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        let initial_value = match response {
            ClientRpcResponse::SequenceResult(result) => {
                assert!(result.success);
                result.value.unwrap()
            }
            other => panic!("unexpected response: {:?}", other),
        };

        // After a reserve, current should show the next available position
        let reserve_req = ClientRpcRequest::SequenceReserve {
            key: "seq:current_new".to_string(),
            count: 50,
        };
        handler.handle(reserve_req, &ctx).await.unwrap();

        let request = ClientRpcRequest::SequenceCurrent {
            key: "seq:current_new".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SequenceResult(result) => {
                assert!(result.success);
                // After reserving 50, current should be higher than initial
                let current = result.value.unwrap();
                assert!(current > initial_value);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// Rate Limiter Tests
// =============================================================================

mod rate_limiter_tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_try_acquire_success() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::RateLimiterTryAcquire {
            key: "ratelimit:test".to_string(),
            tokens: 1,
            capacity: 10,
            refill_rate: 1.0,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RateLimiterResult(result) => {
                assert!(result.success);
                // success=true means tokens were acquired
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_available() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // First acquire some tokens to initialize
        let acquire_req = ClientRpcRequest::RateLimiterTryAcquire {
            key: "ratelimit:avail".to_string(),
            tokens: 3,
            capacity: 10,
            refill_rate: 1.0,
        };
        handler.handle(acquire_req, &ctx).await.unwrap();

        // Check available
        let request = ClientRpcRequest::RateLimiterAvailable {
            key: "ratelimit:avail".to_string(),
            capacity: 10,
            refill_rate: 1.0,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RateLimiterResult(result) => {
                assert!(result.success);
                // tokens_remaining indicates available tokens
                assert!(result.tokens_remaining.is_some());
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Acquire all tokens
        let acquire_req = ClientRpcRequest::RateLimiterTryAcquire {
            key: "ratelimit:reset".to_string(),
            tokens: 10,
            capacity: 10,
            refill_rate: 1.0,
        };
        handler.handle(acquire_req, &ctx).await.unwrap();

        // Reset
        let request = ClientRpcRequest::RateLimiterReset {
            key: "ratelimit:reset".to_string(),
            capacity: 10,
            refill_rate: 1.0,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RateLimiterResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }

        // Should now be able to acquire again
        let acquire_req = ClientRpcRequest::RateLimiterTryAcquire {
            key: "ratelimit:reset".to_string(),
            tokens: 5,
            capacity: 10,
            refill_rate: 1.0,
        };
        let response = handler.handle(acquire_req, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RateLimiterResult(result) => {
                assert!(result.success);
                // success=true means tokens were acquired
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// Barrier Tests
// =============================================================================

mod barrier_tests {
    use super::*;

    #[tokio::test]
    async fn test_barrier_status() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::BarrierStatus {
            name: "barrier:status".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::BarrierStatusResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_barrier_leave() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Leave from a barrier (even if not entered)
        let request = ClientRpcRequest::BarrierLeave {
            name: "barrier:leave".to_string(),
            participant_id: "p1".to_string(),
            timeout_ms: 5000,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::BarrierLeaveResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// Semaphore Tests
// =============================================================================

mod semaphore_tests {
    use super::*;

    #[tokio::test]
    async fn test_semaphore_try_acquire_success() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::SemaphoreTryAcquire {
            name: "sem:test".to_string(),
            holder_id: "holder1".to_string(),
            permits: 1,
            capacity: 10,
            ttl_ms: 5000,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SemaphoreTryAcquireResult(result) => {
                assert!(result.success);
                assert!(result.permits_acquired.unwrap_or(0) > 0);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_semaphore_release() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // First acquire
        let acquire_req = ClientRpcRequest::SemaphoreTryAcquire {
            name: "sem:release".to_string(),
            holder_id: "holder1".to_string(),
            permits: 3,
            capacity: 10,
            ttl_ms: 5000,
        };
        handler.handle(acquire_req, &ctx).await.unwrap();

        // Release
        let request = ClientRpcRequest::SemaphoreRelease {
            name: "sem:release".to_string(),
            holder_id: "holder1".to_string(),
            permits: 1,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SemaphoreReleaseResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_semaphore_status() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::SemaphoreStatus {
            name: "sem:status".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::SemaphoreStatusResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// RWLock Tests
// =============================================================================

mod rwlock_tests {
    use super::*;

    #[tokio::test]
    async fn test_rwlock_try_acquire_read_success() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::RWLockTryAcquireRead {
            name: "rwlock:read".to_string(),
            holder_id: "reader1".to_string(),
            ttl_ms: 5000,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RWLockTryAcquireReadResult(result) => {
                assert!(result.success);
                // success=true means lock was acquired
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rwlock_try_acquire_write_success() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::RWLockTryAcquireWrite {
            name: "rwlock:write".to_string(),
            holder_id: "writer1".to_string(),
            ttl_ms: 5000,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RWLockTryAcquireWriteResult(result) => {
                assert!(result.success);
                // success=true means lock was acquired
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rwlock_release_read() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Acquire read lock
        let acquire_req = ClientRpcRequest::RWLockTryAcquireRead {
            name: "rwlock:release_read".to_string(),
            holder_id: "reader1".to_string(),
            ttl_ms: 5000,
        };
        handler.handle(acquire_req, &ctx).await.unwrap();

        // Release read lock
        let request = ClientRpcRequest::RWLockReleaseRead {
            name: "rwlock:release_read".to_string(),
            holder_id: "reader1".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RWLockReleaseReadResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_rwlock_status() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::RWLockStatus {
            name: "rwlock:status".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::RWLockStatusResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}

// =============================================================================
// Queue Tests
// =============================================================================

mod queue_tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_create() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::QueueCreate {
            queue_name: "queue:test".to_string(),
            default_visibility_timeout_ms: Some(30000),
            default_ttl_ms: Some(86400000),
            max_delivery_attempts: Some(5),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::QueueCreateResult(result) => {
                assert!(result.success);
                assert!(result.created); // First create should return true
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_queue_create_idempotent() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        let request = ClientRpcRequest::QueueCreate {
            queue_name: "queue:idem".to_string(),
            default_visibility_timeout_ms: Some(30000),
            default_ttl_ms: None,
            max_delivery_attempts: None,
        };

        // First create
        handler.handle(request.clone(), &ctx).await.unwrap();

        // Second create (idempotent)
        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::QueueCreateResult(result) => {
                assert!(result.success);
                assert!(!result.created); // Already existed
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_queue_enqueue() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Create queue first
        let create_req = ClientRpcRequest::QueueCreate {
            queue_name: "queue:enqueue".to_string(),
            default_visibility_timeout_ms: Some(30000),
            default_ttl_ms: None,
            max_delivery_attempts: None,
        };
        handler.handle(create_req, &ctx).await.unwrap();

        // Enqueue message
        let request = ClientRpcRequest::QueueEnqueue {
            queue_name: "queue:enqueue".to_string(),
            payload: "test message".as_bytes().to_vec(),
            ttl_ms: None,
            message_group_id: None,
            deduplication_id: None,
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::QueueEnqueueResult(result) => {
                assert!(result.success);
                assert!(result.item_id.is_some());
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_queue_status() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Create queue
        let create_req = ClientRpcRequest::QueueCreate {
            queue_name: "queue:status".to_string(),
            default_visibility_timeout_ms: Some(30000),
            default_ttl_ms: None,
            max_delivery_attempts: None,
        };
        handler.handle(create_req, &ctx).await.unwrap();

        let request = ClientRpcRequest::QueueStatus {
            queue_name: "queue:status".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::QueueStatusResult(result) => {
                assert!(result.success);
                assert!(result.exists);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_queue_delete() {
        let (ctx, _kv) = test_context().await;
        let handler = CoordinationHandler;

        // Create queue
        let create_req = ClientRpcRequest::QueueCreate {
            queue_name: "queue:delete".to_string(),
            default_visibility_timeout_ms: Some(30000),
            default_ttl_ms: None,
            max_delivery_attempts: None,
        };
        handler.handle(create_req, &ctx).await.unwrap();

        let request = ClientRpcRequest::QueueDelete {
            queue_name: "queue:delete".to_string(),
        };

        let response = handler.handle(request, &ctx).await.unwrap();
        match response {
            ClientRpcResponse::QueueDeleteResult(result) => {
                assert!(result.success);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}
