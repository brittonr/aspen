//! Tiger Style constants for consumer groups.
//!
//! All resource limits are explicit and chosen to prevent unbounded allocation
//! while supporting practical consumer group workloads.

/// Maximum number of consumer groups cluster-wide.
///
/// Groups are long-lived resources, so this limit is relatively generous.
pub const MAX_CONSUMER_GROUPS: usize = 1000;

/// Maximum consumers per group.
///
/// Beyond this, you should create multiple groups.
pub const MAX_CONSUMERS_PER_GROUP: u32 = 100;

/// Maximum pending (unacknowledged) messages per consumer.
///
/// Prevents a single consumer from hoarding messages.
/// If exceeded, the consumer should ack before fetching more.
pub const MAX_PENDING_PER_CONSUMER: u32 = 1000;

/// Maximum partitions per group (partitioned mode only).
///
/// Partitions should roughly match consumer count.
pub const MAX_PARTITIONS_PER_GROUP: u32 = 256;

/// Maximum events to fetch in a single poll.
pub const MAX_FETCH_BATCH_SIZE: u32 = 100;

/// Maximum receipts to ack in a single batch ack.
pub const MAX_BATCH_ACK_SIZE: usize = 100;

/// Default visibility timeout in milliseconds.
///
/// How long a message is invisible to other consumers after being delivered.
/// This is NATS JetStream's "AckWait".
pub const DEFAULT_VISIBILITY_TIMEOUT_MS: u64 = 30_000; // 30 seconds

/// Minimum visibility timeout in milliseconds.
pub const MIN_VISIBILITY_TIMEOUT_MS: u64 = 1_000; // 1 second

/// Maximum visibility timeout in milliseconds.
pub const MAX_VISIBILITY_TIMEOUT_MS: u64 = 43_200_000; // 12 hours

/// Default max delivery attempts before dead-lettering.
pub const DEFAULT_MAX_DELIVERY_ATTEMPTS: u32 = 5;

/// Minimum max delivery attempts.
pub const MIN_DELIVERY_ATTEMPTS: u32 = 1;

/// Maximum max delivery attempts.
pub const MAX_DELIVERY_ATTEMPTS: u32 = 100;

/// Consumer heartbeat interval in milliseconds.
///
/// Consumers must heartbeat at least this often to stay alive.
pub const CONSUMER_HEARTBEAT_INTERVAL_MS: u64 = 10_000; // 10 seconds

/// Consumer heartbeat timeout in milliseconds.
///
/// If a consumer doesn't heartbeat within this period, it's considered dead.
pub const CONSUMER_HEARTBEAT_TIMEOUT_MS: u64 = 30_000; // 30 seconds

/// Maximum dead letter queue entries per group.
///
/// Old DLQ entries can be pruned after being processed.
pub const MAX_DLQ_SIZE: u32 = 10_000;

/// Rebalance cooldown period in milliseconds.
///
/// Minimum time between rebalances to prevent thrashing.
pub const REBALANCE_COOLDOWN_MS: u64 = 5_000; // 5 seconds

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Consumer group limits must be positive
const _: () = assert!(MAX_CONSUMER_GROUPS > 0);
const _: () = assert!(MAX_CONSUMERS_PER_GROUP > 0);
const _: () = assert!(MAX_CONSUMERS_PER_GROUP >= 2); // need at least 2 for competition
const _: () = assert!(MAX_PENDING_PER_CONSUMER > 0);
const _: () = assert!(MAX_PENDING_PER_CONSUMER >= 1); // need at least 1 pending
const _: () = assert!(MAX_PARTITIONS_PER_GROUP > 0);

// Batch sizes must be positive
const _: () = assert!(MAX_FETCH_BATCH_SIZE > 0);
const _: () = assert!(MAX_BATCH_ACK_SIZE > 0);

// Visibility timeout ordering
const _: () = assert!(MIN_VISIBILITY_TIMEOUT_MS > 0);
const _: () = assert!(DEFAULT_VISIBILITY_TIMEOUT_MS >= MIN_VISIBILITY_TIMEOUT_MS);
const _: () = assert!(DEFAULT_VISIBILITY_TIMEOUT_MS <= MAX_VISIBILITY_TIMEOUT_MS);
const _: () = assert!(MAX_VISIBILITY_TIMEOUT_MS > 0);

// Delivery attempt ordering
const _: () = assert!(MIN_DELIVERY_ATTEMPTS > 0);
const _: () = assert!(DEFAULT_MAX_DELIVERY_ATTEMPTS >= MIN_DELIVERY_ATTEMPTS);
const _: () = assert!(DEFAULT_MAX_DELIVERY_ATTEMPTS <= MAX_DELIVERY_ATTEMPTS);
const _: () = assert!(MAX_DELIVERY_ATTEMPTS > 0);

// Heartbeat timing relationships
const _: () = assert!(CONSUMER_HEARTBEAT_INTERVAL_MS > 0);
const _: () = assert!(CONSUMER_HEARTBEAT_TIMEOUT_MS > CONSUMER_HEARTBEAT_INTERVAL_MS);
// Timeout should be at least 2x interval for reliability
const _: () = assert!(CONSUMER_HEARTBEAT_TIMEOUT_MS >= CONSUMER_HEARTBEAT_INTERVAL_MS * 2);

// DLQ and rebalance limits must be positive
const _: () = assert!(MAX_DLQ_SIZE > 0);
const _: () = assert!(REBALANCE_COOLDOWN_MS > 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_relationships() {
        // Visibility timeout range should be reasonable
        assert!(MAX_VISIBILITY_TIMEOUT_MS / MIN_VISIBILITY_TIMEOUT_MS >= 1000);
    }
}
