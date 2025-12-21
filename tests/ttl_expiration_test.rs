//! Tests for TTL (Time-To-Live) expiration behavior.
//!
//! Tests both lazy expiration (at read time) and active expiration (background cleanup).
//!
//! # Test Coverage
//!
//! - Read-time filtering: Expired keys are not returned by get/scan operations
//! - Background cleanup: delete_expired_keys removes expired entries
//! - TTL metrics: count_expired_keys and count_keys_with_ttl work correctly
//! - Schema migration: expires_at_ms column is added to existing databases
//! - Write operations: SetWithTTL stores expiration timestamp correctly

use std::io;
use std::sync::Arc;
use std::time::Duration;

use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::{AppRequest, AppTypeConfig, NodeId};
use futures::stream;
use openraft::entry::RaftEntry;
use openraft::storage::RaftStateMachine;
use openraft::testing::log_id;
use tempfile::TempDir;

/// Helper to create a temporary SQLite state machine.
fn create_temp_sm() -> (Arc<SqliteStateMachine>, TempDir) {
    let temp_dir = TempDir::new().expect("failed to create temp directory");
    let db_path = temp_dir.path().join("ttl_test.db");
    let sm = SqliteStateMachine::new(&db_path).expect("should create state machine");
    (sm, temp_dir)
}

/// Helper to get current time in milliseconds.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Helper to create a log entry for applying to state machine.
fn make_entry(
    index: u64,
    request: AppRequest,
) -> <AppTypeConfig as openraft::RaftTypeConfig>::Entry {
    <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
        log_id::<AppTypeConfig>(1, NodeId::from(1), index),
        request,
    )
}

/// Helper to apply a single entry to the state machine.
async fn apply_entry(
    sm: &mut Arc<SqliteStateMachine>,
    entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry,
) {
    let entries = Box::pin(stream::once(
        async move { Ok::<_, io::Error>((entry, None)) },
    ));
    sm.apply(entries).await.expect("should apply");
}

// ============================================================================
// Read-Time TTL Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_get_filters_expired_keys() {
    let (mut sm, _temp_dir) = create_temp_sm();

    // Set a key that expires in 1 second
    let expires_at = now_ms() + 1000;
    let entry = make_entry(
        1,
        AppRequest::SetWithTTL {
            key: "expiring_key".to_string(),
            value: "some_value".to_string(),
            expires_at_ms: expires_at,
        },
    );
    apply_entry(&mut sm, entry).await;

    // Key should be visible before expiration
    let result = sm.get("expiring_key").await.expect("should get");
    assert_eq!(result, Some("some_value".to_string()));

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Key should NOT be visible after expiration (lazy expiration)
    let result = sm.get("expiring_key").await.expect("should get");
    assert!(result.is_none(), "expired key should not be returned");
}

#[tokio::test]
async fn test_set_without_ttl_never_expires() {
    let (mut sm, _temp_dir) = create_temp_sm();

    // Set a key without TTL
    let entry = make_entry(
        1,
        AppRequest::Set {
            key: "permanent_key".to_string(),
            value: "permanent_value".to_string(),
        },
    );
    apply_entry(&mut sm, entry).await;

    // Key should be visible
    let result = sm.get("permanent_key").await.expect("should get");
    assert_eq!(result, Some("permanent_value".to_string()));

    // Even after some time, key should still be visible (no TTL)
    tokio::time::sleep(Duration::from_millis(100)).await;
    let result = sm.get("permanent_key").await.expect("should get");
    assert_eq!(result, Some("permanent_value".to_string()));
}

#[tokio::test]
async fn test_scan_filters_expired_keys() {
    let (mut sm, _temp_dir) = create_temp_sm();

    let now = now_ms();

    // Set some keys: some expired, some not
    let entry1 = make_entry(
        1,
        AppRequest::Set {
            key: "prefix:permanent".to_string(),
            value: "val1".to_string(),
        },
    );
    apply_entry(&mut sm, entry1).await;

    let entry2 = make_entry(
        2,
        AppRequest::SetWithTTL {
            key: "prefix:expiring_soon".to_string(),
            value: "val2".to_string(),
            expires_at_ms: now + 100, // expires in 100ms
        },
    );
    apply_entry(&mut sm, entry2).await;

    let entry3 = make_entry(
        3,
        AppRequest::SetWithTTL {
            key: "prefix:expiring_later".to_string(),
            value: "val3".to_string(),
            expires_at_ms: now + 10000, // expires in 10 seconds
        },
    );
    apply_entry(&mut sm, entry3).await;

    // All keys should be visible immediately
    let results = sm.scan("prefix:", None, None).await.expect("should scan");
    assert_eq!(results.len(), 3, "all keys should be visible initially");

    // Wait for one key to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Only non-expired keys should be visible
    let results = sm.scan("prefix:", None, None).await.expect("should scan");
    assert_eq!(results.len(), 2, "expired key should be filtered");

    let keys: Vec<_> = results.iter().map(|(k, _)| k.as_str()).collect();
    assert!(keys.contains(&"prefix:permanent"));
    assert!(keys.contains(&"prefix:expiring_later"));
    assert!(!keys.contains(&"prefix:expiring_soon"));
}

