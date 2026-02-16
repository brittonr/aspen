//! Coordination protocol types.
//!
//! Re-exports response types from aspen-coordination-protocol and defines
//! request enums for distributed coordination primitives.

pub use aspen_coordination_protocol::*;
use serde::Deserialize;
use serde::Serialize;

/// Coordination domain request (locks, counters, sequences, rate limiters,
/// barriers, semaphores, RW locks, queues, service registry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationRequest {
    // Lock operations
    /// Acquire a distributed lock with timeout.
    LockAcquire {
        /// Lock key (unique identifier for this lock).
        key: String,
        /// Holder ID (unique identifier for this lock holder).
        holder_id: String,
        /// Lock TTL in milliseconds.
        ttl_ms: u64,
        /// Acquire timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Try to acquire a distributed lock without blocking.
    LockTryAcquire {
        /// Lock key.
        key: String,
        /// Holder ID.
        holder_id: String,
        /// Lock TTL in milliseconds.
        ttl_ms: u64,
    },
    /// Release a distributed lock.
    LockRelease {
        /// Lock key.
        key: String,
        /// Holder ID that acquired the lock.
        holder_id: String,
        /// Fencing token from acquire operation.
        fencing_token: u64,
    },
    /// Renew a distributed lock's TTL.
    LockRenew {
        /// Lock key.
        key: String,
        /// Holder ID.
        holder_id: String,
        /// Fencing token from acquire operation.
        fencing_token: u64,
        /// New TTL in milliseconds.
        ttl_ms: u64,
    },

    // Counter operations
    /// Get the current value of an atomic counter.
    CounterGet { key: String },
    /// Increment an atomic counter by 1.
    CounterIncrement { key: String },
    /// Decrement an atomic counter by 1 (saturates at 0).
    CounterDecrement { key: String },
    /// Add an amount to an atomic counter.
    CounterAdd { key: String, amount: u64 },
    /// Subtract an amount from an atomic counter (saturates at 0).
    CounterSubtract { key: String, amount: u64 },
    /// Set an atomic counter to a specific value.
    CounterSet { key: String, value: u64 },
    /// Compare-and-set an atomic counter.
    CounterCompareAndSet { key: String, expected: u64, new_value: u64 },

    // Signed counter operations
    /// Get the current value of a signed atomic counter.
    SignedCounterGet { key: String },
    /// Add an amount to a signed atomic counter (can be negative).
    SignedCounterAdd { key: String, amount: i64 },

    // Sequence operations
    /// Get the next unique ID from a sequence.
    SequenceNext { key: String },
    /// Reserve a range of IDs from a sequence.
    SequenceReserve { key: String, count: u64 },
    /// Get the current (next available) value of a sequence without consuming it.
    SequenceCurrent { key: String },

    // Rate limiter operations
    /// Try to acquire tokens from a rate limiter without blocking.
    RateLimiterTryAcquire {
        key: String,
        tokens: u64,
        capacity_tokens: u64,
        refill_rate: f64,
    },
    /// Acquire tokens from a rate limiter with timeout.
    RateLimiterAcquire {
        key: String,
        tokens: u64,
        capacity_tokens: u64,
        refill_rate: f64,
        timeout_ms: u64,
    },
    /// Check available tokens in a rate limiter without consuming.
    RateLimiterAvailable {
        key: String,
        capacity_tokens: u64,
        refill_rate: f64,
    },
    /// Reset a rate limiter to full capacity.
    RateLimiterReset {
        key: String,
        capacity_tokens: u64,
        refill_rate: f64,
    },

    // Barrier operations
    /// Enter a barrier, waiting until all participants arrive.
    BarrierEnter {
        name: String,
        participant_id: String,
        required_count: u32,
        timeout_ms: u64,
    },
    /// Leave a barrier after work is complete.
    BarrierLeave {
        name: String,
        participant_id: String,
        timeout_ms: u64,
    },
    /// Query barrier status without blocking.
    BarrierStatus { name: String },

    // Semaphore operations
    /// Acquire permits from a semaphore, blocking until available.
    SemaphoreAcquire {
        name: String,
        holder_id: String,
        permits: u32,
        capacity_permits: u32,
        ttl_ms: u64,
        timeout_ms: u64,
    },
    /// Try to acquire permits without blocking.
    SemaphoreTryAcquire {
        name: String,
        holder_id: String,
        permits: u32,
        capacity_permits: u32,
        ttl_ms: u64,
    },
    /// Release permits back to a semaphore.
    SemaphoreRelease {
        name: String,
        holder_id: String,
        permits: u32,
    },
    /// Query semaphore status.
    SemaphoreStatus { name: String },

    // RWLock operations
    /// Acquire read lock (blocking until available or timeout).
    RWLockAcquireRead {
        name: String,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    },
    /// Try to acquire read lock (non-blocking).
    RWLockTryAcquireRead {
        name: String,
        holder_id: String,
        ttl_ms: u64,
    },
    /// Acquire write lock (blocking until available or timeout).
    RWLockAcquireWrite {
        name: String,
        holder_id: String,
        ttl_ms: u64,
        timeout_ms: u64,
    },
    /// Try to acquire write lock (non-blocking).
    RWLockTryAcquireWrite {
        name: String,
        holder_id: String,
        ttl_ms: u64,
    },
    /// Release read lock.
    RWLockReleaseRead { name: String, holder_id: String },
    /// Release write lock.
    RWLockReleaseWrite {
        name: String,
        holder_id: String,
        fencing_token: u64,
    },
    /// Downgrade write lock to read lock.
    RWLockDowngrade {
        name: String,
        holder_id: String,
        fencing_token: u64,
        ttl_ms: u64,
    },
    /// Query RWLock status.
    RWLockStatus { name: String },

    // Queue operations
    /// Create a distributed queue.
    QueueCreate {
        queue_name: String,
        default_visibility_timeout_ms: Option<u64>,
        default_ttl_ms: Option<u64>,
        max_delivery_attempts: Option<u32>,
    },
    /// Delete a queue and all its items.
    QueueDelete { queue_name: String },
    /// Enqueue an item to a distributed queue.
    QueueEnqueue {
        queue_name: String,
        payload: Vec<u8>,
        ttl_ms: Option<u64>,
        message_group_id: Option<String>,
        deduplication_id: Option<String>,
    },
    /// Enqueue multiple items in a batch.
    QueueEnqueueBatch {
        queue_name: String,
        items: Vec<QueueEnqueueItem>,
    },
    /// Dequeue items from a queue with visibility timeout (non-blocking).
    QueueDequeue {
        queue_name: String,
        consumer_id: String,
        max_items: u32,
        visibility_timeout_ms: u64,
    },
    /// Dequeue items with blocking wait.
    QueueDequeueWait {
        queue_name: String,
        consumer_id: String,
        max_items: u32,
        visibility_timeout_ms: u64,
        wait_timeout_ms: u64,
    },
    /// Peek at items without removing them.
    QueuePeek { queue_name: String, max_items: u32 },
    /// Acknowledge successful processing of an item.
    QueueAck { queue_name: String, receipt_handle: String },
    /// Negative acknowledge - return to queue or move to DLQ.
    QueueNack {
        queue_name: String,
        receipt_handle: String,
        move_to_dlq: bool,
        error_message: Option<String>,
    },
    /// Extend visibility timeout for a pending item.
    QueueExtendVisibility {
        queue_name: String,
        receipt_handle: String,
        additional_timeout_ms: u64,
    },
    /// Get queue status.
    QueueStatus { queue_name: String },
    /// Get items from dead letter queue.
    QueueGetDLQ { queue_name: String, max_items: u32 },
    /// Move DLQ item back to main queue.
    QueueRedriveDLQ { queue_name: String, item_id: u64 },

    // Service registry operations
    /// Register a service instance.
    ServiceRegister {
        service_name: String,
        instance_id: String,
        address: String,
        version: String,
        tags: String,
        weight: u32,
        custom_metadata: String,
        ttl_ms: u64,
        lease_id: Option<u64>,
    },
    /// Deregister a service instance.
    ServiceDeregister {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
    },
    /// Discover service instances.
    ServiceDiscover {
        service_name: String,
        healthy_only: bool,
        tags: String,
        version_prefix: Option<String>,
        limit: Option<u32>,
    },
    /// Discover services by name prefix.
    ServiceList { prefix: String, limit: u32 },
    /// Get a specific service instance.
    ServiceGetInstance { service_name: String, instance_id: String },
    /// Send heartbeat to renew TTL.
    ServiceHeartbeat {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
    },
    /// Update instance health status.
    ServiceUpdateHealth {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
        status: String,
    },
    /// Update instance metadata.
    ServiceUpdateMetadata {
        service_name: String,
        instance_id: String,
        fencing_token: u64,
        version: Option<String>,
        tags: Option<String>,
        weight: Option<u32>,
        custom_metadata: Option<String>,
    },
}

