//! Error types for consumer group operations.
//!
//! Uses snafu for structured error handling with context.

use snafu::Snafu;

use super::constants::MAX_BATCH_ACK_SIZE;
use super::constants::MAX_CONSUMER_GROUPS;
use super::constants::MAX_CONSUMERS_PER_GROUP;
use super::constants::MAX_DELIVERY_ATTEMPTS;
use super::constants::MAX_FETCH_BATCH_SIZE;
use super::constants::MAX_PARTITIONS_PER_GROUP;
use super::constants::MAX_PENDING_PER_CONSUMER;
use super::constants::MAX_VISIBILITY_TIMEOUT_MS;

/// Errors that can occur during consumer group operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ConsumerGroupError {
    /// Consumer group not found.
    #[snafu(display("consumer group not found: {group_id}"))]
    GroupNotFound {
        /// The group ID that was not found.
        group_id: String,
    },

    /// Consumer group already exists.
    #[snafu(display("consumer group already exists: {group_id}"))]
    GroupAlreadyExists {
        /// The group ID that already exists.
        group_id: String,
    },

    /// Consumer not found in group.
    #[snafu(display("consumer '{consumer_id}' not found in group '{group_id}'"))]
    ConsumerNotFound {
        /// The group ID.
        group_id: String,
        /// The consumer ID that was not found.
        consumer_id: String,
    },

    /// Consumer already exists in group.
    #[snafu(display("consumer '{consumer_id}' already exists in group '{group_id}'"))]
    ConsumerAlreadyExists {
        /// The group ID.
        group_id: String,
        /// The consumer ID that already exists.
        consumer_id: String,
    },

    /// Too many consumer groups cluster-wide.
    #[snafu(display("too many consumer groups: {count} (max {MAX_CONSUMER_GROUPS})"))]
    TooManyGroups {
        /// Current number of groups.
        count: usize,
    },

    /// Too many consumers in a group.
    #[snafu(display("too many consumers in group '{group_id}': {count} (max {MAX_CONSUMERS_PER_GROUP})"))]
    TooManyConsumers {
        /// The group ID.
        group_id: String,
        /// Current number of consumers.
        count: u32,
    },

    /// Too many pending messages for consumer.
    #[snafu(display(
        "consumer '{consumer_id}' has too many pending messages: {count} (max {MAX_PENDING_PER_CONSUMER})"
    ))]
    TooManyPending {
        /// The consumer ID.
        consumer_id: String,
        /// Current pending count.
        count: u32,
    },

    /// Too many partitions for group.
    #[snafu(display("group '{group_id}' has too many partitions: {count} (max {MAX_PARTITIONS_PER_GROUP})"))]
    TooManyPartitions {
        /// The group ID.
        group_id: String,
        /// Requested partition count.
        count: u32,
    },

    /// Invalid group ID format.
    #[snafu(display("invalid group ID: {reason}"))]
    InvalidGroupId {
        /// Reason the ID is invalid.
        reason: String,
    },

    /// Invalid consumer ID format.
    #[snafu(display("invalid consumer ID: {reason}"))]
    InvalidConsumerId {
        /// Reason the ID is invalid.
        reason: String,
    },

    /// Invalid configuration.
    #[snafu(display("invalid configuration: {reason}"))]
    InvalidConfig {
        /// Description of the invalid configuration.
        reason: String,
    },

    /// Visibility timeout too small.
    #[snafu(display("visibility timeout {timeout_ms}ms is too small (min 1000ms)"))]
    VisibilityTimeoutTooSmall {
        /// The provided timeout.
        timeout_ms: u64,
    },

    /// Visibility timeout too large.
    #[snafu(display("visibility timeout {timeout_ms}ms is too large (max {MAX_VISIBILITY_TIMEOUT_MS}ms)"))]
    VisibilityTimeoutTooLarge {
        /// The provided timeout.
        timeout_ms: u64,
    },

    /// Max delivery attempts too small.
    #[snafu(display("max delivery attempts {attempts} is too small (min 1)"))]
    MaxDeliveryAttemptsTooSmall {
        /// The provided value.
        attempts: u32,
    },

    /// Max delivery attempts too large.
    #[snafu(display("max delivery attempts {attempts} is too large (max {MAX_DELIVERY_ATTEMPTS})"))]
    MaxDeliveryAttemptsTooLarge {
        /// The provided value.
        attempts: u32,
    },

    /// Fetch batch size too large.
    #[snafu(display("fetch batch size {size} is too large (max {MAX_FETCH_BATCH_SIZE})"))]
    FetchBatchTooLarge {
        /// The requested batch size.
        size: u32,
    },

    /// Ack batch too large.
    #[snafu(display("ack batch size {size} is too large (max {MAX_BATCH_ACK_SIZE})"))]
    AckBatchTooLarge {
        /// The requested batch size.
        size: usize,
    },

    /// Fencing token mismatch.
    #[snafu(display("fencing token mismatch for consumer '{consumer_id}': expected {expected}, got {actual}"))]
    FencingTokenMismatch {
        /// The consumer ID.
        consumer_id: String,
        /// Expected token.
        expected: u64,
        /// Actual token provided.
        actual: u64,
    },

    /// Consumer session expired.
    #[snafu(display("consumer session expired for '{consumer_id}' in group '{group_id}'"))]
    SessionExpired {
        /// The group ID.
        group_id: String,
        /// The consumer ID.
        consumer_id: String,
    },

    /// Group is rebalancing.
    #[snafu(display("group '{group_id}' is currently rebalancing"))]
    GroupRebalancing {
        /// The group ID.
        group_id: String,
    },

    /// Group is in invalid state for operation.
    #[snafu(display("group '{group_id}' is in state '{state}', cannot perform {operation}"))]
    InvalidGroupState {
        /// The group ID.
        group_id: String,
        /// Current state.
        state: String,
        /// Attempted operation.
        operation: String,
    },

    /// Partition not assigned to consumer.
    #[snafu(display("partition {partition_id} is not assigned to consumer '{consumer_id}'"))]
    PartitionNotAssigned {
        /// The partition ID.
        partition_id: u32,
        /// The consumer ID.
        consumer_id: String,
    },

    /// Key-value store operation failed.
    #[snafu(display("key-value store operation failed: {source}"))]
    KvStoreFailed {
        /// The underlying KV store error.
        source: aspen_core::KeyValueStoreError,
    },

    /// Serialization failed.
    #[snafu(display("serialization failed: {message}"))]
    SerializationFailed {
        /// Description of the failure.
        message: String,
    },

    /// Internal error.
    #[snafu(display("internal error: {message}"))]
    Internal {
        /// Description of the internal error.
        message: String,
    },

    /// Offset not found.
    #[snafu(display("no committed offset for group '{group_id}' partition {partition_id}"))]
    OffsetNotFound {
        /// The group ID.
        group_id: String,
        /// The partition ID.
        partition_id: u32,
    },
}

impl From<aspen_core::KeyValueStoreError> for ConsumerGroupError {
    fn from(source: aspen_core::KeyValueStoreError) -> Self {
        ConsumerGroupError::KvStoreFailed { source }
    }
}

/// Result type for consumer group operations.
pub type Result<T> = std::result::Result<T, ConsumerGroupError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ConsumerGroupError::GroupNotFound {
            group_id: "my-group".to_string(),
        };
        assert!(err.to_string().contains("my-group"));

        let err = ConsumerGroupError::TooManyConsumers {
            group_id: "g1".to_string(),
            count: 150,
        };
        assert!(err.to_string().contains("150"));
        assert!(err.to_string().contains("100")); // MAX_CONSUMERS_PER_GROUP
    }
}
