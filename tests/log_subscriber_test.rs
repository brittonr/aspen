//! Integration tests for the log subscriber protocol.
//!
//! Tests the log subscription streaming protocol including:
//! - Protocol message serialization/deserialization
//! - Subscription request handling
//! - Log entry broadcasting
//! - Key prefix filtering
//! - Error handling

mod support;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen::raft::log_subscriber::EndOfStreamReason;
use aspen::raft::log_subscriber::KvOperation;
use aspen::raft::log_subscriber::LOG_SUBSCRIBE_PROTOCOL_VERSION;
use aspen::raft::log_subscriber::LogEntryMessage;
use aspen::raft::log_subscriber::LogEntryPayload;
use aspen::raft::log_subscriber::MAX_LOG_ENTRY_MESSAGE_SIZE;
use aspen::raft::log_subscriber::SubscribeRejectReason;
use aspen::raft::log_subscriber::SubscribeRequest;
use aspen::raft::log_subscriber::SubscribeResponse;
use tokio::sync::broadcast;

// ============================================================================
// Protocol Message Serialization Tests
// ============================================================================

#[test]
fn test_subscribe_request_from_index_roundtrip() {
    let request = SubscribeRequest::from_index(100);
    let bytes = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: SubscribeRequest = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(deserialized.start_index, 100);
    assert!(deserialized.key_prefix.is_empty());
    assert_eq!(deserialized.protocol_version, LOG_SUBSCRIBE_PROTOCOL_VERSION);
}

#[test]
fn test_subscribe_request_latest_only_roundtrip() {
    let request = SubscribeRequest::latest_only();
    let bytes = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: SubscribeRequest = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(deserialized.start_index, u64::MAX);
    assert!(deserialized.key_prefix.is_empty());
}

#[test]
fn test_subscribe_request_with_prefix_roundtrip() {
    let request = SubscribeRequest::with_prefix(50, b"user:".to_vec());
    let bytes = postcard::to_stdvec(&request).expect("serialize");
    let deserialized: SubscribeRequest = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(deserialized.start_index, 50);
    assert_eq!(deserialized.key_prefix, b"user:");
}

#[test]
fn test_subscribe_response_accepted_roundtrip() {
    let response = SubscribeResponse::Accepted {
        current_index: 42,
        node_id: 1,
    };
    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: SubscribeResponse = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        SubscribeResponse::Accepted { current_index, node_id } => {
            assert_eq!(current_index, 42);
            assert_eq!(node_id, 1);
        }
        _ => panic!("expected Accepted variant"),
    }
}

#[test]
fn test_subscribe_response_rejected_roundtrip() {
    let reasons = vec![
        SubscribeRejectReason::TooManySubscribers,
        SubscribeRejectReason::IndexNotAvailable,
        SubscribeRejectReason::UnsupportedVersion,
        SubscribeRejectReason::NotReady,
        SubscribeRejectReason::InternalError,
    ];
    for reason in reasons {
        let response = SubscribeResponse::Rejected { reason };
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: SubscribeResponse = postcard::from_bytes(&bytes).expect("deserialize");
        match deserialized {
            SubscribeResponse::Rejected { reason: r } => {
                assert_eq!(r, reason);
            }
            _ => panic!("expected Rejected variant"),
        }
    }
}

#[test]
fn test_log_entry_message_entry_roundtrip() {
    let payload = LogEntryPayload {
        index: 100,
        term: 5,
        committed_at_ms: 1700000000000,
        operation: KvOperation::Set {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
        },
    };
    let message = LogEntryMessage::Entry(payload);
    let bytes = postcard::to_stdvec(&message).expect("serialize");
    let deserialized: LogEntryMessage = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        LogEntryMessage::Entry(p) => {
            assert_eq!(p.index, 100);
            assert_eq!(p.term, 5);
            match p.operation {
                KvOperation::Set { key, value } => {
                    assert_eq!(key, b"test_key");
                    assert_eq!(value, b"test_value");
                }
                _ => panic!("expected Set operation"),
            }
        }
        _ => panic!("expected Entry variant"),
    }
}

#[test]
fn test_log_entry_message_keepalive_roundtrip() {
    let message = LogEntryMessage::Keepalive {
        committed_index: 500,
        timestamp_ms: 1700000000000,
    };
    let bytes = postcard::to_stdvec(&message).expect("serialize");
    let deserialized: LogEntryMessage = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        LogEntryMessage::Keepalive {
            committed_index,
            timestamp_ms,
        } => {
            assert_eq!(committed_index, 500);
            assert_eq!(timestamp_ms, 1700000000000);
        }
        _ => panic!("expected Keepalive variant"),
    }
}

