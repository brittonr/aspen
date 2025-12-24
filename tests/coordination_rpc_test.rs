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

// ============================================================================
// Batch Operation Tests (In-Memory Store)
// ============================================================================

use aspen::api::{
    BatchCondition, BatchOperation, KeyValueStore, ReadRequest, WriteCommand, WriteRequest,
};

#[tokio::test]
async fn test_batch_write_set_operations() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Batch set multiple keys
    let request = WriteRequest {
        command: WriteCommand::Batch {
            operations: vec![
                BatchOperation::Set {
                    key: "key1".to_string(),
                    value: "value1".to_string(),
                },
                BatchOperation::Set {
                    key: "key2".to_string(),
                    value: "value2".to_string(),
                },
                BatchOperation::Set {
                    key: "key3".to_string(),
                    value: "value3".to_string(),
                },
            ],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.batch_applied, Some(3));

    // Verify all keys were set
    let r1 = store
        .read(ReadRequest::new("key1".to_string()))
        .await
        .unwrap();
    assert_eq!(r1.kv.unwrap().value, "value1");

    let r2 = store
        .read(ReadRequest::new("key2".to_string()))
        .await
        .unwrap();
    assert_eq!(r2.kv.unwrap().value, "value2");

    let r3 = store
        .read(ReadRequest::new("key3".to_string()))
        .await
        .unwrap();
    assert_eq!(r3.kv.unwrap().value, "value3");
}

#[tokio::test]
async fn test_batch_write_mixed_operations() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // First, set some keys
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "to_delete".to_string(),
                value: "delete_me".to_string(),
            },
        })
        .await
        .unwrap();

    // Now batch: set one key, delete another
    let request = WriteRequest {
        command: WriteCommand::Batch {
            operations: vec![
                BatchOperation::Set {
                    key: "new_key".to_string(),
                    value: "new_value".to_string(),
                },
                BatchOperation::Delete {
                    key: "to_delete".to_string(),
                },
            ],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.batch_applied, Some(2));

    // Verify new key exists
    let r1 = store
        .read(ReadRequest::new("new_key".to_string()))
        .await
        .unwrap();
    assert_eq!(r1.kv.unwrap().value, "new_value");

    // Verify deleted key is gone
    let r2 = store.read(ReadRequest::new("to_delete".to_string())).await;
    assert!(r2.is_err(), "Deleted key should not exist");
}

#[tokio::test]
async fn test_conditional_batch_success() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Set up preconditions
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "version".to_string(),
                value: "1".to_string(),
            },
        })
        .await
        .unwrap();

    // Conditional batch: if version == "1", update to "2" and set new data
    let request = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::ValueEquals {
                key: "version".to_string(),
                expected: "1".to_string(),
            }],
            operations: vec![
                BatchOperation::Set {
                    key: "version".to_string(),
                    value: "2".to_string(),
                },
                BatchOperation::Set {
                    key: "data".to_string(),
                    value: "updated".to_string(),
                },
            ],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.conditions_met, Some(true));
    assert_eq!(result.batch_applied, Some(2));

    // Verify updates
    let version = store
        .read(ReadRequest::new("version".to_string()))
        .await
        .unwrap();
    assert_eq!(version.kv.unwrap().value, "2");

    let data = store
        .read(ReadRequest::new("data".to_string()))
        .await
        .unwrap();
    assert_eq!(data.kv.unwrap().value, "updated");
}

#[tokio::test]
async fn test_conditional_batch_failure() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Set up preconditions
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "version".to_string(),
                value: "2".to_string(), // Not "1"
            },
        })
        .await
        .unwrap();

    // Conditional batch: if version == "1", update (should fail)
    let request = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::ValueEquals {
                key: "version".to_string(),
                expected: "1".to_string(),
            }],
            operations: vec![BatchOperation::Set {
                key: "data".to_string(),
                value: "should_not_appear".to_string(),
            }],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.conditions_met, Some(false));
    assert_eq!(result.failed_condition_index, Some(0));
    assert_eq!(result.batch_applied, None);

    // Verify data was NOT written
    let data_result = store.read(ReadRequest::new("data".to_string())).await;
    assert!(
        data_result.is_err(),
        "Data should not be written on failed condition"
    );
}

