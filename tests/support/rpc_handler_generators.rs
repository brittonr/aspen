//! RPC handler-specific generators for property-based and chaos testing.
//!
//! Provides comprehensive generators for:
//! - Coordination primitives (locks, counters, barriers, queues, rate limiters, semaphores,
//!   RWLocks)
//! - KV operations (read, write, delete, scan, batch, CAS)
//! - Operation sequences for stateful testing
//! - Fault injection scenarios for chaos testing
//!
//! # Usage
//!
//! ```ignore
//! use proptest::prelude::*;
//! use support::rpc_handler_generators::*;
//!
//! proptest! {
//!     #[test]
//!     fn test_lock_operations(ops in arbitrary_lock_operation_sequence(50)) {
//!         // Test lock invariants across operation sequence
//!     }
//! }
//! ```
//!
//! # Tiger Style
//!
//! - All generators respect resource bounds from `aspen::raft::constants`
//! - Bounded operation sequences prevent memory exhaustion
//! - Deterministic generation for reproducible tests

#![allow(dead_code)] // Generators may not all be used initially

use proptest::prelude::*;

// =============================================================================
// Coordination Types
// =============================================================================

/// Lock operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum LockOperation {
    /// Attempt to acquire a lock (blocking with timeout).
    Acquire {
        key: String,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    },
    /// Try to acquire a lock (non-blocking).
    TryAcquire {
        key: String,
        holder_id: String,
        ttl_ms: u64,
    },
    /// Release a held lock.
    Release {
        key: String,
        holder_id: String,
        fencing_token: u64,
    },
    /// Renew a lock's TTL.
    Renew {
        key: String,
        holder_id: String,
        fencing_token: u64,
        new_ttl_ms: u64,
    },
}

/// Counter operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum CounterOperation {
    /// Get current counter value.
    Get { key: String },
    /// Increment counter by 1.
    Increment { key: String },
    /// Decrement counter by 1.
    Decrement { key: String },
    /// Add amount to counter.
    Add { key: String, amount: u64 },
    /// Subtract amount from counter.
    Subtract { key: String, amount: u64 },
    /// Set counter to specific value.
    Set { key: String, value: u64 },
    /// Compare-and-set counter.
    CompareAndSet { key: String, expected: u64, new_value: u64 },
}

/// Signed counter operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum SignedCounterOperation {
    /// Get current signed counter value.
    Get { key: String },
    /// Add (positive or negative) to signed counter.
    Add { key: String, delta: i64 },
}

/// Queue operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum QueueOperation {
    /// Create a new queue.
    Create {
        name: String,
        visibility_timeout_ms: u64,
        max_delivery_attempts: u32,
    },
    /// Delete a queue.
    Delete { name: String },
    /// Enqueue an item.
    Enqueue {
        queue_name: String,
        payload: Vec<u8>,
        ttl_ms: Option<u64>,
        dedup_id: Option<String>,
    },
    /// Dequeue items.
    Dequeue { queue_name: String, max_items: u32 },
    /// Acknowledge an item.
    Ack { queue_name: String, receipt_handle: String },
    /// Negative-acknowledge an item.
    Nack { queue_name: String, receipt_handle: String },
    /// Peek at queue items without dequeuing.
    Peek { queue_name: String, max_items: u32 },
    /// Get queue status.
    Status { queue_name: String },
}

/// Barrier operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum BarrierOperation {
    /// Enter a barrier.
    Enter {
        name: String,
        participant_id: String,
        required_count: u32,
        timeout_ms: u64,
    },
    /// Leave a barrier.
    Leave {
        name: String,
        participant_id: String,
        timeout_ms: u64,
    },
    /// Get barrier status.
    Status { name: String },
}

/// Semaphore operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum SemaphoreOperation {
    /// Acquire permits (blocking).
    Acquire {
        name: String,
        holder_id: String,
        permits: u32,
        timeout_ms: u64,
    },
    /// Try to acquire permits (non-blocking).
    TryAcquire {
        name: String,
        holder_id: String,
        permits: u32,
    },
    /// Release permits.
    Release {
        name: String,
        holder_id: String,
        permits: u32,
    },
    /// Get semaphore status.
    Status { name: String },
}