#[test]
fn test_log_entry_message_end_of_stream_roundtrip() {
    let reasons = vec![
        EndOfStreamReason::ServerShutdown,
        EndOfStreamReason::ClientDisconnect,
        EndOfStreamReason::Lagged,
        EndOfStreamReason::InternalError,
    ];
    for reason in reasons {
        let message = LogEntryMessage::EndOfStream { reason };
        let bytes = postcard::to_stdvec(&message).expect("serialize");
        let deserialized: LogEntryMessage = postcard::from_bytes(&bytes).expect("deserialize");
        match deserialized {
            LogEntryMessage::EndOfStream { reason: r } => {
                assert_eq!(r, reason);
            }
            _ => panic!("expected EndOfStream variant"),
        }
    }
}

// ============================================================================
// KvOperation Tests
// ============================================================================

#[test]
fn test_kv_operation_set_roundtrip() {
    let op = KvOperation::Set {
        key: b"mykey".to_vec(),
        value: b"myvalue".to_vec(),
    };
    let bytes = postcard::to_stdvec(&op).expect("serialize");
    let deserialized: KvOperation = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        KvOperation::Set { key, value } => {
            assert_eq!(key, b"mykey");
            assert_eq!(value, b"myvalue");
        }
        _ => panic!("expected Set"),
    }
}

#[test]
fn test_kv_operation_set_with_ttl_roundtrip() {
    let op = KvOperation::SetWithTTL {
        key: b"expire_key".to_vec(),
        value: b"expire_value".to_vec(),
        expires_at_ms: 1800000000000,
    };
    let bytes = postcard::to_stdvec(&op).expect("serialize");
    let deserialized: KvOperation = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        KvOperation::SetWithTTL {
            key,
            value,
            expires_at_ms,
        } => {
            assert_eq!(key, b"expire_key");
            assert_eq!(value, b"expire_value");
            assert_eq!(expires_at_ms, 1800000000000);
        }
        _ => panic!("expected SetWithTTL"),
    }
}

#[test]
fn test_kv_operation_set_multi_roundtrip() {
    let op = KvOperation::SetMulti {
        pairs: vec![
            (b"k1".to_vec(), b"v1".to_vec()),
            (b"k2".to_vec(), b"v2".to_vec()),
            (b"k3".to_vec(), b"v3".to_vec()),
        ],
    };
    let bytes = postcard::to_stdvec(&op).expect("serialize");
    let deserialized: KvOperation = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        KvOperation::SetMulti { pairs } => {
            assert_eq!(pairs.len(), 3);
            assert_eq!(pairs[0], (b"k1".to_vec(), b"v1".to_vec()));
        }
        _ => panic!("expected SetMulti"),
    }
}

#[test]
fn test_kv_operation_delete_roundtrip() {
    let op = KvOperation::Delete {
        key: b"deleted_key".to_vec(),
    };
    let bytes = postcard::to_stdvec(&op).expect("serialize");
    let deserialized: KvOperation = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        KvOperation::Delete { key } => {
            assert_eq!(key, b"deleted_key");
        }
        _ => panic!("expected Delete"),
    }
}

#[test]
fn test_kv_operation_compare_and_swap_roundtrip() {
    let op = KvOperation::CompareAndSwap {
        key: b"cas_key".to_vec(),
        expected: Some(b"old_value".to_vec()),
        new_value: b"new_value".to_vec(),
    };
    let bytes = postcard::to_stdvec(&op).expect("serialize");
    let deserialized: KvOperation = postcard::from_bytes(&bytes).expect("deserialize");
    match deserialized {
        KvOperation::CompareAndSwap {
            key,
            expected,
            new_value,
        } => {
            assert_eq!(key, b"cas_key");
            assert_eq!(expected, Some(b"old_value".to_vec()));
            assert_eq!(new_value, b"new_value");
        }
        _ => panic!("expected CompareAndSwap"),
    }
}

#[test]
fn test_kv_operation_noop_roundtrip() {
    let op = KvOperation::Noop;
    let bytes = postcard::to_stdvec(&op).expect("serialize");
    let deserialized: KvOperation = postcard::from_bytes(&bytes).expect("deserialize");
    assert!(matches!(deserialized, KvOperation::Noop));
}

