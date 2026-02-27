//! Coordination primitive response types.
//!
//! Response types for distributed coordination primitives including locks,
//! counters, sequences, rate limiters, barriers, semaphores, read-write locks,
//! queues, and service registry.

use serde::Deserialize;
use serde::Serialize;

// =============================================================================
// Lock types
// =============================================================================

/// Lock operation result response.
///
/// Used for distributed lock acquire, try_acquire, release, and renew operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockResultResponse {
    /// Whether the lock operation succeeded.
    pub is_success: bool,
    /// Fencing token for the lock (if acquired).
    pub fencing_token: Option<u64>,
    /// Current holder ID (useful when lock is already held).
    pub holder_id: Option<String>,
    /// Lock deadline in Unix milliseconds (when lock expires).
    pub deadline_ms: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Counter types
// =============================================================================

/// Counter operation result response.
///
/// Used for atomic counter get, increment, decrement, add, subtract, set operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterResultResponse {
    /// Whether the counter operation succeeded.
    pub is_success: bool,
    /// Current counter value after operation.
    pub value: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Signed counter operation result response.
///
/// Used for signed atomic counter operations that can go negative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCounterResultResponse {
    /// Whether the counter operation succeeded.
    pub is_success: bool,
    /// Current counter value after operation (can be negative).
    pub value: Option<i64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Sequence types
// =============================================================================

/// Sequence generator result response.
///
/// Used for sequence next, reserve, and current operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceResultResponse {
    /// Whether the sequence operation succeeded.
    pub is_success: bool,
    /// Sequence value (next ID or start of reserved range).
    pub value: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Rate limiter types
// =============================================================================

/// Rate limiter result response.
///
/// Used for rate limiter try_acquire, acquire, available, and reset operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterResultResponse {
    /// Whether the rate limit operation succeeded (tokens acquired).
    pub is_success: bool,
    /// Remaining tokens after operation.
    pub tokens_remaining: Option<u64>,
    /// Milliseconds to wait before retrying (when rate limited).
    pub retry_after_ms: Option<u64>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Barrier types
// =============================================================================

/// Barrier operation result response.
///
/// Used for BarrierEnter, BarrierLeave, and BarrierStatus operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Current number of participants at the barrier.
    pub current_count: Option<u32>,
    /// Required number of participants to release the barrier.
    pub required_count: Option<u32>,
    /// Current barrier phase: "waiting", "ready", or "leaving".
    pub phase: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Semaphore types
// =============================================================================

/// Semaphore operation result response.
///
/// Used for SemaphoreAcquire, SemaphoreTryAcquire, SemaphoreRelease, and SemaphoreStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemaphoreResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Number of permits acquired (for acquire operations).
    pub permits_acquired: Option<u32>,
    /// Number of permits currently available.
    pub available: Option<u32>,
    /// Total capacity of the semaphore (in permit count).
    pub capacity_permits: Option<u32>,
    /// Suggested retry delay in milliseconds (if acquire failed due to no permits).
    pub retry_after_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Read-Write Lock types
// =============================================================================

/// Read-write lock operation result response.
///
/// Used for RWLockAcquireRead, RWLockAcquireWrite, RWLockRelease, RWLockDowngrade, and
/// RWLockStatus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RWLockResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Current lock mode: "free", "read", or "write".
    pub mode: Option<String>,
    /// Fencing token (for write locks and downgrade).
    pub fencing_token: Option<u64>,
    /// Lock deadline in milliseconds since epoch.
    pub deadline_ms: Option<u64>,
    /// Number of active readers.
    pub reader_count: Option<u32>,
    /// Writer holder ID (if mode == "write").
    pub writer_holder: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Queue types
// =============================================================================

/// Item to enqueue in a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueItem {
    /// Item payload.
    pub payload: Vec<u8>,
    /// Optional TTL in milliseconds.
    pub ttl_ms: Option<u64>,
    /// Optional message group ID for FIFO ordering.
    pub message_group_id: Option<String>,
    /// Optional deduplication ID.
    pub deduplication_id: Option<String>,
}