/// RWLock operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum RWLockOperation {
    /// Acquire read lock.
    AcquireRead {
        name: String,
        holder_id: String,
        timeout_ms: u64,
    },
    /// Try to acquire read lock.
    TryAcquireRead { name: String, holder_id: String },
    /// Acquire write lock.
    AcquireWrite {
        name: String,
        holder_id: String,
        timeout_ms: u64,
    },
    /// Try to acquire write lock.
    TryAcquireWrite { name: String, holder_id: String },
    /// Release read lock.
    ReleaseRead { name: String, holder_id: String },
    /// Release write lock.
    ReleaseWrite { name: String, holder_id: String },
    /// Downgrade write lock to read lock.
    Downgrade { name: String, holder_id: String },
    /// Get RWLock status.
    Status { name: String },
}

/// Rate limiter operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimiterOperation {
    /// Try to acquire tokens (non-blocking).
    TryAcquire { name: String, tokens: u64 },
    /// Acquire tokens (blocking with timeout).
    Acquire { name: String, tokens: u64, timeout_ms: u64 },
    /// Check available tokens.
    Available { name: String },
    /// Reset the rate limiter.
    Reset { name: String },
}

// =============================================================================
// KV Types
// =============================================================================

/// KV operation for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum KvOperation {
    /// Read a key.
    Read { key: String },
    /// Write a key-value pair.
    Write { key: String, value: String },
    /// Delete a key.
    Delete { key: String },
    /// Scan keys with prefix.
    Scan {
        prefix: String,
        limit: u32,
        continuation_token: Option<String>,
    },
    /// Batch read multiple keys.
    BatchRead { keys: Vec<String> },
    /// Batch write multiple key-value pairs.
    BatchWrite { pairs: Vec<(String, String)> },
    /// Compare-and-swap operation.
    CompareAndSwap {
        key: String,
        expected: Option<String>,
        new_value: String,
    },
    /// Compare-and-delete operation.
    CompareAndDelete { key: String, expected: String },
}

// =============================================================================
// Fault Injection Types
// =============================================================================

/// Fault scenario for chaos testing.
#[derive(Debug, Clone, PartialEq)]
pub enum FaultScenario {
    /// Network partition (node isolated).
    Partition { node: usize, duration_secs: u64 },
    /// Node crash and recovery.
    Crash { node: usize, recovery_after_secs: u64 },
    /// Clock skew on a node.
    ClockSkew { node: usize, skew_ms: i64 },
    /// Network delay injection.
    Delay { min_ms: u64, max_ms: u64 },
    /// Packet loss percentage.
    PacketLoss { rate: f64 },
    /// Leader stepdown (force election).
    LeaderStepdown,
}

// =============================================================================
// Core Generator Helpers
// =============================================================================

/// Generate a valid resource key (for locks, counters, queues, etc.).
pub fn arbitrary_resource_key() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{1,20}"
}

/// Generate a valid holder ID.
pub fn arbitrary_holder_id() -> impl Strategy<Value = String> {
    "[a-z]{1,10}"
}

/// Generate a valid TTL in milliseconds.
pub fn arbitrary_ttl_ms() -> impl Strategy<Value = u64> {
    1000u64..60000u64 // 1s to 60s
}

/// Generate a valid timeout in milliseconds.
pub fn arbitrary_timeout_ms() -> impl Strategy<Value = u64> {
    0u64..30000u64 // 0 to 30s
}

/// Generate a valid participant ID.
pub fn arbitrary_participant_id() -> impl Strategy<Value = String> {
    "[a-z]{3,10}"
}

/// Generate a valid queue name.
pub fn arbitrary_queue_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{2,20}"
}

/// Generate a valid payload for queue items.
pub fn arbitrary_queue_payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..1000)
}

/// Generate a valid KV key.
pub fn arbitrary_kv_key() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,50}"
}

/// Generate a valid KV value.
pub fn arbitrary_kv_value() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..500).prop_map(|v| v.into_iter().collect())
}

// =============================================================================
// Lock Generators
// =============================================================================

/// Generate a lock acquire operation.
pub fn arbitrary_lock_acquire() -> impl Strategy<Value = LockOperation> {
    (arbitrary_resource_key(), arbitrary_holder_id(), arbitrary_ttl_ms(), arbitrary_timeout_ms()).prop_map(
        |(key, holder_id, ttl_ms, timeout_ms)| LockOperation::Acquire {
            key,
            holder_id,
            ttl_ms,
            timeout_ms,
        },
    )
}