#[tokio::test]
async fn test_conditional_batch_key_exists() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Set up: create a key
    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: "existing".to_string(),
                value: "value".to_string(),
            },
        })
        .await
        .unwrap();

    // Conditional batch: if key exists, update it
    let request = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::KeyExists {
                key: "existing".to_string(),
            }],
            operations: vec![BatchOperation::Set {
                key: "existing".to_string(),
                value: "updated".to_string(),
            }],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.conditions_met, Some(true));

    let existing = store
        .read(ReadRequest::new("existing".to_string()))
        .await
        .unwrap();
    assert_eq!(existing.kv.unwrap().value, "updated");
}

#[tokio::test]
async fn test_conditional_batch_key_not_exists() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Conditional batch: create only if key doesn't exist
    let request = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::KeyNotExists {
                key: "new_key".to_string(),
            }],
            operations: vec![BatchOperation::Set {
                key: "new_key".to_string(),
                value: "created".to_string(),
            }],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.conditions_met, Some(true));

    let new_key = store
        .read(ReadRequest::new("new_key".to_string()))
        .await
        .unwrap();
    assert_eq!(new_key.kv.unwrap().value, "created");

    // Try again - should fail because key now exists
    let request2 = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::KeyNotExists {
                key: "new_key".to_string(),
            }],
            operations: vec![BatchOperation::Set {
                key: "new_key".to_string(),
                value: "overwritten".to_string(),
            }],
        },
    };

    let result2 = store.write(request2).await.unwrap();
    assert_eq!(result2.conditions_met, Some(false));

    // Value should still be "created"
    let new_key_after = store
        .read(ReadRequest::new("new_key".to_string()))
        .await
        .unwrap();
    assert_eq!(new_key_after.kv.unwrap().value, "created");
}

#[tokio::test]
async fn test_conditional_batch_multiple_conditions() {
    let store = Arc::new(DeterministicKeyValueStore::new());

    // Set up multiple keys
    store
        .write(WriteRequest {
            command: WriteCommand::SetMulti {
                pairs: vec![
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                ],
            },
        })
        .await
        .unwrap();

    // Conditional batch: check multiple conditions
    let request = WriteRequest {
        command: WriteCommand::ConditionalBatch {
            conditions: vec![
                BatchCondition::ValueEquals {
                    key: "key1".to_string(),
                    expected: "value1".to_string(),
                },
                BatchCondition::KeyExists {
                    key: "key2".to_string(),
                },
                BatchCondition::KeyNotExists {
                    key: "key3".to_string(),
                },
            ],
            operations: vec![BatchOperation::Set {
                key: "result".to_string(),
                value: "all_conditions_passed".to_string(),
            }],
        },
    };

    let result = store.write(request).await.unwrap();
    assert_eq!(result.conditions_met, Some(true));

    let result_val = store
        .read(ReadRequest::new("result".to_string()))
        .await
        .unwrap();
    assert_eq!(result_val.kv.unwrap().value, "all_conditions_passed");
}

// ============================================================================
// Batch RPC Serialization Tests
// ============================================================================

use aspen::client_rpc::{
    BatchCondition as RpcBatchCondition, BatchReadResultResponse, BatchWriteOperation,
    BatchWriteResultResponse, ConditionalBatchWriteResultResponse,
};

