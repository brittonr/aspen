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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
    /// Number of permits acquired (for acquire operations).
    pub permits_acquired: Option<u32>,
    /// Number of permits currently available.
    pub available: Option<u32>,
    /// Total capacity of the semaphore.
    pub capacity: Option<u32>,
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
    pub success: bool,
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
    pub success: bool,
    /// True if queue was created, false if it already existed.
    pub created: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue delete operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDeleteResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Number of items deleted.
    pub items_deleted: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue enqueue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Item ID assigned to the enqueued item.
    pub item_id: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue batch enqueue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEnqueueBatchResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
    /// Peeked items.
    pub items: Vec<QueueItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue ack operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAckResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue nack operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueNackResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue extend visibility operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExtendVisibilityResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// New visibility deadline (Unix ms).
    pub new_deadline_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue status result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatusResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the queue exists.
    pub exists: bool,
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
    pub success: bool,
    /// DLQ items.
    pub items: Vec<QueueDLQItemResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Queue redrive DLQ operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRedriveDLQResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
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
    pub success: bool,
    /// Whether the instance was found.
    pub found: bool,
    /// The instance (if found).
    pub instance: Option<ServiceInstanceResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service heartbeat operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHeartbeatResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
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
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Service update metadata operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateMetadataResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