/// Generate a lock try-acquire operation.
pub fn arbitrary_lock_try_acquire() -> impl Strategy<Value = LockOperation> {
    (arbitrary_resource_key(), arbitrary_holder_id(), arbitrary_ttl_ms())
        .prop_map(|(key, holder_id, ttl_ms)| LockOperation::TryAcquire { key, holder_id, ttl_ms })
}

/// Generate a lock release operation.
pub fn arbitrary_lock_release() -> impl Strategy<Value = LockOperation> {
    (arbitrary_resource_key(), arbitrary_holder_id(), 1u64..1000u64).prop_map(|(key, holder_id, fencing_token)| {
        LockOperation::Release {
            key,
            holder_id,
            fencing_token,
        }
    })
}

/// Generate a lock renew operation.
pub fn arbitrary_lock_renew() -> impl Strategy<Value = LockOperation> {
    (arbitrary_resource_key(), arbitrary_holder_id(), 1u64..1000u64, arbitrary_ttl_ms()).prop_map(
        |(key, holder_id, fencing_token, new_ttl_ms)| LockOperation::Renew {
            key,
            holder_id,
            fencing_token,
            new_ttl_ms,
        },
    )
}

/// Generate any lock operation.
pub fn arbitrary_lock_operation() -> impl Strategy<Value = LockOperation> {
    prop_oneof![
        arbitrary_lock_acquire(),
        arbitrary_lock_try_acquire(),
        arbitrary_lock_release(),
        arbitrary_lock_renew(),
    ]
}

/// Generate a sequence of lock operations for stateful testing.
pub fn arbitrary_lock_operation_sequence(max_len: usize) -> impl Strategy<Value = Vec<LockOperation>> {
    prop::collection::vec(arbitrary_lock_operation(), 1..max_len)
}

// =============================================================================
// Counter Generators
// =============================================================================

/// Generate any counter operation.
pub fn arbitrary_counter_operation() -> impl Strategy<Value = CounterOperation> {
    prop_oneof![
        arbitrary_resource_key().prop_map(|key| CounterOperation::Get { key }),
        arbitrary_resource_key().prop_map(|key| CounterOperation::Increment { key }),
        arbitrary_resource_key().prop_map(|key| CounterOperation::Decrement { key }),
        (arbitrary_resource_key(), 1u64..1000u64).prop_map(|(key, amount)| CounterOperation::Add { key, amount }),
        (arbitrary_resource_key(), 1u64..1000u64).prop_map(|(key, amount)| CounterOperation::Subtract { key, amount }),
        (arbitrary_resource_key(), 0u64..u64::MAX / 2).prop_map(|(key, value)| CounterOperation::Set { key, value }),
        (arbitrary_resource_key(), 0u64..1000u64, 0u64..1000u64).prop_map(|(key, expected, new_value)| {
            CounterOperation::CompareAndSet {
                key,
                expected,
                new_value,
            }
        }),
    ]
}

/// Generate a sequence of counter operations.
pub fn arbitrary_counter_operation_sequence(max_len: usize) -> impl Strategy<Value = Vec<CounterOperation>> {
    prop::collection::vec(arbitrary_counter_operation(), 1..max_len)
}

/// Generate signed counter operations.
pub fn arbitrary_signed_counter_operation() -> impl Strategy<Value = SignedCounterOperation> {
    prop_oneof![
        arbitrary_resource_key().prop_map(|key| SignedCounterOperation::Get { key }),
        (arbitrary_resource_key(), -1000i64..1000i64)
            .prop_map(|(key, delta)| SignedCounterOperation::Add { key, delta }),
    ]
}

// =============================================================================
// Queue Generators
// =============================================================================

/// Generate a queue create operation.
pub fn arbitrary_queue_create() -> impl Strategy<Value = QueueOperation> {
    (
        arbitrary_queue_name(),
        1000u64..60000u64, // visibility_timeout: 1s to 60s
        1u32..10u32,       // max_delivery_attempts: 1 to 10
    )
        .prop_map(|(name, visibility_timeout_ms, max_delivery_attempts)| QueueOperation::Create {
            name,
            visibility_timeout_ms,
            max_delivery_attempts,
        })
}