#[test]
fn test_batch_read_request_roundtrip() {
    let request = ClientRpcRequest::BatchRead {
        keys: vec!["key1".to_string(), "key2".to_string(), "key3".to_string()],
    };
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcRequest::BatchRead { keys } => {
            assert_eq!(keys.len(), 3);
            assert_eq!(keys[0], "key1");
            assert_eq!(keys[1], "key2");
            assert_eq!(keys[2], "key3");
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_batch_write_request_roundtrip() {
    let request = ClientRpcRequest::BatchWrite {
        operations: vec![
            BatchWriteOperation::Set {
                key: "key1".to_string(),
                value: b"value1".to_vec(),
            },
            BatchWriteOperation::Delete {
                key: "key2".to_string(),
            },
        ],
    };
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcRequest::BatchWrite { operations } => {
            assert_eq!(operations.len(), 2);
            match &operations[0] {
                BatchWriteOperation::Set { key, value } => {
                    assert_eq!(key, "key1");
                    assert_eq!(value, b"value1");
                }
                _ => panic!("Expected Set"),
            }
            match &operations[1] {
                BatchWriteOperation::Delete { key } => {
                    assert_eq!(key, "key2");
                }
                _ => panic!("Expected Delete"),
            }
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_conditional_batch_write_request_roundtrip() {
    let request = ClientRpcRequest::ConditionalBatchWrite {
        conditions: vec![
            RpcBatchCondition::ValueEquals {
                key: "version".to_string(),
                expected: b"1".to_vec(),
            },
            RpcBatchCondition::KeyExists {
                key: "exists".to_string(),
            },
            RpcBatchCondition::KeyNotExists {
                key: "new".to_string(),
            },
        ],
        operations: vec![BatchWriteOperation::Set {
            key: "data".to_string(),
            value: b"updated".to_vec(),
        }],
    };
    let serialized = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: ClientRpcRequest = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcRequest::ConditionalBatchWrite {
            conditions,
            operations,
        } => {
            assert_eq!(conditions.len(), 3);
            assert_eq!(operations.len(), 1);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_batch_read_result_response_roundtrip() {
    let response = ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
        success: true,
        values: Some(vec![
            Some(b"value1".to_vec()),
            None,
            Some(b"value3".to_vec()),
        ]),
        error: None,
    });
    let serialized = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcResponse::BatchReadResult(result) => {
            assert!(result.success);
            let values = result.values.unwrap();
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], Some(b"value1".to_vec()));
            assert_eq!(values[1], None);
            assert_eq!(values[2], Some(b"value3".to_vec()));
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_batch_write_result_response_roundtrip() {
    let response = ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
        success: true,
        operations_applied: Some(5),
        error: None,
    });
    let serialized = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcResponse::BatchWriteResult(result) => {
            assert!(result.success);
            assert_eq!(result.operations_applied, Some(5));
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_conditional_batch_write_result_response_roundtrip() {
    // Test success case
    let response =
        ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
            success: true,
            conditions_met: true,
            operations_applied: Some(3),
            failed_condition_index: None,
            failed_condition_reason: None,
            error: None,
        });
    let serialized = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: ClientRpcResponse = postcard::from_bytes(&serialized).expect("deserialize");
    match deserialized {
        ClientRpcResponse::ConditionalBatchWriteResult(result) => {
            assert!(result.success);
            assert!(result.conditions_met);
            assert_eq!(result.operations_applied, Some(3));
        }
        _ => panic!("Wrong variant"),
    }

    // Test failure case
    let response_fail =
        ClientRpcResponse::ConditionalBatchWriteResult(ConditionalBatchWriteResultResponse {
            success: false,
            conditions_met: false,
            operations_applied: None,
            failed_condition_index: Some(1),
            failed_condition_reason: Some("value mismatch".to_string()),
            error: None,
        });
    let serialized_fail = postcard::to_stdvec(&response_fail).expect("serialize");
    let deserialized_fail: ClientRpcResponse =
        postcard::from_bytes(&serialized_fail).expect("deserialize");
    match deserialized_fail {
        ClientRpcResponse::ConditionalBatchWriteResult(result) => {
            assert!(!result.success);
            assert!(!result.conditions_met);
            assert_eq!(result.failed_condition_index, Some(1));
            assert_eq!(
                result.failed_condition_reason,
                Some("value mismatch".to_string())
            );
        }
        _ => panic!("Wrong variant"),
    }
}
