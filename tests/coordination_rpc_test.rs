//! Tests for coordination primitive RPC serialization and in-memory functionality.
//!
//! Tests the new coordination RPC types (locks, counters, sequences, rate limiters)
//! for correct serialization and basic functionality using the in-memory store.

use std::sync::Arc;

use aspen::api::inmemory::DeterministicKeyValueStore;
use aspen::client_rpc::{
    ClientRpcRequest, ClientRpcResponse, CounterResultResponse, LockResultResponse,
    RateLimiterResultResponse, SequenceResultResponse, SignedCounterResultResponse,
};
use aspen::coordination::{
    AtomicCounter, CounterConfig, DistributedLock, DistributedRateLimiter, LockConfig,
    RateLimiterConfig, SequenceConfig, SequenceGenerator,
};

// ============================================================================
// Coordination RPC Request Serialization Tests
// ============================================================================

#[test]
fn test_lock_acquire_request_roundtrip() {
    let request = ClientRpcRequest::LockAcquire {
        key: "test_lock".to_string(),
        holder_id: "client_1".to_string(),
        ttl_ms: 30000,
        timeout_ms: 5000,
    };
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcRequest::LockAcquire {
            key,
            holder_id,
            ttl_ms,
            timeout_ms,
        } => {
            assert_eq!(key, "test_lock");
            assert_eq!(holder_id, "client_1");
            assert_eq!(ttl_ms, 30000);
            assert_eq!(timeout_ms, 5000);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_lock_try_acquire_request_roundtrip() {
    let request = ClientRpcRequest::LockTryAcquire {
        key: "test_lock".to_string(),
        holder_id: "client_1".to_string(),
        ttl_ms: 30000,
    };
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcRequest::LockTryAcquire {
            key,
            holder_id,
            ttl_ms,
        } => {
            assert_eq!(key, "test_lock");
            assert_eq!(holder_id, "client_1");
            assert_eq!(ttl_ms, 30000);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_lock_release_request_roundtrip() {
    let request = ClientRpcRequest::LockRelease {
        key: "test_lock".to_string(),
        holder_id: "client_1".to_string(),
        fencing_token: 42,
    };
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcRequest::LockRelease {
            key,
            holder_id,
            fencing_token,
        } => {
            assert_eq!(key, "test_lock");
            assert_eq!(holder_id, "client_1");
            assert_eq!(fencing_token, 42);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_counter_operations_roundtrip() {
    let requests = vec![
        ClientRpcRequest::CounterGet {
            key: "counter1".to_string(),
        },
        ClientRpcRequest::CounterIncrement {
            key: "counter1".to_string(),
        },
        ClientRpcRequest::CounterDecrement {
            key: "counter1".to_string(),
        },
        ClientRpcRequest::CounterAdd {
            key: "counter1".to_string(),
            amount: 10,
        },
        ClientRpcRequest::CounterSubtract {
            key: "counter1".to_string(),
            amount: 5,
        },
        ClientRpcRequest::CounterSet {
            key: "counter1".to_string(),
            value: 100,
        },
        ClientRpcRequest::CounterCompareAndSet {
            key: "counter1".to_string(),
            expected: 100,
            new_value: 200,
        },
    ];

    for request in requests {
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&request),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn test_sequence_operations_roundtrip() {
    let requests = vec![
        ClientRpcRequest::SequenceNext {
            key: "seq1".to_string(),
        },
        ClientRpcRequest::SequenceReserve {
            key: "seq1".to_string(),
            count: 100,
        },
        ClientRpcRequest::SequenceCurrent {
            key: "seq1".to_string(),
        },
    ];

    for request in requests {
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&request),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn test_rate_limiter_operations_roundtrip() {
    let requests = vec![
        ClientRpcRequest::RateLimiterTryAcquire {
            key: "limiter1".to_string(),
            tokens: 1,
            capacity: 100,
            refill_rate: 10.0,
        },
        ClientRpcRequest::RateLimiterAcquire {
            key: "limiter1".to_string(),
            tokens: 5,
            capacity: 100,
            refill_rate: 10.0,
            timeout_ms: 5000,
        },
        ClientRpcRequest::RateLimiterAvailable {
            key: "limiter1".to_string(),
            capacity: 100,
            refill_rate: 10.0,
        },
        ClientRpcRequest::RateLimiterReset {
            key: "limiter1".to_string(),
            capacity: 100,
            refill_rate: 10.0,
        },
    ];

    for request in requests {
        let serialized = postcard::to_stdvec(&request).expect("serialize");
        let deserialized: ClientRpcRequest =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&request),
            std::mem::discriminant(&deserialized)
        );
    }
}

// ============================================================================
// Coordination RPC Response Serialization Tests
// ============================================================================

#[test]
fn test_lock_result_response_roundtrip() {
    let responses = vec![
        ClientRpcResponse::LockResult(LockResultResponse {
            success: true,
            fencing_token: Some(42),
            holder_id: Some("client_1".to_string()),
            deadline_ms: Some(1234567890),
            error: None,
        }),
        ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: Some("other_client".to_string()),
            deadline_ms: Some(9999999999),
            error: Some("lock held by another client".to_string()),
        }),
    ];

    for response in responses {
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&response),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn test_counter_result_response_roundtrip() {
    let responses = vec![
        ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(42),
            error: None,
        }),
        ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some("storage error".to_string()),
        }),
    ];

    for response in responses {
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&response),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn test_signed_counter_result_response_roundtrip() {
    let responses = vec![
        ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: true,
            value: Some(-42),
            error: None,
        }),
        ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: true,
            value: Some(i64::MAX),
            error: None,
        }),
    ];

    for response in responses {
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&response),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn test_sequence_result_response_roundtrip() {
    let response = ClientRpcResponse::SequenceResult(SequenceResultResponse {
        success: true,
        value: Some(12345),
        error: None,
    });
    let serialized = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcResponse::SequenceResult(r) => {
            assert!(r.success);
            assert_eq!(r.value, Some(12345));
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_rate_limiter_result_response_roundtrip() {
    let responses = vec![
        ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: true,
            tokens_remaining: Some(95),
            retry_after_ms: None,
            error: None,
        }),
        ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
            success: false,
            tokens_remaining: Some(0),
            retry_after_ms: Some(500),
            error: None,
        }),
    ];

    for response in responses {
        let serialized = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: ClientRpcResponse =
            postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(
            std::mem::discriminant(&response),
            std::mem::discriminant(&deserialized)
        );
    }
}