/// Generate a queue enqueue operation.
pub fn arbitrary_queue_enqueue() -> impl Strategy<Value = QueueOperation> {
    (
        arbitrary_queue_name(),
        arbitrary_queue_payload(),
        prop::option::of(1000u64..3600000u64), // ttl: 1s to 1 hour
        prop::option::of("[a-z0-9]{1,32}"),    // dedup_id
    )
        .prop_map(|(queue_name, payload, ttl_ms, dedup_id)| QueueOperation::Enqueue {
            queue_name,
            payload,
            ttl_ms,
            dedup_id,
        })
}

/// Generate a queue dequeue operation.
pub fn arbitrary_queue_dequeue() -> impl Strategy<Value = QueueOperation> {
    (arbitrary_queue_name(), 1u32..100u32)
        .prop_map(|(queue_name, max_items)| QueueOperation::Dequeue { queue_name, max_items })
}

/// Generate any queue operation.
pub fn arbitrary_queue_operation() -> impl Strategy<Value = QueueOperation> {
    prop_oneof![
        arbitrary_queue_create(),
        arbitrary_queue_name().prop_map(|name| QueueOperation::Delete { name }),
        arbitrary_queue_enqueue(),
        arbitrary_queue_dequeue(),
        (arbitrary_queue_name(), "[a-z0-9]{32}").prop_map(|(queue_name, receipt_handle)| QueueOperation::Ack {
            queue_name,
            receipt_handle: receipt_handle.to_string(),
        }),
        (arbitrary_queue_name(), "[a-z0-9]{32}").prop_map(|(queue_name, receipt_handle)| QueueOperation::Nack {
            queue_name,
            receipt_handle: receipt_handle.to_string(),
        }),
        (arbitrary_queue_name(), 1u32..100u32)
            .prop_map(|(queue_name, max_items)| QueueOperation::Peek { queue_name, max_items }),
        arbitrary_queue_name().prop_map(|queue_name| QueueOperation::Status { queue_name }),
    ]
}

/// Generate a sequence of queue operations.
pub fn arbitrary_queue_operation_sequence(max_len: usize) -> impl Strategy<Value = Vec<QueueOperation>> {
    prop::collection::vec(arbitrary_queue_operation(), 1..max_len)
}

// =============================================================================
// Barrier Generators
// =============================================================================

/// Generate any barrier operation.
pub fn arbitrary_barrier_operation() -> impl Strategy<Value = BarrierOperation> {
    prop_oneof![
        (arbitrary_resource_key(), arbitrary_participant_id(), 2u32..10u32, arbitrary_timeout_ms()).prop_map(
            |(name, participant_id, required_count, timeout_ms)| {
                BarrierOperation::Enter {
                    name,
                    participant_id,
                    required_count,
                    timeout_ms,
                }
            }
        ),
        (arbitrary_resource_key(), arbitrary_participant_id(), arbitrary_timeout_ms()).prop_map(
            |(name, participant_id, timeout_ms)| BarrierOperation::Leave {
                name,
                participant_id,
                timeout_ms,
            }
        ),
        arbitrary_resource_key().prop_map(|name| BarrierOperation::Status { name }),
    ]
}

// =============================================================================
// Semaphore Generators
// =============================================================================

/// Generate any semaphore operation.
pub fn arbitrary_semaphore_operation() -> impl Strategy<Value = SemaphoreOperation> {
    prop_oneof![
        (arbitrary_resource_key(), arbitrary_holder_id(), 1u32..10u32, arbitrary_timeout_ms()).prop_map(
            |(name, holder_id, permits, timeout_ms)| SemaphoreOperation::Acquire {
                name,
                holder_id,
                permits,
                timeout_ms,
            }
        ),
        (arbitrary_resource_key(), arbitrary_holder_id(), 1u32..10u32).prop_map(|(name, holder_id, permits)| {
            SemaphoreOperation::TryAcquire {
                name,
                holder_id,
                permits,
            }
        }),
        (arbitrary_resource_key(), arbitrary_holder_id(), 1u32..10u32).prop_map(|(name, holder_id, permits)| {
            SemaphoreOperation::Release {
                name,
                holder_id,
                permits,
            }
        }),
        arbitrary_resource_key().prop_map(|name| SemaphoreOperation::Status { name }),
    ]
}