/// Queue create operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueCreateResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// True if queue was created, false if it already existed.
    pub was_created: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue delete operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDeleteResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Number of items deleted.
    pub items_deleted: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue enqueue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Item ID assigned to the enqueued item.
    pub item_id: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue batch enqueue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueBatchResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Item IDs assigned to the enqueued items.
    pub item_ids: Vec<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A dequeued item with receipt handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeuedItemResponse {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Receipt handle for ack/nack.
    pub receipt_handle: String,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Visibility deadline (Unix ms).
    pub visibility_deadline_ms: u64,
}

/// Queue dequeue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDequeueResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Dequeued items.
    pub items: Vec<QueueDequeuedItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A queue item for peek response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemResponse {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Expiration time (Unix ms), 0 = no expiration.
    pub expires_at_ms: u64,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
}

/// Queue peek operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePeekResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Peeked items.
    pub items: Vec<QueueItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue ack operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAckResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue nack operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueNackResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue extend visibility operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExtendVisibilityResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// New visibility deadline (Unix ms).
    pub new_deadline_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue status result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the queue exists.
    pub does_exist: bool,
    /// Approximate number of visible items.
    pub visible_count: Option<u64>,
    /// Approximate number of pending items.
    pub pending_count: Option<u64>,
    /// Approximate number of DLQ items.
    pub dlq_count: Option<u64>,
    /// Total items enqueued.
    pub total_enqueued: Option<u64>,
    /// Total items acked.
    pub total_acked: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A DLQ item response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDLQItemResponse {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason for moving to DLQ.
    pub reason: String,
    /// Time moved to DLQ (Unix ms).
    pub moved_at_ms: u64,
    /// Last error message (if any).
    pub last_error: Option<String>,
}

/// Queue get DLQ operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueGetDLQResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// DLQ items.
    pub items: Vec<QueueDLQItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue redrive DLQ operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRedriveDLQResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Service Registry types
// =============================================================================

/// Service register operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegisterResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Fencing token for this registration.
    pub fencing_token: Option<u64>,
    /// Registration deadline (Unix ms).
    pub deadline_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service deregister operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeregisterResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the instance was registered before removal.
    pub was_registered: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A service instance in discovery results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstanceResponse {
    /// Unique instance identifier.
    pub instance_id: String,
    /// Service name.
    pub service_name: String,
    /// Network address (host:port).
    pub address: String,
    /// Health status: "healthy", "unhealthy", "unknown".
    pub health_status: String,
    /// Version string.
    pub version: String,
    /// Tags for filtering.
    pub tags: Vec<String>,
    /// Load balancing weight.
    pub weight: u32,
    /// Custom metadata (JSON object).
    pub custom_metadata: String,
    /// Registration time (Unix ms).
    pub registered_at_ms: u64,
    /// Last heartbeat time (Unix ms).
    pub last_heartbeat_ms: u64,
    /// TTL deadline (Unix ms).
    pub deadline_ms: u64,
    /// Associated lease ID (if any).
    pub lease_id: Option<u64>,
    /// Fencing token.
    pub fencing_token: u64,
}

