//! Error types for consumer group operations.
//!
//! Uses snafu for structured error handling with context.

use snafu::Snafu;

use crate::consumer_group::constants::MAX_BATCH_ACK_SIZE;
use crate::consumer_group::constants::MAX_BATCH_RECEIVE_SIZE;
use crate::consumer_group::constants::MAX_CONSUMER_GROUPS;
use crate::consumer_group::constants::MAX_CONSUMERS_PER_GROUP;
use crate::consumer_group::constants::MAX_DELIVERY_ATTEMPTS;
use crate::consumer_group::constants::MAX_PARTITIONS_PER_GROUP;
use crate::consumer_group::constants::MAX_PENDING_PER_CONSUMER;
use crate::consumer_group::constants::MAX_VISIBILITY_TIMEOUT_MS;
use crate::consumer_group::constants::MIN_DELIVERY_ATTEMPTS;
use crate::consumer_group::constants::MIN_VISIBILITY_TIMEOUT_MS;

/// Errors that can occur during consumer group operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ConsumerGroupError {
    /// Consumer group not found.
    #[snafu(display("consumer group '{group_id}' not found"))]
    GroupNotFound {
        /// The group ID that was not found.
        group_id: String,
    },

    /// Consumer group already exists.
    #[snafu(display("consumer group '{group_id}' already exists"))]
    GroupAlreadyExists {
        /// The group ID that already exists.
        group_id: String,
    },

    /// Maximum consumer groups exceeded.
    #[snafu(display("cannot create group: system has {count} groups, maximum is {MAX_CONSUMER_GROUPS}"))]
    TooManyGroups {
        /// Current number of groups.
        count: usize,
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

    /// Maximum consumers per group exceeded.
    #[snafu(display("group '{group_id}' has {count} consumers, maximum is {MAX_CONSUMERS_PER_GROUP}"))]
    TooManyConsumers {
        /// The group ID.
        group_id: String,
        /// Current number of consumers.
        count: u32,
    },

    /// Maximum partitions exceeded.
    #[snafu(display("group '{group_id}' requested {count} partitions, maximum is {MAX_PARTITIONS_PER_GROUP}"))]
    TooManyPartitions {
        /// The group ID.
        group_id: String,
        /// Requested number of partitions.
        count: u32,
    },

    /// Maximum pending messages per consumer exceeded.
    #[snafu(display("consumer '{consumer_id}' has {count} pending messages, maximum is {MAX_PENDING_PER_CONSUMER}"))]
    TooManyPending {
        /// The consumer ID.
        consumer_id: String,
        /// Current number of pending messages.
        count: u32,
    },

    /// Receive batch size too large.
    #[snafu(display("receive batch size {size} exceeds maximum of {MAX_BATCH_RECEIVE_SIZE}"))]
    ReceiveBatchTooLarge {
        /// Requested batch size.
        size: u32,
    },

    /// Ack batch size too large.
    #[snafu(display("ack batch size {size} exceeds maximum of {MAX_BATCH_ACK_SIZE}"))]
    AckBatchTooLarge {
        /// Requested batch size.
        size: usize,
    },

    /// Visibility timeout too large.
    #[snafu(display("visibility timeout {timeout_ms}ms exceeds maximum of {MAX_VISIBILITY_TIMEOUT_MS}ms"))]
    VisibilityTimeoutTooLarge {
        /// Requested timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Visibility timeout too small.
    #[snafu(display("visibility timeout {timeout_ms}ms is below minimum of {MIN_VISIBILITY_TIMEOUT_MS}ms"))]
    VisibilityTimeoutTooSmall {
        /// Requested timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Max delivery attempts too large.
    #[snafu(display("max delivery attempts {attempts} exceeds maximum of {MAX_DELIVERY_ATTEMPTS}"))]
    MaxDeliveryAttemptsTooLarge {
        /// Requested max delivery attempts.
        attempts: u32,
    },

    /// Max delivery attempts too small.
    #[snafu(display("max delivery attempts {attempts} is below minimum of {MIN_DELIVERY_ATTEMPTS}"))]
    MaxDeliveryAttemptsTooSmall {
        /// Requested max delivery attempts.
        attempts: u32,
    },

    /// Fencing token mismatch (stale consumer).
    #[snafu(display("fencing token mismatch for consumer '{consumer_id}': expected {expected}, got {actual}"))]
    FencingTokenMismatch {
        /// The consumer ID.
        consumer_id: String,
        /// Expected fencing token.
        expected: u64,
        /// Actual fencing token provided.
        actual: u64,
    },

    /// Session ID mismatch (stale consumer session).
    #[snafu(display("session ID mismatch for consumer '{consumer_id}': expected {expected}, got {actual}"))]
    SessionIdMismatch {
        /// The consumer ID.
        consumer_id: String,
        /// Expected session ID.
        expected: u64,
        /// Actual session ID provided.
        actual: u64,
    },

    /// Message not found or already processed.
    #[snafu(display("message with cursor {cursor} not found or already processed"))]
    MessageNotFound {
        /// The cursor of the message.
        cursor: u64,
    },

    /// Receipt handle invalid or expired.
    #[snafu(display("receipt handle invalid or expired: {reason}"))]
    InvalidReceiptHandle {
        /// Reason the receipt handle is invalid.
        reason: String,
    },

    /// Consumer session expired.
    #[snafu(display("consumer '{consumer_id}' session expired (last heartbeat: {last_heartbeat_ms}ms ago)"))]
    SessionExpired {
        /// The consumer ID.
        consumer_id: String,
        /// Time since last heartbeat in milliseconds.
        last_heartbeat_ms: u64,
    },

    /// Group is rebalancing.
    #[snafu(display("group '{group_id}' is rebalancing (generation {generation}), try again later"))]
    Rebalancing {
        /// The group ID.
        group_id: String,
        /// Current generation during rebalance.
        generation: u64,
    },

    /// Offset commit failed.
    #[snafu(display("offset commit failed for group '{group_id}': {reason}"))]
    OffsetCommitFailed {
        /// The group ID.
        group_id: String,
        /// Reason for failure.
        reason: String,
    },

    /// Invalid group ID.
    #[snafu(display("invalid group ID: {reason}"))]
    InvalidGroupId {
        /// Reason the group ID is invalid.
        reason: String,
    },

    /// Invalid consumer ID.
    #[snafu(display("invalid consumer ID: {reason}"))]
    InvalidConsumerId {
        /// Reason the consumer ID is invalid.
        reason: String,
    },

    /// Invalid configuration.
    #[snafu(display("invalid configuration: {reason}"))]
    InvalidConfig {
        /// Reason the configuration is invalid.
        reason: String,
    },

    /// Key-value store operation failed.
    #[snafu(display("key-value store operation failed: {source}"))]
    KvStoreFailed {
        /// The underlying KV store error.
        source: aspen_core::KeyValueStoreError,
    },

    /// Pub/sub operation failed.
    #[snafu(display("pub/sub operation failed: {source}"))]
    PubSubFailed {
        /// The underlying pub/sub error.
        source: crate::error::PubSubError,
    },

    /// Serialization failed.
    #[snafu(display("serialization failed: {message}"))]
    SerializationFailed {
        /// Error message.
        message: String,
    },

    /// Tuple operation failed.
    #[snafu(display("tuple operation failed: {source}"))]
    TupleFailed {
        /// The underlying tuple error.
        source: aspen_layer::TupleError,
    },

    /// Subspace operation failed.
    #[snafu(display("subspace operation failed: {source}"))]
    SubspaceFailed {
        /// The underlying subspace error.
        source: aspen_layer::SubspaceError,
    },

    /// Shutdown in progress.
    #[snafu(display("shutdown in progress"))]
    ShuttingDown,

    /// Log read operation failed.
    #[snafu(display("log read failed: {message}"))]
    LogReadFailed {
        /// Error message.
        message: String,
    },

    /// Internal error.
    #[snafu(display("internal error: {message}"))]
    Internal {
        /// Error message.
        message: String,
    },
}

impl From<aspen_core::KeyValueStoreError> for ConsumerGroupError {
    fn from(source: aspen_core::KeyValueStoreError) -> Self {
        ConsumerGroupError::KvStoreFailed { source }
    }
}

impl From<crate::error::PubSubError> for ConsumerGroupError {
    fn from(source: crate::error::PubSubError) -> Self {
        ConsumerGroupError::PubSubFailed { source }
    }
}

impl From<aspen_layer::TupleError> for ConsumerGroupError {
    fn from(source: aspen_layer::TupleError) -> Self {
        ConsumerGroupError::TupleFailed { source }
    }
}

impl From<aspen_layer::SubspaceError> for ConsumerGroupError {
    fn from(source: aspen_layer::SubspaceError) -> Self {
        ConsumerGroupError::SubspaceFailed { source }
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
            group_id: "test-group".to_string(),
        };
        assert!(err.to_string().contains("test-group"));

        let err = ConsumerGroupError::TooManyConsumers {
            group_id: "test-group".to_string(),
            count: 300,
        };
        assert!(err.to_string().contains("300"));
        assert!(err.to_string().contains(&MAX_CONSUMERS_PER_GROUP.to_string()));

        let err = ConsumerGroupError::FencingTokenMismatch {
            consumer_id: "consumer-1".to_string(),
            expected: 100,
            actual: 50,
        };
        assert!(err.to_string().contains("consumer-1"));
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_error_from_conversions() {
        // Test that From conversions compile
        fn accepts_error(_: ConsumerGroupError) {}

        let _ = |e: aspen_core::KeyValueStoreError| accepts_error(e.into());
        let _ = |e: crate::error::PubSubError| accepts_error(e.into());
        let _ = |e: aspen_layer::TupleError| accepts_error(e.into());
        let _ = |e: aspen_layer::SubspaceError| accepts_error(e.into());
    }
}