// =============================================================================
// RWLock Generators
// =============================================================================

/// Generate any RWLock operation.
pub fn arbitrary_rwlock_operation() -> impl Strategy<Value = RWLockOperation> {
    prop_oneof![
        (arbitrary_resource_key(), arbitrary_holder_id(), arbitrary_timeout_ms()).prop_map(
            |(name, holder_id, timeout_ms)| RWLockOperation::AcquireRead {
                name,
                holder_id,
                timeout_ms,
            }
        ),
        (arbitrary_resource_key(), arbitrary_holder_id())
            .prop_map(|(name, holder_id)| RWLockOperation::TryAcquireRead { name, holder_id }),
        (arbitrary_resource_key(), arbitrary_holder_id(), arbitrary_timeout_ms()).prop_map(
            |(name, holder_id, timeout_ms)| RWLockOperation::AcquireWrite {
                name,
                holder_id,
                timeout_ms,
            }
        ),
        (arbitrary_resource_key(), arbitrary_holder_id())
            .prop_map(|(name, holder_id)| RWLockOperation::TryAcquireWrite { name, holder_id }),
        (arbitrary_resource_key(), arbitrary_holder_id())
            .prop_map(|(name, holder_id)| RWLockOperation::ReleaseRead { name, holder_id }),
        (arbitrary_resource_key(), arbitrary_holder_id())
            .prop_map(|(name, holder_id)| RWLockOperation::ReleaseWrite { name, holder_id }),
        (arbitrary_resource_key(), arbitrary_holder_id())
            .prop_map(|(name, holder_id)| RWLockOperation::Downgrade { name, holder_id }),
        arbitrary_resource_key().prop_map(|name| RWLockOperation::Status { name }),
    ]
}

// =============================================================================
// Rate Limiter Generators
// =============================================================================

/// Generate any rate limiter operation.
pub fn arbitrary_rate_limiter_operation() -> impl Strategy<Value = RateLimiterOperation> {
    prop_oneof![
        (arbitrary_resource_key(), 1u64..100u64)
            .prop_map(|(name, tokens)| RateLimiterOperation::TryAcquire { name, tokens }),
        (arbitrary_resource_key(), 1u64..100u64, arbitrary_timeout_ms()).prop_map(|(name, tokens, timeout_ms)| {
            RateLimiterOperation::Acquire {
                name,
                tokens,
                timeout_ms,
            }
        }),
        arbitrary_resource_key().prop_map(|name| RateLimiterOperation::Available { name }),
        arbitrary_resource_key().prop_map(|name| RateLimiterOperation::Reset { name }),
    ]
}

// =============================================================================
// KV Generators
// =============================================================================

/// Generate a KV read operation.
pub fn arbitrary_kv_read() -> impl Strategy<Value = KvOperation> {
    arbitrary_kv_key().prop_map(|key| KvOperation::Read { key })
}

/// Generate a KV write operation.
pub fn arbitrary_kv_write() -> impl Strategy<Value = KvOperation> {
    (arbitrary_kv_key(), arbitrary_kv_value()).prop_map(|(key, value)| KvOperation::Write { key, value })
}

/// Generate a KV delete operation.
pub fn arbitrary_kv_delete() -> impl Strategy<Value = KvOperation> {
    arbitrary_kv_key().prop_map(|key| KvOperation::Delete { key })
}

/// Generate a KV scan operation.
pub fn arbitrary_kv_scan() -> impl Strategy<Value = KvOperation> {
    ("[a-z]{0,10}", 1u32..100u32, prop::option::of("[a-z0-9]{32}")).prop_map(|(prefix, limit, continuation_token)| {
        KvOperation::Scan {
            prefix: prefix.to_string(),
            limit,
            continuation_token,
        }
    })
}

/// Generate a KV batch read operation.
pub fn arbitrary_kv_batch_read() -> impl Strategy<Value = KvOperation> {
    prop::collection::vec(arbitrary_kv_key(), 1..50).prop_map(|keys| KvOperation::BatchRead { keys })
}

/// Generate a KV batch write operation.
pub fn arbitrary_kv_batch_write() -> impl Strategy<Value = KvOperation> {
    prop::collection::vec((arbitrary_kv_key(), arbitrary_kv_value()), 1..50)
        .prop_map(|pairs| KvOperation::BatchWrite { pairs })
}