// ============================================================================
// Key Prefix Filtering Tests
// ============================================================================

#[test]
fn test_kv_operation_matches_prefix_empty() {
    let op = KvOperation::Set {
        key: b"any_key".to_vec(),
        value: b"value".to_vec(),
    };
    // Empty prefix matches everything
    assert!(op.matches_prefix(b""));
}

#[test]
fn test_kv_operation_matches_prefix_set() {
    let op = KvOperation::Set {
        key: b"user:123:name".to_vec(),
        value: b"Alice".to_vec(),
    };
    assert!(op.matches_prefix(b"user:"));
    assert!(op.matches_prefix(b"user:123"));
    assert!(!op.matches_prefix(b"session:"));
}

#[test]
fn test_kv_operation_matches_prefix_set_multi() {
    let op = KvOperation::SetMulti {
        pairs: vec![
            (b"user:1".to_vec(), b"v1".to_vec()),
            (b"session:2".to_vec(), b"v2".to_vec()),
        ],
    };
    // Matches if any key matches
    assert!(op.matches_prefix(b"user:"));
    assert!(op.matches_prefix(b"session:"));
    assert!(!op.matches_prefix(b"config:"));
}

#[test]
fn test_kv_operation_matches_prefix_delete_multi() {
    let op = KvOperation::DeleteMulti {
        keys: vec![b"temp:a".to_vec(), b"temp:b".to_vec(), b"cache:c".to_vec()],
    };
    assert!(op.matches_prefix(b"temp:"));
    assert!(op.matches_prefix(b"cache:"));
    assert!(!op.matches_prefix(b"perm:"));
}

#[test]
fn test_kv_operation_matches_prefix_noop_always_matches() {
    let op = KvOperation::Noop;
    // Control operations always match
    assert!(op.matches_prefix(b""));
    assert!(op.matches_prefix(b"anything:"));
}

#[test]
fn test_kv_operation_matches_prefix_membership_always_matches() {
    let op = KvOperation::MembershipChange {
        description: "added node 2".to_string(),
    };
    // Control operations always match
    assert!(op.matches_prefix(b""));
    assert!(op.matches_prefix(b"anything:"));
}

// ============================================================================
// Primary Key Tests
// ============================================================================

#[test]
fn test_kv_operation_primary_key_set() {
    let op = KvOperation::Set {
        key: b"my_key".to_vec(),
        value: b"value".to_vec(),
    };
    assert_eq!(op.primary_key(), Some(b"my_key".as_slice()));
}

#[test]
fn test_kv_operation_primary_key_set_multi() {
    let op = KvOperation::SetMulti {
        pairs: vec![
            (b"first".to_vec(), b"v1".to_vec()),
            (b"second".to_vec(), b"v2".to_vec()),
        ],
    };
    // Returns first key
    assert_eq!(op.primary_key(), Some(b"first".as_slice()));
}

#[test]
fn test_kv_operation_primary_key_noop() {
    let op = KvOperation::Noop;
    assert_eq!(op.primary_key(), None);
}

// ============================================================================
// Key Count Tests
// ============================================================================

#[test]
fn test_kv_operation_key_count_single() {
    assert_eq!(
        KvOperation::Set {
            key: b"k".to_vec(),
            value: b"v".to_vec()
        }
        .key_count(),
        1
    );
    assert_eq!(KvOperation::Delete { key: b"k".to_vec() }.key_count(), 1);
}

#[test]
fn test_kv_operation_key_count_multi() {
    let op = KvOperation::SetMulti {
        pairs: vec![
            (b"k1".to_vec(), b"v1".to_vec()),
            (b"k2".to_vec(), b"v2".to_vec()),
            (b"k3".to_vec(), b"v3".to_vec()),
        ],
    };
    assert_eq!(op.key_count(), 3);
}

#[test]
fn test_kv_operation_key_count_noop() {
    assert_eq!(KvOperation::Noop.key_count(), 0);
}

// ============================================================================
// Broadcast Channel Tests
// ============================================================================

