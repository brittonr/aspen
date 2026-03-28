//! Error injection tests for coordination primitives.
//!
//! Uses `FailingKeyValueStore` to verify error handling paths:
//! - Rate limiter fail-open on NotLeader (napkin 2026-02-26)
//! - Rate limiter returns StorageUnavailable on real failures
//! - Counter propagates storage errors
//! - Sequence handles storage failures during batch reservation
//! - Lock handles storage failures during acquire

use std::sync::Arc;

use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::DistributedLock;
use aspen_coordination::DistributedRateLimiter;
use aspen_coordination::LockConfig;
use aspen_coordination::RateLimiterConfig;
use aspen_coordination::SequenceConfig;
use aspen_coordination::SequenceGenerator;
use aspen_testing::DeterministicKeyValueStore;
use aspen_testing::FailingKeyValueStore;

// ---------------------------------------------------------------------------
// Rate limiter fail-open on NotLeader
// ---------------------------------------------------------------------------

/// Rate limiter must fail-open (allow the request) when the underlying store
/// returns NotLeader. This is the correct behavior for followers: the leader
/// manages the authoritative rate limit state, and followers should not block
/// requests just because they can't update the counter.
///
/// Regression: napkin 2026-02-26 — rate limiter previously fail-closed on
/// StorageUnavailable, blocking all requests on followers.
#[tokio::test]
async fn test_rate_limiter_fail_open_on_not_leader() {
    let inner = DeterministicKeyValueStore::new();

    // First, seed initial state so read succeeds
    let config = RateLimiterConfig::new(100.0, 100);
    let seeder = DistributedRateLimiter::new(inner.clone(), "rate:test", config.clone());
    seeder.try_acquire().await.unwrap();

    // Now wrap with failures on writes only (simulating follower: reads work, writes get NotLeader)
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_writes().await;
    failing.set_write_error("not leader; current leader: Some(2); forward").await;

    let limiter = DistributedRateLimiter::new(failing.clone() as Arc<_>, "rate:test", config);

    // Should succeed (fail-open) even though write fails with "not leader"
    // The rate limiter detects "forward" in the error message and allows the request
    let result = limiter.try_acquire().await;
    assert!(result.is_ok(), "Rate limiter should fail-open on NotLeader, got: {:?}", result);
}

/// Rate limiter must return StorageUnavailable (fail-closed) when the store
/// returns a genuine storage error (not NotLeader).
#[tokio::test]
async fn test_rate_limiter_fail_closed_on_storage_error() {
    let inner = DeterministicKeyValueStore::new();

    // Seed state
    let config = RateLimiterConfig::new(100.0, 100);
    let seeder = DistributedRateLimiter::new(inner.clone(), "rate:test2", config.clone());
    seeder.try_acquire().await.unwrap();

    // Fail all reads with a real storage error
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_reads().await;
    failing.set_read_error("disk I/O error").await;

    let limiter = DistributedRateLimiter::new(failing as Arc<_>, "rate:test2", config);

    let result = limiter.try_acquire().await;
    assert!(result.is_err(), "Rate limiter should fail-closed on I/O error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("disk I/O error")
            || err.to_string().contains("StorageUnavailable")
            || err.to_string().contains("injected fault"),
        "Error should mention storage issue, got: {}",
        err
    );
}

// ---------------------------------------------------------------------------
// Counter error propagation
// ---------------------------------------------------------------------------

/// Counter increment must propagate storage errors, not swallow them.
#[tokio::test]
async fn test_counter_propagates_write_errors() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_writes().await;

    let counter = AtomicCounter::new(failing as Arc<_>, "cnt:err", CounterConfig::default());

    let result = counter.increment().await;
    assert!(result.is_err(), "Counter should propagate write errors, got: {:?}", result);
}

/// Counter get must propagate read errors.
#[tokio::test]
async fn test_counter_propagates_read_errors() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_reads().await;

    let counter = AtomicCounter::new(failing as Arc<_>, "cnt:err2", CounterConfig::default());

    let result = counter.get().await;
    assert!(result.is_err(), "Counter should propagate read errors, got: {:?}", result);
}