/// Generate a KV compare-and-swap operation.
pub fn arbitrary_kv_cas() -> impl Strategy<Value = KvOperation> {
    (arbitrary_kv_key(), prop::option::of(arbitrary_kv_value()), arbitrary_kv_value()).prop_map(
        |(key, expected, new_value)| KvOperation::CompareAndSwap {
            key,
            expected,
            new_value,
        },
    )
}

/// Generate any KV operation.
pub fn arbitrary_kv_operation() -> impl Strategy<Value = KvOperation> {
    prop_oneof![
        arbitrary_kv_read(),
        arbitrary_kv_write(),
        arbitrary_kv_delete(),
        arbitrary_kv_scan(),
        arbitrary_kv_batch_read(),
        arbitrary_kv_batch_write(),
        arbitrary_kv_cas(),
        (arbitrary_kv_key(), arbitrary_kv_value())
            .prop_map(|(key, expected)| KvOperation::CompareAndDelete { key, expected }),
    ]
}

/// Generate a sequence of KV operations.
pub fn arbitrary_kv_operation_sequence(max_len: usize) -> impl Strategy<Value = Vec<KvOperation>> {
    prop::collection::vec(arbitrary_kv_operation(), 1..max_len)
}

// =============================================================================
// Fault Injection Generators
// =============================================================================

/// Generate a partition fault.
pub fn arbitrary_partition_fault(max_nodes: usize) -> impl Strategy<Value = FaultScenario> {
    (0..max_nodes, 1u64..10u64).prop_map(|(node, duration_secs)| FaultScenario::Partition { node, duration_secs })
}

/// Generate a crash fault.
pub fn arbitrary_crash_fault(max_nodes: usize) -> impl Strategy<Value = FaultScenario> {
    (0..max_nodes, 1u64..10u64).prop_map(|(node, recovery_after_secs)| FaultScenario::Crash {
        node,
        recovery_after_secs,
    })
}

/// Generate a clock skew fault.
pub fn arbitrary_clock_skew_fault(max_nodes: usize) -> impl Strategy<Value = FaultScenario> {
    (0..max_nodes, -500i64..500i64).prop_map(|(node, skew_ms)| FaultScenario::ClockSkew { node, skew_ms })
}

/// Generate any fault scenario.
pub fn arbitrary_fault_scenario(max_nodes: usize) -> impl Strategy<Value = FaultScenario> {
    prop_oneof![
        arbitrary_partition_fault(max_nodes),
        arbitrary_crash_fault(max_nodes),
        arbitrary_clock_skew_fault(max_nodes),
        (0u64..500u64)
            .prop_flat_map(|min| (Just(min), min..min + 200))
            .prop_map(|(min_ms, max_ms)| FaultScenario::Delay { min_ms, max_ms }),
        (0.0f64..0.5f64).prop_map(|rate| FaultScenario::PacketLoss { rate }),
        Just(FaultScenario::LeaderStepdown),
    ]
}

/// Generate a sequence of fault scenarios for chaos testing.
pub fn arbitrary_fault_sequence(max_nodes: usize, max_len: usize) -> impl Strategy<Value = Vec<FaultScenario>> {
    prop::collection::vec(arbitrary_fault_scenario(max_nodes), 1..max_len)
}

// =============================================================================
// Combined Operation Generators
// =============================================================================

/// Any coordination operation (for unified testing).
#[derive(Debug, Clone, PartialEq)]
pub enum CoordinationOperation {
    Lock(LockOperation),
    Counter(CounterOperation),
    Queue(QueueOperation),
    Barrier(BarrierOperation),
    Semaphore(SemaphoreOperation),
    RWLock(RWLockOperation),
    RateLimiter(RateLimiterOperation),
}

/// Generate any coordination operation.
pub fn arbitrary_coordination_operation() -> impl Strategy<Value = CoordinationOperation> {
    prop_oneof![
        arbitrary_lock_operation().prop_map(CoordinationOperation::Lock),
        arbitrary_counter_operation().prop_map(CoordinationOperation::Counter),
        arbitrary_queue_operation().prop_map(CoordinationOperation::Queue),
        arbitrary_barrier_operation().prop_map(CoordinationOperation::Barrier),
        arbitrary_semaphore_operation().prop_map(CoordinationOperation::Semaphore),
        arbitrary_rwlock_operation().prop_map(CoordinationOperation::RWLock),
        arbitrary_rate_limiter_operation().prop_map(CoordinationOperation::RateLimiter),
    ]
}