impl CoordinationRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        self.to_operation_lock_counter()
            .or_else(|| self.to_operation_ratelimiter_barrier())
            .or_else(|| self.to_operation_semaphore_rwlock())
            .or_else(|| self.to_operation_queue())
            .or_else(|| self.to_operation_service())
    }

    fn to_operation_lock_counter(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::LockAcquire { key, .. }
            | Self::LockTryAcquire { key, .. }
            | Self::LockRelease { key, .. }
            | Self::LockRenew { key, .. } => Some(Operation::Write {
                key: format!("_lock:{key}"),
                value: vec![],
            }),

            Self::CounterGet { key } | Self::SignedCounterGet { key } | Self::SequenceCurrent { key } => {
                Some(Operation::Read {
                    key: format!("_counter:{key}"),
                })
            }
            Self::CounterIncrement { key }
            | Self::CounterDecrement { key }
            | Self::CounterAdd { key, .. }
            | Self::CounterSubtract { key, .. }
            | Self::CounterSet { key, .. }
            | Self::CounterCompareAndSet { key, .. }
            | Self::SignedCounterAdd { key, .. }
            | Self::SequenceNext { key }
            | Self::SequenceReserve { key, .. } => Some(Operation::Write {
                key: format!("_counter:{key}"),
                value: vec![],
            }),

            _ => None,
        }
    }

    fn to_operation_ratelimiter_barrier(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::RateLimiterTryAcquire { key, .. }
            | Self::RateLimiterAcquire { key, .. }
            | Self::RateLimiterReset { key, .. } => Some(Operation::Write {
                key: format!("_ratelimit:{key}"),
                value: vec![],
            }),
            Self::RateLimiterAvailable { key, .. } => Some(Operation::Read {
                key: format!("_ratelimit:{key}"),
            }),

            Self::BarrierEnter { name, .. } | Self::BarrierLeave { name, .. } => Some(Operation::Write {
                key: format!("_barrier:{name}"),
                value: vec![],
            }),
            Self::BarrierStatus { name } => Some(Operation::Read {
                key: format!("_barrier:{name}"),
            }),

            _ => None,
        }
    }

    fn to_operation_semaphore_rwlock(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::SemaphoreAcquire { name, .. }
            | Self::SemaphoreTryAcquire { name, .. }
            | Self::SemaphoreRelease { name, .. } => Some(Operation::Write {
                key: format!("_semaphore:{name}"),
                value: vec![],
            }),
            Self::SemaphoreStatus { name } => Some(Operation::Read {
                key: format!("_semaphore:{name}"),
            }),

            Self::RWLockAcquireRead { name, .. }
            | Self::RWLockTryAcquireRead { name, .. }
            | Self::RWLockAcquireWrite { name, .. }
            | Self::RWLockTryAcquireWrite { name, .. }
            | Self::RWLockReleaseRead { name, .. }
            | Self::RWLockReleaseWrite { name, .. }
            | Self::RWLockDowngrade { name, .. } => Some(Operation::Write {
                key: format!("_rwlock:{name}"),
                value: vec![],
            }),
            Self::RWLockStatus { name } => Some(Operation::Read {
                key: format!("_rwlock:{name}"),
            }),

            _ => None,
        }
    }

    fn to_operation_queue(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::QueueCreate { queue_name, .. }
            | Self::QueueDelete { queue_name }
            | Self::QueueEnqueue { queue_name, .. }
            | Self::QueueEnqueueBatch { queue_name, .. }
            | Self::QueueDequeue { queue_name, .. }
            | Self::QueueDequeueWait { queue_name, .. }
            | Self::QueueAck { queue_name, .. }
            | Self::QueueNack { queue_name, .. }
            | Self::QueueExtendVisibility { queue_name, .. }
            | Self::QueueRedriveDLQ { queue_name, .. } => Some(Operation::Write {
                key: format!("_queue:{queue_name}"),
                value: vec![],
            }),
            Self::QueuePeek { queue_name, .. }
            | Self::QueueStatus { queue_name }
            | Self::QueueGetDLQ { queue_name, .. } => Some(Operation::Read {
                key: format!("_queue:{queue_name}"),
            }),

            _ => None,
        }
    }

    fn to_operation_service(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::ServiceRegister { service_name, .. }
            | Self::ServiceDeregister { service_name, .. }
            | Self::ServiceHeartbeat { service_name, .. }
            | Self::ServiceUpdateHealth { service_name, .. }
            | Self::ServiceUpdateMetadata { service_name, .. } => Some(Operation::Write {
                key: format!("_service:{service_name}"),
                value: vec![],
            }),
            Self::ServiceDiscover { service_name, .. } | Self::ServiceGetInstance { service_name, .. } => {
                Some(Operation::Read {
                    key: format!("_service:{service_name}"),
                })
            }
            Self::ServiceList { prefix, .. } => Some(Operation::Read {
                key: format!("_service:{prefix}"),
            }),

            _ => None,
        }
    }
}