/// Counter works normally after faults are cleared.
#[tokio::test]
async fn test_counter_recovers_after_fault_cleared() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);

    let counter = AtomicCounter::new(failing.clone() as Arc<_>, "cnt:recover", CounterConfig::default());

    // Normal operation
    counter.increment().await.unwrap();
    assert_eq!(counter.get().await.unwrap(), 1);

    // Inject fault
    failing.fail_all_writes().await;
    assert!(counter.increment().await.is_err());

    // Clear fault — should recover
    failing.clear_faults().await;
    counter.increment().await.unwrap();
    assert_eq!(counter.get().await.unwrap(), 2);
}

// ---------------------------------------------------------------------------
// Sequence error handling
// ---------------------------------------------------------------------------

/// Sequence must propagate errors during batch reservation.
#[tokio::test]
async fn test_sequence_propagates_write_errors() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_writes().await;

    let seq = SequenceGenerator::new(failing as Arc<_>, "seq:err", SequenceConfig {
        batch_size_ids: 1,
        start_value: 1,
    });

    let result = seq.next().await;
    assert!(result.is_err(), "Sequence should propagate write errors during batch reservation");
}

/// Sequence recovers after transient write failures.
#[tokio::test]
async fn test_sequence_recovers_after_transient_failure() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);

    let seq = SequenceGenerator::new(failing.clone() as Arc<_>, "seq:transient", SequenceConfig {
        batch_size_ids: 1,
        start_value: 1,
    });

    // Get first ID
    let id1 = seq.next().await.unwrap();
    assert_eq!(id1, 1);

    // Inject failure
    failing.fail_all_writes().await;
    assert!(seq.next().await.is_err());

    // Clear and recover
    failing.clear_faults().await;
    let id2 = seq.next().await.unwrap();
    // ID must be > id1 (monotonicity preserved across failures)
    assert!(id2 > id1, "Sequence must maintain monotonicity after recovery: {} should be > {}", id2, id1);
}

// ---------------------------------------------------------------------------
// Lock error handling
// ---------------------------------------------------------------------------

/// Lock acquire must propagate storage errors.
#[tokio::test]
async fn test_lock_propagates_write_errors() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_writes().await;

    let lock = DistributedLock::new(failing as Arc<_>, "lock:err", "holder-1", LockConfig {
        ttl_ms: 5000,
        acquire_timeout_ms: 1000,
        ..LockConfig::default()
    });

    let result = lock.try_acquire().await;
    assert!(result.is_err(), "Lock should propagate write errors");
}

/// Lock acquire must propagate read errors.
#[tokio::test]
async fn test_lock_propagates_read_errors() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);
    failing.fail_all_reads().await;

    let lock = DistributedLock::new(failing as Arc<_>, "lock:err2", "holder-1", LockConfig {
        ttl_ms: 5000,
        acquire_timeout_ms: 1000,
        ..LockConfig::default()
    });

    let result = lock.try_acquire().await;
    assert!(result.is_err(), "Lock should propagate read errors");
}

// ---------------------------------------------------------------------------
// Intermittent failure resilience
// ---------------------------------------------------------------------------

/// Counter should handle intermittent write failures via CAS retry.
/// fail_writes_every(3) means writes 1,2 succeed, write 3 fails, etc.
/// The CAS retry loop in counter should recover from occasional failures.
#[tokio::test]
async fn test_counter_survives_intermittent_failures() {
    let inner = DeterministicKeyValueStore::new();
    let failing = FailingKeyValueStore::new(inner);
    // Every 5th write fails
    failing.fail_writes_every(5).await;

    let counter = AtomicCounter::new(failing as Arc<_>, "cnt:flaky", CounterConfig::default());

    let mut successes = 0u32;
    let mut failures = 0u32;

    for _ in 0..20 {
        match counter.increment().await {
            Ok(_) => successes += 1,
            Err(_) => failures += 1,
        }
    }

    // Most increments should succeed (CAS retry handles some failures)
    assert!(
        successes > failures,
        "Most operations should succeed with intermittent failures: {successes} successes, {failures} failures"
    );

    // Final value should reflect successful increments
    let val = counter.get().await.unwrap();
    assert_eq!(val, successes as u64, "Counter value should match successful increments");
}