/// Service discover operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoverResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// List of matching instances.
    pub instances: Vec<ServiceInstanceResponse>,
    /// Number of instances returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service list operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceListResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// List of service names.
    pub services: Vec<String>,
    /// Number of services returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service get instance operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceGetInstanceResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the instance was found.
    pub was_found: bool,
    /// The instance (if found).
    pub instance: Option<ServiceInstanceResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service heartbeat operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHeartbeatResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// New deadline (Unix ms).
    pub new_deadline_ms: Option<u64>,
    /// Current health status.
    pub health_status: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service update health operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateHealthResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service update metadata operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateMetadataResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: postcard roundtrip.
    fn postcard_roundtrip<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(val: &T) {
        let bytes = postcard::to_stdvec(val).expect("postcard serialize");
        let _: T = postcard::from_bytes(&bytes).expect("postcard deserialize");
    }

    /// Helper: JSON roundtrip.
    fn json_roundtrip<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(val: &T) {
        let json = serde_json::to_string(val).expect("json serialize");
        let _: T = serde_json::from_str(&json).expect("json deserialize");
    }

    /// Helper: run both roundtrips.
    fn roundtrip<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(val: &T) {
        postcard_roundtrip(val);
        json_roundtrip(val);
    }

    // =========================================================================
    // Lock
    // =========================================================================

    #[test]
    fn lock_result_roundtrip_success() {
        roundtrip(&LockResultResponse {
            is_success: true,
            fencing_token: Some(42),
            holder_id: Some("node-1".into()),
            deadline_ms: Some(1_700_000_000_000),
            error: None,
        });
    }

    #[test]
    fn lock_result_roundtrip_failure() {
        roundtrip(&LockResultResponse {
            is_success: false,
            fencing_token: None,
            holder_id: Some("other-holder".into()),
            deadline_ms: None,
            error: Some("lock already held".into()),
        });
    }

    #[test]
    fn lock_result_roundtrip_all_none() {
        roundtrip(&LockResultResponse {
            is_success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: None,
        });
    }

    // =========================================================================
    // Counter
    // =========================================================================

    #[test]
    fn counter_result_roundtrip() {
        roundtrip(&CounterResultResponse {
            is_success: true,
            value: Some(100),
            error: None,
        });
        roundtrip(&CounterResultResponse {
            is_success: true,
            value: Some(u64::MAX),
            error: None,
        });
    }

    #[test]
    fn signed_counter_result_roundtrip() {
        roundtrip(&SignedCounterResultResponse {
            is_success: true,
            value: Some(-42),
            error: None,
        });
        roundtrip(&SignedCounterResultResponse {
            is_success: true,
            value: Some(i64::MIN),
            error: None,
        });
        roundtrip(&SignedCounterResultResponse {
            is_success: true,
            value: Some(i64::MAX),
            error: None,
        });
    }

    // =========================================================================
    // Sequence
    // =========================================================================

    #[test]
    fn sequence_result_roundtrip() {
        roundtrip(&SequenceResultResponse {
            is_success: true,
            value: Some(1),
            error: None,
        });
        roundtrip(&SequenceResultResponse {
            is_success: false,
            value: None,
            error: Some("sequence exhausted".into()),
        });
    }

    // =========================================================================
    // Rate Limiter
    // =========================================================================

    #[test]
    fn rate_limiter_result_roundtrip() {
        roundtrip(&RateLimiterResultResponse {
            is_success: true,
            tokens_remaining: Some(9),
            retry_after_ms: None,
            error: None,
        });
        roundtrip(&RateLimiterResultResponse {
            is_success: false,
            tokens_remaining: Some(0),
            retry_after_ms: Some(1000),
            error: Some("rate limited".into()),
        });
    }

    // =========================================================================
    // Barrier
    // =========================================================================

    #[test]
    fn barrier_result_roundtrip() {
        roundtrip(&BarrierResultResponse {
            is_success: true,
            current_count: Some(3),
            required_count: Some(5),
            phase: Some("waiting".into()),
            error: None,
        });
    }

    // =========================================================================
    // Semaphore
    // =========================================================================

    #[test]
    fn semaphore_result_roundtrip() {
        roundtrip(&SemaphoreResultResponse {
            is_success: true,
            permits_acquired: Some(2),
            available: Some(8),
            capacity_permits: Some(10),
            retry_after_ms: None,
            error: None,
        });
        roundtrip(&SemaphoreResultResponse {
            is_success: false,
            permits_acquired: None,
            available: Some(0),
            capacity_permits: Some(10),
            retry_after_ms: Some(500),
            error: Some("no permits available".into()),
        });
    }

    // =========================================================================
    // RWLock
    // =========================================================================

    #[test]
    fn rwlock_result_roundtrip() {
        roundtrip(&RWLockResultResponse {
            is_success: true,
            mode: Some("write".into()),
            fencing_token: Some(7),
            deadline_ms: Some(1_700_000_000_000),
            reader_count: Some(0),
            writer_holder: Some("writer-1".into()),
            error: None,
        });
        roundtrip(&RWLockResultResponse {
            is_success: true,
            mode: Some("read".into()),
            fencing_token: None,
            deadline_ms: Some(1_700_000_000_000),
            reader_count: Some(3),
            writer_holder: None,
            error: None,
        });
    }

    // =========================================================================
    // Queue types
    // =========================================================================

    #[test]
    fn queue_enqueue_item_roundtrip() {
        roundtrip(&QueueEnqueueItem {
            payload: vec![1, 2, 3],
            ttl_ms: Some(60_000),
            message_group_id: Some("group-a".into()),
            deduplication_id: Some("dedup-1".into()),
        });
        roundtrip(&QueueEnqueueItem {
            payload: vec![],
            ttl_ms: None,
            message_group_id: None,
            deduplication_id: None,
        });
    }

    #[test]
    fn queue_create_result_roundtrip() {
        roundtrip(&QueueCreateResultResponse {
            is_success: true,
            was_created: true,
            error: None,
        });
    }

    #[test]
    fn queue_delete_result_roundtrip() {
        roundtrip(&QueueDeleteResultResponse {
            is_success: true,
            items_deleted: Some(42),
            error: None,
        });
    }

    #[test]
    fn queue_enqueue_result_roundtrip() {
        roundtrip(&QueueEnqueueResultResponse {
            is_success: true,
            item_id: Some(12345),
            error: None,
        });
    }

    #[test]
    fn queue_enqueue_batch_result_roundtrip() {
        roundtrip(&QueueEnqueueBatchResultResponse {
            is_success: true,
            item_ids: vec![1, 2, 3, 4, 5],
            error: None,
        });
        roundtrip(&QueueEnqueueBatchResultResponse {
            is_success: true,
            item_ids: vec![],
            error: None,
        });
    }

    #[test]
    fn queue_dequeue_result_roundtrip() {
        roundtrip(&QueueDequeueResultResponse {
            is_success: true,
            items: vec![QueueDequeuedItemResponse {
                item_id: 1,
                payload: vec![0xDE, 0xAD],
                receipt_handle: "rh-abc".into(),
                delivery_attempts: 1,
                enqueued_at_ms: 1_700_000_000_000,
                visibility_deadline_ms: 1_700_000_060_000,
            }],
            error: None,
        });
        // Empty dequeue
        roundtrip(&QueueDequeueResultResponse {
            is_success: true,
            items: vec![],
            error: None,
        });
    }

    #[test]
    fn queue_peek_result_roundtrip() {
        roundtrip(&QueuePeekResultResponse {
            is_success: true,
            items: vec![QueueItemResponse {
                item_id: 1,
                payload: vec![42],
                enqueued_at_ms: 1_700_000_000_000,
                expires_at_ms: 0,
                delivery_attempts: 0,
            }],
            error: None,
        });
    }

    #[test]
    fn queue_ack_nack_roundtrip() {
        roundtrip(&QueueAckResultResponse {
            is_success: true,
            error: None,
        });
        roundtrip(&QueueNackResultResponse {
            is_success: true,
            error: None,
        });
    }

    #[test]
    fn queue_extend_visibility_roundtrip() {
        roundtrip(&QueueExtendVisibilityResultResponse {
            is_success: true,
            new_deadline_ms: Some(1_700_000_120_000),
            error: None,
        });
    }

    #[test]
    fn queue_status_roundtrip() {
        roundtrip(&QueueStatusResultResponse {
            is_success: true,
            does_exist: true,
            visible_count: Some(10),
            pending_count: Some(3),
            dlq_count: Some(1),
            total_enqueued: Some(1000),
            total_acked: Some(986),
            error: None,
        });
    }

    #[test]
    fn queue_dlq_roundtrip() {
        roundtrip(&QueueGetDLQResultResponse {
            is_success: true,
            items: vec![QueueDLQItemResponse {
                item_id: 99,
                payload: vec![1, 2, 3],
                enqueued_at_ms: 1_700_000_000_000,
                delivery_attempts: 5,
                reason: "max retries exceeded".into(),
                moved_at_ms: 1_700_000_300_000,
                last_error: Some("timeout".into()),
            }],
            error: None,
        });
    }

    #[test]
    fn queue_redrive_dlq_roundtrip() {
        roundtrip(&QueueRedriveDLQResultResponse {
            is_success: true,
            error: None,
        });
    }

    // =========================================================================
    // Service Registry
    // =========================================================================

    fn sample_instance() -> ServiceInstanceResponse {
        ServiceInstanceResponse {
            instance_id: "inst-1".into(),
            service_name: "my-service".into(),
            address: "10.0.0.1:8080".into(),
            health_status: "healthy".into(),
            version: "1.0.0".into(),
            tags: vec!["region:us-east".into(), "env:prod".into()],
            weight: 100,
            custom_metadata: r#"{"zone":"a"}"#.into(),
            registered_at_ms: 1_700_000_000_000,
            last_heartbeat_ms: 1_700_000_060_000,
            deadline_ms: 1_700_000_120_000,
            lease_id: Some(42),
            fencing_token: 7,
        }
    }

    #[test]
    fn service_register_result_roundtrip() {
        roundtrip(&ServiceRegisterResultResponse {
            is_success: true,
            fencing_token: Some(1),
            deadline_ms: Some(1_700_000_060_000),
            error: None,
        });
    }

    #[test]
    fn service_deregister_result_roundtrip() {
        roundtrip(&ServiceDeregisterResultResponse {
            is_success: true,
            was_registered: true,
            error: None,
        });
    }

    #[test]
    fn service_discover_result_roundtrip() {
        roundtrip(&ServiceDiscoverResultResponse {
            is_success: true,
            instances: vec![sample_instance()],
            count: 1,
            error: None,
        });
        // Empty results
        roundtrip(&ServiceDiscoverResultResponse {
            is_success: true,
            instances: vec![],
            count: 0,
            error: None,
        });
    }

    #[test]
    fn service_list_result_roundtrip() {
        roundtrip(&ServiceListResultResponse {
            is_success: true,
            services: vec!["svc-a".into(), "svc-b".into()],
            count: 2,
            error: None,
        });
    }

    #[test]
    fn service_get_instance_result_roundtrip() {
        roundtrip(&ServiceGetInstanceResultResponse {
            is_success: true,
            was_found: true,
            instance: Some(sample_instance()),
            error: None,
        });
        roundtrip(&ServiceGetInstanceResultResponse {
            is_success: true,
            was_found: false,
            instance: None,
            error: None,
        });
    }

    #[test]
    fn service_heartbeat_result_roundtrip() {
        roundtrip(&ServiceHeartbeatResultResponse {
            is_success: true,
            new_deadline_ms: Some(1_700_000_120_000),
            health_status: Some("healthy".into()),
            error: None,
        });
    }

    #[test]
    fn service_update_health_result_roundtrip() {
        roundtrip(&ServiceUpdateHealthResultResponse {
            is_success: true,
            error: None,
        });
    }

    #[test]
    fn service_update_metadata_result_roundtrip() {
        roundtrip(&ServiceUpdateMetadataResultResponse {
            is_success: true,
            error: None,
        });
    }
}