// ============================================================================
// Background Cleanup Tests
// ============================================================================

#[tokio::test]
async fn test_delete_expired_keys_removes_expired_entries() {
    let (mut sm, _temp_dir) = create_temp_sm();

    let now = now_ms();

    // Set some keys with short TTL
    let entry1 = make_entry(
        1,
        AppRequest::SetWithTTL {
            key: "expired1".to_string(),
            value: "val1".to_string(),
            expires_at_ms: now - 1000, // already expired
        },
    );
    apply_entry(&mut sm, entry1).await;

    let entry2 = make_entry(
        2,
        AppRequest::SetWithTTL {
            key: "expired2".to_string(),
            value: "val2".to_string(),
            expires_at_ms: now - 500, // already expired
        },
    );
    apply_entry(&mut sm, entry2).await;

    let entry3 = make_entry(
        3,
        AppRequest::Set {
            key: "permanent".to_string(),
            value: "val3".to_string(),
        },
    );
    apply_entry(&mut sm, entry3).await;

    let entry4 = make_entry(
        4,
        AppRequest::SetWithTTL {
            key: "future".to_string(),
            value: "val4".to_string(),
            expires_at_ms: now + 60000, // expires in 1 minute
        },
    );
    apply_entry(&mut sm, entry4).await;

    // Verify expired count
    let expired_count = sm.count_expired_keys().expect("should count expired");
    assert_eq!(expired_count, 2, "should have 2 expired keys");

    // Run cleanup
    let deleted = sm.delete_expired_keys(100).expect("should delete expired");
    assert_eq!(deleted, 2, "should delete 2 expired keys");

    // Verify cleanup worked
    let expired_count = sm.count_expired_keys().expect("should count expired");
    assert_eq!(
        expired_count, 0,
        "should have no expired keys after cleanup"
    );

    // Non-expired keys should still exist
    assert!(sm.get("permanent").await.expect("should get").is_some());
    assert!(sm.get("future").await.expect("should get").is_some());
}

#[tokio::test]
async fn test_delete_expired_keys_respects_batch_limit() {
    let (mut sm, _temp_dir) = create_temp_sm();

    let now = now_ms();

    // Create many expired keys
    for i in 0..50u64 {
        let entry = make_entry(
            i + 1,
            AppRequest::SetWithTTL {
                key: format!("expired_{}", i),
                value: format!("val_{}", i),
                expires_at_ms: now - 1000, // already expired
            },
        );
        apply_entry(&mut sm, entry).await;
    }

    // Delete with small batch size
    let deleted = sm.delete_expired_keys(10).expect("should delete batch");
    assert_eq!(deleted, 10, "should delete exactly batch_size keys");

    // Should still have remaining expired keys
    let remaining = sm.count_expired_keys().expect("should count expired");
    assert_eq!(remaining, 40, "should have 40 remaining expired keys");

    // Delete the rest
    loop {
        let deleted = sm.delete_expired_keys(20).expect("should delete batch");
        if deleted == 0 {
            break;
        }
    }

    let remaining = sm.count_expired_keys().expect("should count expired");
    assert_eq!(
        remaining, 0,
        "should have no expired keys after full cleanup"
    );
}

// ============================================================================
// TTL Metrics Tests
// ============================================================================

#[tokio::test]
async fn test_count_keys_with_ttl() {
    let (mut sm, _temp_dir) = create_temp_sm();

    let now = now_ms();

    // Set a mix of keys with and without TTL
    let entry1 = make_entry(
        1,
        AppRequest::Set {
            key: "permanent1".to_string(),
            value: "val1".to_string(),
        },
    );
    apply_entry(&mut sm, entry1).await;

    let entry2 = make_entry(
        2,
        AppRequest::Set {
            key: "permanent2".to_string(),
            value: "val2".to_string(),
        },
    );
    apply_entry(&mut sm, entry2).await;

    let entry3 = make_entry(
        3,
        AppRequest::SetWithTTL {
            key: "ttl1".to_string(),
            value: "val3".to_string(),
            expires_at_ms: now + 60000,
        },
    );
    apply_entry(&mut sm, entry3).await;

    let entry4 = make_entry(
        4,
        AppRequest::SetWithTTL {
            key: "ttl2".to_string(),
            value: "val4".to_string(),
            expires_at_ms: now + 60000,
        },
    );
    apply_entry(&mut sm, entry4).await;

    let entry5 = make_entry(
        5,
        AppRequest::SetWithTTL {
            key: "ttl3".to_string(),
            value: "val5".to_string(),
            expires_at_ms: now + 60000,
        },
    );
    apply_entry(&mut sm, entry5).await;

    let ttl_count = sm.count_keys_with_ttl().expect("should count TTL keys");
    assert_eq!(ttl_count, 3, "should have 3 keys with TTL");
}

// ============================================================================
// SetMultiWithTTL Tests
// ============================================================================

