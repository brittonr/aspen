//! Tiger Style resource limits for consumer groups.
//!
//! These constants enforce bounded resource usage following Tiger Style principles.
//! All limits are chosen to prevent unbounded memory allocation while supporting
//! practical consumer group workloads.

/// Maximum consumers per consumer group.
///
/// Limits the number of concurrent consumers to prevent unbounded memory usage
/// for tracking consumer state and partition assignments.
pub const MAX_CONSUMERS_PER_GROUP: u32 = 256;

/// Maximum partitions per consumer group.
///
/// In partitioned mode, this limits how many partitions can be created.
/// More partitions allow better parallelism but increase coordination overhead.
pub const MAX_PARTITIONS_PER_GROUP: u32 = 256;

/// Maximum pending (unacknowledged) messages per consumer.
///
/// Prevents a slow consumer from accumulating too many in-flight messages,
/// which could delay redelivery and increase memory usage.
pub const MAX_PENDING_PER_CONSUMER: u32 = 1000;

/// Maximum total consumer groups.
///
/// System-wide limit on the number of consumer groups.
pub const MAX_CONSUMER_GROUPS: usize = 512;

/// Default visibility timeout in milliseconds (30 seconds).
///
/// How long a message is invisible to other consumers after being received.
/// If not acknowledged within this time, the message becomes available for redelivery.
/// NATS JetStream calls this "AckWait".
pub const DEFAULT_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// Minimum visibility timeout in milliseconds (1 second).
///
/// Lower bound on visibility timeout to ensure messages have reasonable
/// processing time before becoming available for redelivery.
pub const MIN_VISIBILITY_TIMEOUT_MS: u64 = 1_000;

/// Maximum visibility timeout in milliseconds (5 minutes).
///
/// Upper bound on visibility timeout to prevent messages from being locked
/// for unreasonably long periods.
pub const MAX_VISIBILITY_TIMEOUT_MS: u64 = 300_000;

/// Consumer heartbeat interval in milliseconds (10 seconds).
///
/// How often consumers should send heartbeats to maintain group membership.
/// Should be less than `CONSUMER_HEARTBEAT_TIMEOUT_MS`.
pub const CONSUMER_HEARTBEAT_INTERVAL_MS: u64 = 10_000;

/// Consumer heartbeat timeout in milliseconds (30 seconds).
///
/// If a consumer doesn't send a heartbeat within this period, it's considered
/// dead and its assigned partitions (in partitioned mode) or pending messages
/// (in competing mode) are redistributed.
pub const CONSUMER_HEARTBEAT_TIMEOUT_MS: u64 = 30_000;

/// Maximum messages in a batch acknowledgment.
///
/// Limits the size of batch ack operations to prevent oversized transactions.
pub const MAX_BATCH_ACK_SIZE: usize = 100;

/// Maximum messages in a batch receive.
///
/// Limits how many messages can be received in a single `receive()` call.
pub const MAX_BATCH_RECEIVE_SIZE: u32 = 100;

/// Default maximum delivery attempts before dead-lettering (5).
///
/// After this many failed delivery attempts, a message is moved to the
/// dead letter queue (DLQ) for manual inspection.
/// NATS JetStream calls this "MaxDeliver".
pub const DEFAULT_MAX_DELIVERY_ATTEMPTS: u32 = 5;

/// Minimum delivery attempts allowed.
///
/// Must deliver at least once before giving up.
pub const MIN_DELIVERY_ATTEMPTS: u32 = 1;

/// Maximum delivery attempts allowed.
///
/// Upper bound on delivery attempts configuration.
pub const MAX_DELIVERY_ATTEMPTS: u32 = 100;

/// Rebalance debounce period in milliseconds (5 seconds).
///
/// After a consumer joins or leaves, wait this long before rebalancing
/// to batch multiple membership changes together.
pub const REBALANCE_DEBOUNCE_MS: u64 = 5_000;

/// Maximum time to wait for rebalance completion in milliseconds (30 seconds).
pub const MAX_REBALANCE_TIMEOUT_MS: u64 = 30_000;

/// Consumer group key prefix.
///
/// All consumer group state is stored under this prefix in the KV store.
/// This is the same as `PUBSUB_GROUPS_PREFIX` from the parent module.
pub const CONSUMER_GROUP_PREFIX: &str = "__pubsub/groups/";

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time constant assertions
    const _: () = {
        // Heartbeat interval should be less than timeout
        assert!(CONSUMER_HEARTBEAT_INTERVAL_MS < CONSUMER_HEARTBEAT_TIMEOUT_MS);

        // Default visibility timeout should be within max
        assert!(DEFAULT_VISIBILITY_TIMEOUT_MS <= MAX_VISIBILITY_TIMEOUT_MS);

        // Batch sizes should be reasonable
        assert!(MAX_BATCH_ACK_SIZE >= 10);
        assert!(MAX_BATCH_RECEIVE_SIZE >= 10);

        // Consumer limits should be reasonable
        assert!(MAX_CONSUMERS_PER_GROUP >= 2);
        assert!(MAX_PENDING_PER_CONSUMER >= 10);
    };

    #[test]
    fn test_prefix_is_valid() {
        // Prefix should end with separator for proper namespacing
        assert!(CONSUMER_GROUP_PREFIX.ends_with('/'));
        // Prefix should start with system prefix
        assert!(CONSUMER_GROUP_PREFIX.starts_with("__"));
    }

    #[test]
    fn test_timing_relationships() {
        // Heartbeat interval should give at least 2 retries before timeout
        assert!(CONSUMER_HEARTBEAT_INTERVAL_MS * 2 < CONSUMER_HEARTBEAT_TIMEOUT_MS);

        // Rebalance debounce should be shorter than heartbeat timeout
        assert!(REBALANCE_DEBOUNCE_MS < CONSUMER_HEARTBEAT_TIMEOUT_MS);
    }
}