#[tokio::test]
async fn test_broadcast_channel_basic() {
    let (tx, mut rx1) = broadcast::channel::<LogEntryPayload>(100);
    let mut rx2 = tx.subscribe();

    let payload = LogEntryPayload {
        index: 1,
        term: 1,
        committed_at_ms: 1700000000000,
        operation: KvOperation::Set {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        },
    };

    tx.send(payload.clone()).expect("send should succeed");

    let received1 = rx1.recv().await.expect("rx1 should receive");
    let received2 = rx2.recv().await.expect("rx2 should receive");

    assert_eq!(received1.index, 1);
    assert_eq!(received2.index, 1);
}

#[tokio::test]
async fn test_broadcast_channel_multiple_entries() {
    let (tx, mut rx) = broadcast::channel::<LogEntryPayload>(100);

    for i in 1..=10 {
        let payload = LogEntryPayload {
            index: i,
            term: 1,
            committed_at_ms: 1700000000000 + i * 100,
            operation: KvOperation::Set {
                key: format!("key_{}", i).into_bytes(),
                value: format!("value_{}", i).into_bytes(),
            },
        };
        tx.send(payload).expect("send should succeed");
    }

    for i in 1..=10 {
        let received = rx.recv().await.expect("should receive entry");
        assert_eq!(received.index, i);
    }
}

#[tokio::test]
async fn test_broadcast_channel_lagged() {
    // Small buffer to trigger lag
    let (tx, mut rx) = broadcast::channel::<LogEntryPayload>(2);

    // Send more messages than buffer size
    for i in 1..=5 {
        let payload = LogEntryPayload {
            index: i,
            term: 1,
            committed_at_ms: 1700000000000,
            operation: KvOperation::Noop,
        };
        let _ = tx.send(payload);
    }

    // First recv should return Lagged error
    let result = rx.recv().await;
    assert!(matches!(result, Err(broadcast::error::RecvError::Lagged(_))));
}

// ============================================================================
// Committed Index Tracking Tests
// ============================================================================

#[test]
fn test_committed_index_atomic() {
    let committed_index = Arc::new(AtomicU64::new(0));

    // Simulate updating the index from different "threads"
    committed_index.store(100, Ordering::Release);
    assert_eq!(committed_index.load(Ordering::Acquire), 100);

    committed_index.fetch_add(50, Ordering::Release);
    assert_eq!(committed_index.load(Ordering::Acquire), 150);
}

// ============================================================================
// Message Size Tests
// ============================================================================

#[test]
fn test_log_entry_message_size_within_limit() {
    // A typical log entry should be well within limits
    let payload = LogEntryPayload {
        index: u64::MAX,
        term: u64::MAX,
        committed_at_ms: u64::MAX,
        operation: KvOperation::Set {
            key: vec![0u8; 1024],          // 1KB key
            value: vec![0u8; 1024 * 1024], // 1MB value
        },
    };
    let message = LogEntryMessage::Entry(payload);
    let bytes = postcard::to_stdvec(&message).expect("serialize");

    assert!(
        bytes.len() < MAX_LOG_ENTRY_MESSAGE_SIZE,
        "message size {} should be less than {}",
        bytes.len(),
        MAX_LOG_ENTRY_MESSAGE_SIZE
    );
}

#[test]
fn test_subscribe_request_size() {
    // Large prefix should still serialize
    let request = SubscribeRequest::with_prefix(0, vec![0u8; 10_000]);
    let bytes = postcard::to_stdvec(&request).expect("serialize");
    assert!(bytes.len() < MAX_LOG_ENTRY_MESSAGE_SIZE);
}

// ============================================================================
// Display Trait Tests
// ============================================================================

#[test]
fn test_subscribe_reject_reason_display() {
    assert_eq!(SubscribeRejectReason::TooManySubscribers.to_string(), "too many subscribers");
    assert_eq!(SubscribeRejectReason::IndexNotAvailable.to_string(), "requested index not available");
    assert_eq!(SubscribeRejectReason::UnsupportedVersion.to_string(), "protocol version not supported");
    assert_eq!(SubscribeRejectReason::NotReady.to_string(), "server not ready");
    assert_eq!(SubscribeRejectReason::InternalError.to_string(), "internal error");
}

#[test]
fn test_end_of_stream_reason_display() {
    assert_eq!(EndOfStreamReason::ServerShutdown.to_string(), "server shutdown");
    assert_eq!(EndOfStreamReason::ClientDisconnect.to_string(), "client disconnect");
    assert_eq!(EndOfStreamReason::Lagged.to_string(), "subscriber lagged");
    assert_eq!(EndOfStreamReason::InternalError.to_string(), "internal error");
}