// ============================================================================
// Rate Limiting Functional Tests (In-Memory Store)
// ============================================================================

#[tokio::test]
async fn test_rate_limiter_basic_acquire() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let limiter = DistributedRateLimiter::new(
        store,
        "test_limiter",
        RateLimiterConfig::new(10.0, 5), // 10/sec, burst 5
    );

    // Should allow burst of 5
    for i in 0..5 {
        let result = limiter.try_acquire().await;
        assert!(result.is_ok(), "Acquire {} should succeed", i);
    }

    // 6th should fail
    let result = limiter.try_acquire().await;
    assert!(result.is_err(), "6th acquire should fail (burst exhausted)");
}

#[tokio::test]
async fn test_rate_limiter_returns_retry_after() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let limiter = DistributedRateLimiter::new(
        store,
        "test_limiter",
        RateLimiterConfig::new(1.0, 1), // 1/sec, burst 1
    );

    // Exhaust the token
    limiter.try_acquire().await.unwrap();

    // Next acquire should fail with retry_after
    let err = limiter.try_acquire().await.unwrap_err();
    assert!(err.retry_after_ms > 0, "Should have retry_after_ms");
    assert!(
        err.retry_after_ms <= 1000,
        "retry_after_ms should be <= 1 second"
    );
}

#[tokio::test]
async fn test_counter_increment_decrement() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let counter = AtomicCounter::new(store, "test_counter", CounterConfig::default());

    // Initial value should be 0
    assert_eq!(counter.get().await.unwrap(), 0);

    // Increment
    assert_eq!(counter.increment().await.unwrap(), 1);
    assert_eq!(counter.increment().await.unwrap(), 2);

    // Add
    assert_eq!(counter.add(10).await.unwrap(), 12);

    // Decrement
    assert_eq!(counter.decrement().await.unwrap(), 11);

    // Subtract
    assert_eq!(counter.subtract(5).await.unwrap(), 6);

    // Final value
    assert_eq!(counter.get().await.unwrap(), 6);
}

#[tokio::test]
async fn test_sequence_generator_monotonic() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let seq = SequenceGenerator::new(store, "test_seq", SequenceConfig::default());

    let mut prev = 0;
    for _ in 0..100 {
        let id = seq.next().await.unwrap();
        assert!(id > prev, "ID {} should be > {}", id, prev);
        prev = id;
    }
}

#[tokio::test]
async fn test_distributed_lock_acquire_release() {
    let store = Arc::new(DeterministicKeyValueStore::new());
    let lock = DistributedLock::new(store, "test_lock", "holder_1", LockConfig::default());

    // Acquire lock
    let guard = lock.try_acquire().await.unwrap();
    assert!(guard.fencing_token().value() > 0);

    // Release lock explicitly (drop spawns async task which may not complete before next acquire)
    guard.release().await.unwrap();

    // Should be able to acquire again
    let guard2 = lock.try_acquire().await.unwrap();
    assert!(guard2.fencing_token().value() > 0);
}

#[tokio::test]
async fn test_lock_contention() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // First holder acquires
    let lock1 = DistributedLock::new(
        store.clone(),
        "test_lock",
        "holder_1",
        LockConfig::default(),
    );
    let _guard1 = lock1.try_acquire().await.unwrap();

    // Second holder should fail
    let lock2 = DistributedLock::new(
        store.clone(),
        "test_lock",
        "holder_2",
        LockConfig::default(),
    );
    let result = lock2.try_acquire().await;
    assert!(result.is_err(), "Second holder should fail to acquire");
}