/// Generate a mixed sequence of coordination and KV operations.
pub fn arbitrary_mixed_operation_sequence(max_len: usize) -> impl Strategy<Value = Vec<MixedOperation>> {
    prop::collection::vec(
        prop_oneof![
            arbitrary_kv_operation().prop_map(MixedOperation::Kv),
            arbitrary_coordination_operation().prop_map(MixedOperation::Coordination),
        ],
        1..max_len,
    )
}

/// Mixed operation type for combined testing.
#[derive(Debug, Clone, PartialEq)]
pub enum MixedOperation {
    Kv(KvOperation),
    Coordination(CoordinationOperation),
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use proptest::proptest;

    use super::*;

    proptest! {
        #[test]
        fn test_lock_operations_valid(op in arbitrary_lock_operation()) {
            match &op {
                LockOperation::Acquire { key, holder_id, ttl_ms, timeout_ms } => {
                    assert!(!key.is_empty());
                    assert!(!holder_id.is_empty());
                    assert!(*ttl_ms >= 1000);
                    assert!(*timeout_ms <= 30000);
                }
                LockOperation::TryAcquire { key, holder_id, ttl_ms } => {
                    assert!(!key.is_empty());
                    assert!(!holder_id.is_empty());
                    assert!(*ttl_ms >= 1000);
                }
                LockOperation::Release { key, holder_id, fencing_token } => {
                    assert!(!key.is_empty());
                    assert!(!holder_id.is_empty());
                    assert!(*fencing_token >= 1);
                }
                LockOperation::Renew { key, holder_id, fencing_token, new_ttl_ms } => {
                    assert!(!key.is_empty());
                    assert!(!holder_id.is_empty());
                    assert!(*fencing_token >= 1);
                    assert!(*new_ttl_ms >= 1000);
                }
            }
        }

        #[test]
        fn test_kv_operations_valid(op in arbitrary_kv_operation()) {
            match &op {
                KvOperation::Read { key } => {
                    assert!(!key.is_empty() || key.is_empty()); // Empty prefix is valid for scan
                }
                KvOperation::Write { key, value: _ } => {
                    assert!(!key.is_empty());
                }
                KvOperation::Delete { key } => {
                    assert!(!key.is_empty());
                }
                KvOperation::Scan { limit, .. } => {
                    assert!(*limit >= 1);
                    assert!(*limit <= 100);
                }
                KvOperation::BatchRead { keys } => {
                    assert!(!keys.is_empty());
                    assert!(keys.len() <= 50);
                }
                KvOperation::BatchWrite { pairs } => {
                    assert!(!pairs.is_empty());
                    assert!(pairs.len() <= 50);
                }
                KvOperation::CompareAndSwap { key, .. } => {
                    assert!(!key.is_empty());
                }
                KvOperation::CompareAndDelete { key, .. } => {
                    assert!(!key.is_empty());
                }
            }
        }

        #[test]
        fn test_fault_scenarios_valid(fault in arbitrary_fault_scenario(5)) {
            match &fault {
                FaultScenario::Partition { node, duration_secs } => {
                    assert!(*node < 5);
                    assert!(*duration_secs >= 1);
                    assert!(*duration_secs <= 10);
                }
                FaultScenario::Crash { node, recovery_after_secs } => {
                    assert!(*node < 5);
                    assert!(*recovery_after_secs >= 1);
                }
                FaultScenario::ClockSkew { node, skew_ms } => {
                    assert!(*node < 5);
                    assert!(*skew_ms >= -500);
                    assert!(*skew_ms <= 500);
                }
                FaultScenario::Delay { min_ms, max_ms } => {
                    assert!(*max_ms >= *min_ms);
                }
                FaultScenario::PacketLoss { rate } => {
                    assert!(*rate >= 0.0);
                    assert!(*rate <= 0.5);
                }
                FaultScenario::LeaderStepdown => {}
            }
        }
    }
}