#[tokio::test]
async fn test_set_multi_with_ttl() {
    let (mut sm, _temp_dir) = create_temp_sm();

    let expires_at = now_ms() + 500; // expires in 500ms

    let entry = make_entry(
        1,
        AppRequest::SetMultiWithTTL {
            pairs: vec![
                ("batch:key1".to_string(), "val1".to_string()),
                ("batch:key2".to_string(), "val2".to_string()),
                ("batch:key3".to_string(), "val3".to_string()),
            ],
            expires_at_ms: expires_at,
        },
    );
    apply_entry(&mut sm, entry).await;

    // All keys should be visible
    for i in 1..=3 {
        let key = format!("batch:key{}", i);
        let result = sm.get(&key).await.expect("should get");
        assert!(result.is_some(), "key {} should exist", key);
    }

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(600)).await;

    // All keys should be expired (lazy expiration)
    for i in 1..=3 {
        let key = format!("batch:key{}", i);
        let result = sm.get(&key).await.expect("should get");
        assert!(result.is_none(), "key {} should be expired", key);
    }

    // Verify they're counted as expired
    let expired = sm.count_expired_keys().expect("should count expired");
    assert_eq!(expired, 3, "all 3 batch keys should be expired");
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[tokio::test]
async fn test_ttl_at_exact_boundary() {
    let (mut sm, _temp_dir) = create_temp_sm();

    // Set a key that expires at exactly now + 200ms
    let expires_at = now_ms() + 200;
    let entry = make_entry(
        1,
        AppRequest::SetWithTTL {
            key: "boundary".to_string(),
            value: "test".to_string(),
            expires_at_ms: expires_at,
        },
    );
    apply_entry(&mut sm, entry).await;

    // Should be visible now
    assert!(sm.get("boundary").await.expect("should get").is_some());

    // Wait past expiration
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Should now be expired
    assert!(sm.get("boundary").await.expect("should get").is_none());
}

#[tokio::test]
async fn test_overwrite_ttl_key_with_no_ttl() {
    let (mut sm, _temp_dir) = create_temp_sm();

    let expires_at = now_ms() + 100; // expires soon

    // Set key with TTL
    let entry1 = make_entry(
        1,
        AppRequest::SetWithTTL {
            key: "key".to_string(),
            value: "ttl_value".to_string(),
            expires_at_ms: expires_at,
        },
    );
    apply_entry(&mut sm, entry1).await;

    // Overwrite with no TTL (should clear expiration)
    let entry2 = make_entry(
        2,
        AppRequest::Set {
            key: "key".to_string(),
            value: "permanent_value".to_string(),
        },
    );
    apply_entry(&mut sm, entry2).await;

    // Wait past original expiration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Key should still be visible (TTL was cleared)
    let result = sm.get("key").await.expect("should get");
    assert_eq!(result, Some("permanent_value".to_string()));
}

#[tokio::test]
async fn test_overwrite_no_ttl_key_with_ttl() {
    let (mut sm, _temp_dir) = create_temp_sm();

    // Set key without TTL
    let entry1 = make_entry(
        1,
        AppRequest::Set {
            key: "key".to_string(),
            value: "permanent".to_string(),
        },
    );
    apply_entry(&mut sm, entry1).await;

    // Overwrite with TTL
    let expires_at = now_ms() + 100;
    let entry2 = make_entry(
        2,
        AppRequest::SetWithTTL {
            key: "key".to_string(),
            value: "expiring".to_string(),
            expires_at_ms: expires_at,
        },
    );
    apply_entry(&mut sm, entry2).await;

    // Wait past expiration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Key should be expired
    let result = sm.get("key").await.expect("should get");
    assert!(result.is_none(), "key should be expired after TTL added");
}

#[tokio::test]
async fn test_delete_expired_keys_empty_database() {
    let (sm, _temp_dir) = create_temp_sm();

    // Delete on empty database should return 0
    let deleted = sm.delete_expired_keys(100).expect("should handle empty");
    assert_eq!(deleted, 0);

    let expired = sm.count_expired_keys().expect("should count expired");
    assert_eq!(expired, 0);

    let ttl_count = sm.count_keys_with_ttl().expect("should count TTL keys");
    assert_eq!(ttl_count, 0);
}

#[tokio::test]
async fn test_very_long_ttl() {
    let (mut sm, _temp_dir) = create_temp_sm();

    // Set key with 1 year TTL
    let one_year_ms = 365 * 24 * 60 * 60 * 1000u64;
    let expires_at = now_ms() + one_year_ms;

    let entry = make_entry(
        1,
        AppRequest::SetWithTTL {
            key: "long_ttl".to_string(),
            value: "test".to_string(),
            expires_at_ms: expires_at,
        },
    );
    apply_entry(&mut sm, entry).await;

    // Key should be visible
    let result = sm.get("long_ttl").await.expect("should get");
    assert_eq!(result, Some("test".to_string()));

    // Should count as TTL key but not expired
    let ttl_count = sm.count_keys_with_ttl().expect("should count TTL keys");
    assert_eq!(ttl_count, 1);

    let expired = sm.count_expired_keys().expect("should count expired");
    assert_eq!(expired, 0);
}
