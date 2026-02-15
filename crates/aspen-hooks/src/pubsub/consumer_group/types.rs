//! Core types for consumer groups.
//!
//! This module defines the fundamental types used throughout the consumer group
//! implementation, including identifiers, configuration, state, and message types.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

use serde::Deserialize;
use serde::Serialize;

use super::super::cursor::Cursor;
use super::super::event::Event;
use super::super::topic::TopicPattern;
use super::constants::DEFAULT_MAX_DELIVERY_ATTEMPTS;
use super::constants::DEFAULT_VISIBILITY_TIMEOUT_MS;
use super::constants::MAX_DELIVERY_ATTEMPTS;
use super::constants::MAX_PARTITIONS_PER_GROUP;
use super::constants::MAX_VISIBILITY_TIMEOUT_MS;
use super::constants::MIN_DELIVERY_ATTEMPTS;
use super::constants::MIN_VISIBILITY_TIMEOUT_MS;
use super::error::ConsumerGroupError;
use super::error::Result;

// =============================================================================
// Identifiers
// =============================================================================

/// Consumer group identifier.
///
/// Group IDs must be non-empty and contain only alphanumeric characters,
/// hyphens, and underscores. Maximum length is 128 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsumerGroupId(String);

impl ConsumerGroupId {
    /// Maximum length of a group ID.
    pub const MAX_LENGTH: usize = 128;

    /// Create a new consumer group ID.
    ///
    /// # Errors
    ///
    /// Returns error if the ID is empty, too long, or contains invalid characters.
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        Self::validate(&id)?;
        Ok(Self(id))
    }

    /// Create a new consumer group ID without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the ID is valid. Use `new()` for validated construction.
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Validate a group ID string.
    pub fn validate(id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(ConsumerGroupError::InvalidGroupId {
                reason: "group ID cannot be empty".to_string(),
            });
        }

        if id.len() > Self::MAX_LENGTH {
            return Err(ConsumerGroupError::InvalidGroupId {
                reason: format!("group ID '{}' has {} characters, maximum is {}", id, id.len(), Self::MAX_LENGTH),
            });
        }

        for c in id.chars() {
            if !c.is_alphanumeric() && c != '-' && c != '_' {
                return Err(ConsumerGroupError::InvalidGroupId {
                    reason: format!("group ID '{}' contains invalid character '{}'", id, c),
                });
            }
        }

        Ok(())
    }

    /// Get the group ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ConsumerGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ConsumerGroupId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Consumer identifier within a group.
///
/// Consumer IDs must be non-empty and contain only alphanumeric characters,
/// hyphens, and underscores. Maximum length is 128 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsumerId(String);

impl ConsumerId {
    /// Maximum length of a consumer ID.
    pub const MAX_LENGTH: usize = 128;

    /// Create a new consumer ID.
    ///
    /// # Errors
    ///
    /// Returns error if the ID is empty, too long, or contains invalid characters.
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        Self::validate(&id)?;
        Ok(Self(id))
    }

    /// Create a new consumer ID without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure the ID is valid. Use `new()` for validated construction.
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Validate a consumer ID string.
    pub fn validate(id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(ConsumerGroupError::InvalidConsumerId {
                reason: "consumer ID cannot be empty".to_string(),
            });
        }

        if id.len() > Self::MAX_LENGTH {
            return Err(ConsumerGroupError::InvalidConsumerId {
                reason: format!("consumer ID '{}' has {} characters, maximum is {}", id, id.len(), Self::MAX_LENGTH),
            });
        }

        for c in id.chars() {
            if !c.is_alphanumeric() && c != '-' && c != '_' {
                return Err(ConsumerGroupError::InvalidConsumerId {
                    reason: format!("consumer ID '{}' contains invalid character '{}'", id, c),
                });
            }
        }

        Ok(())
    }

    /// Get the consumer ID as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ConsumerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ConsumerId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Partition identifier.
///
/// In partitioned mode, messages are assigned to partitions, and partitions
/// are assigned to consumers. This enables per-partition ordering guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct PartitionId(pub u32);

impl PartitionId {
    /// Create a new partition ID.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the partition ID value.
    #[inline]
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for PartitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for PartitionId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

// =============================================================================
// Configuration Enums
// =============================================================================

/// Assignment mode for consumer groups.
///
/// Determines how messages are distributed to consumers within a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AssignmentMode {
    /// Competing consumers mode (default).
    ///
    /// Any consumer can pull any message. This is similar to NATS JetStream's
    /// pull-based delivery. Simpler to implement, no rebalancing needed, but
    /// provides no ordering guarantees.
    #[default]
    Competing,

    /// Partitioned mode.
    ///
    /// Messages are assigned to partitions (based on message key or round-robin),
    /// and partitions are assigned to consumers. Similar to Kafka. Provides
    /// per-partition ordering guarantees but requires rebalancing when consumers
    /// join or leave.
    Partitioned,
}

impl fmt::Display for AssignmentMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Competing => write!(f, "competing"),
            Self::Partitioned => write!(f, "partitioned"),
        }
    }
}

/// Acknowledgment policy for consumer groups.
///
/// Determines how message delivery is confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AckPolicy {
    /// Explicit acknowledgment required (default).
    ///
    /// Consumers must explicitly acknowledge each message after processing.
    /// Messages not acknowledged within the visibility timeout are redelivered
    /// to another consumer. Provides at-least-once delivery semantics.
    #[default]
    Explicit,

    /// Automatic acknowledgment on receive.
    ///
    /// Messages are acknowledged automatically when received by a consumer.
    /// Simpler but may lose messages if the consumer crashes after receiving
    /// but before processing. Provides at-most-once delivery semantics.
    AutoAck,
}

impl fmt::Display for AckPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Explicit => write!(f, "explicit"),
            Self::AutoAck => write!(f, "auto-ack"),
        }
    }
}

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for creating a consumer group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupConfig {
    /// Unique group identifier.
    pub group_id: ConsumerGroupId,

    /// Topic pattern string to subscribe to.
    ///
    /// Stored as a string for serialization; use `pattern()` to get the parsed `TopicPattern`.
    pub pattern_str: String,

    /// Assignment mode (competing or partitioned).
    pub assignment_mode: AssignmentMode,

    /// Acknowledgment policy.
    pub ack_policy: AckPolicy,

    /// Number of partitions (only used in partitioned mode).
    pub partition_count: u32,

    /// Visibility timeout in milliseconds.
    ///
    /// How long a message is invisible to other consumers after being received.
    /// NATS JetStream calls this "AckWait".
    pub visibility_timeout_ms: u64,

    /// Maximum delivery attempts before dead-lettering.
    ///
    /// Set to 0 for unlimited attempts. NATS JetStream calls this "MaxDeliver".
    pub max_delivery_attempts: u32,

    /// Consumer heartbeat timeout in milliseconds.
    ///
    /// If a consumer doesn't send a heartbeat within this period, it's
    /// considered dead.
    pub consumer_heartbeat_timeout_ms: u64,
}

impl ConsumerGroupConfig {
    /// Get the parsed topic pattern.
    ///
    /// # Errors
    ///
    /// Returns error if the stored pattern string is invalid.
    pub fn pattern(&self) -> Result<TopicPattern> {
        TopicPattern::new(&self.pattern_str).map_err(|e| super::error::ConsumerGroupError::InvalidConfig {
            reason: format!("invalid pattern '{}': {}", self.pattern_str, e),
        })
    }
}

impl ConsumerGroupConfig {
    /// Create a new configuration builder.
    pub fn builder() -> ConsumerGroupConfigBuilder {
        ConsumerGroupConfigBuilder::new()
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate pattern is parseable
        let _pattern = self.pattern()?;

        if self.visibility_timeout_ms < MIN_VISIBILITY_TIMEOUT_MS {
            return Err(ConsumerGroupError::VisibilityTimeoutTooSmall {
                timeout_ms: self.visibility_timeout_ms,
            });
        }

        if self.visibility_timeout_ms > MAX_VISIBILITY_TIMEOUT_MS {
            return Err(ConsumerGroupError::VisibilityTimeoutTooLarge {
                timeout_ms: self.visibility_timeout_ms,
            });
        }

        if self.max_delivery_attempts < MIN_DELIVERY_ATTEMPTS {
            return Err(ConsumerGroupError::MaxDeliveryAttemptsTooSmall {
                attempts: self.max_delivery_attempts,
            });
        }

        if self.max_delivery_attempts > MAX_DELIVERY_ATTEMPTS {
            return Err(ConsumerGroupError::MaxDeliveryAttemptsTooLarge {
                attempts: self.max_delivery_attempts,
            });
        }

        if self.assignment_mode == AssignmentMode::Partitioned && self.partition_count > MAX_PARTITIONS_PER_GROUP {
            return Err(ConsumerGroupError::TooManyPartitions {
                group_id: self.group_id.as_str().to_string(),
                count: self.partition_count,
            });
        }

        Ok(())
    }
}

impl Default for ConsumerGroupConfig {
    fn default() -> Self {
        Self {
            group_id: ConsumerGroupId::new_unchecked("default"),
            pattern_str: ">".to_string(), // Matches all topics
            assignment_mode: AssignmentMode::default(),
            ack_policy: AckPolicy::default(),
            partition_count: 1,
            visibility_timeout_ms: DEFAULT_VISIBILITY_TIMEOUT_MS,
            max_delivery_attempts: DEFAULT_MAX_DELIVERY_ATTEMPTS,
            consumer_heartbeat_timeout_ms: super::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS,
        }
    }
}

/// Builder for consumer group configuration.
#[derive(Debug, Default)]
pub struct ConsumerGroupConfigBuilder {
    group_id: Option<ConsumerGroupId>,
    pattern_str: Option<String>,
    assignment_mode: AssignmentMode,
    ack_policy: AckPolicy,
    partition_count: u32,
    visibility_timeout_ms: u64,
    max_delivery_attempts: u32,
    consumer_heartbeat_timeout_ms: u64,
}

impl ConsumerGroupConfigBuilder {
    /// Create a new builder with defaults.
    pub fn new() -> Self {
        Self {
            group_id: None,
            pattern_str: None,
            assignment_mode: AssignmentMode::default(),
            ack_policy: AckPolicy::default(),
            partition_count: 1,
            visibility_timeout_ms: DEFAULT_VISIBILITY_TIMEOUT_MS,
            max_delivery_attempts: DEFAULT_MAX_DELIVERY_ATTEMPTS,
            consumer_heartbeat_timeout_ms: super::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS,
        }
    }

    /// Set the group ID.
    pub fn group_id(mut self, group_id: impl Into<String>) -> Result<Self> {
        self.group_id = Some(ConsumerGroupId::new(group_id)?);
        Ok(self)
    }

    /// Set the group ID without validation.
    pub fn group_id_unchecked(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = Some(ConsumerGroupId::new_unchecked(group_id));
        self
    }

    /// Set the topic pattern.
    pub fn pattern(mut self, pattern: TopicPattern) -> Self {
        self.pattern_str = Some(pattern.as_str().to_string());
        self
    }

    /// Set the topic pattern from a string.
    pub fn pattern_str(mut self, pattern: impl Into<String>) -> Self {
        self.pattern_str = Some(pattern.into());
        self
    }

    /// Set the assignment mode.
    pub fn assignment_mode(mut self, mode: AssignmentMode) -> Self {
        self.assignment_mode = mode;
        self
    }

    /// Set the acknowledgment policy.
    pub fn ack_policy(mut self, policy: AckPolicy) -> Self {
        self.ack_policy = policy;
        self
    }

    /// Set the number of partitions (for partitioned mode).
    pub fn partition_count(mut self, count: u32) -> Self {
        self.partition_count = count;
        self
    }

    /// Set the visibility timeout in milliseconds.
    pub fn visibility_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.visibility_timeout_ms = timeout_ms;
        self
    }

    /// Set the maximum delivery attempts.
    pub fn max_delivery_attempts(mut self, attempts: u32) -> Self {
        self.max_delivery_attempts = attempts;
        self
    }

    /// Set the consumer heartbeat timeout in milliseconds.
    pub fn consumer_heartbeat_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.consumer_heartbeat_timeout_ms = timeout_ms;
        self
    }

    /// Build the configuration.
    ///
    /// # Errors
    ///
    /// Returns error if group_id or pattern is not set, or if configuration is invalid.
    pub fn build(self) -> Result<ConsumerGroupConfig> {
        let group_id = self.group_id.ok_or_else(|| ConsumerGroupError::InvalidConfig {
            reason: "group_id is required".to_string(),
        })?;

        let pattern_str = self.pattern_str.ok_or_else(|| ConsumerGroupError::InvalidConfig {
            reason: "pattern is required".to_string(),
        })?;

        let config = ConsumerGroupConfig {
            group_id,
            pattern_str,
            assignment_mode: self.assignment_mode,
            ack_policy: self.ack_policy,
            partition_count: self.partition_count,
            visibility_timeout_ms: self.visibility_timeout_ms,
            max_delivery_attempts: self.max_delivery_attempts,
            consumer_heartbeat_timeout_ms: self.consumer_heartbeat_timeout_ms,
        };

        config.validate()?;
        Ok(config)
    }
}

// =============================================================================
// Group State
// =============================================================================

/// State of a consumer group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GroupStateType {
    /// Group is empty (no active consumers).
    #[default]
    Empty,

    /// Group is preparing for rebalance.
    PreparingRebalance,

    /// Group is completing rebalance.
    CompletingRebalance,

    /// Group is stable and consuming.
    Stable,

    /// Group is dead (deleted or expired).
    Dead,
}

impl fmt::Display for GroupStateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty"),
            Self::PreparingRebalance => write!(f, "preparing-rebalance"),
            Self::CompletingRebalance => write!(f, "completing-rebalance"),
            Self::Stable => write!(f, "stable"),
            Self::Dead => write!(f, "dead"),
        }
    }
}

/// Full state of a consumer group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupState {
    /// Group identifier.
    pub group_id: ConsumerGroupId,

    /// Topic pattern string.
    pub pattern_str: String,

    /// Current state type.
    pub state: GroupStateType,

    /// Current generation ID (increments on each rebalance).
    pub generation_id: u64,

    /// Assignment mode.
    pub assignment_mode: AssignmentMode,

    /// Acknowledgment policy.
    pub ack_policy: AckPolicy,

    /// Visibility timeout in milliseconds.
    pub visibility_timeout_ms: u64,

    /// Maximum delivery attempts.
    pub max_delivery_attempts: u32,

    /// Number of partitions (for partitioned mode).
    pub partition_count: u32,

    /// Group leader consumer ID (if any).
    pub leader_id: Option<ConsumerId>,

    /// Active member count.
    pub member_count: u32,

    /// Creation time (Unix ms).
    pub created_at_ms: u64,

    /// Last modification time (Unix ms).
    pub updated_at_ms: u64,
}

impl GroupState {
    /// Get the parsed topic pattern.
    ///
    /// # Errors
    ///
    /// Returns error if the stored pattern string is invalid.
    pub fn pattern(&self) -> Result<TopicPattern> {
        TopicPattern::new(&self.pattern_str).map_err(|e| super::error::ConsumerGroupError::InvalidConfig {
            reason: format!("invalid pattern '{}': {}", self.pattern_str, e),
        })
    }
}

// =============================================================================
// Consumer State
// =============================================================================

/// State of a consumer within a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerState {
    /// Consumer identifier.
    pub consumer_id: ConsumerId,

    /// Group this consumer belongs to.
    pub group_id: ConsumerGroupId,

    /// Assigned partitions (in partitioned mode).
    pub assigned_partitions: Vec<PartitionId>,

    /// Session ID (monotonically increasing, used for fencing).
    pub session_id: u64,

    /// Fencing token (used to detect stale consumers).
    pub fencing_token: u64,

    /// Number of pending (unacknowledged) messages.
    pub pending_count: u32,

    /// Consumer metadata (e.g., hostname, version).
    pub metadata: Option<String>,

    /// Consumer tags for filtering.
    pub tags: Vec<String>,

    /// Join time (Unix ms).
    pub joined_at_ms: u64,

    /// Last heartbeat time (Unix ms).
    pub last_heartbeat_ms: u64,
}

impl ConsumerState {
    /// Check if this consumer's session has expired based on the timeout.
    pub fn is_expired(&self, timeout_ms: u64, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.last_heartbeat_ms) > timeout_ms
    }
}

/// Options for joining a consumer group.
#[derive(Debug, Clone, Default)]
pub struct JoinOptions {
    /// Custom metadata for this consumer (e.g., hostname, version).
    pub metadata: Option<String>,

    /// Tags for routing/filtering.
    pub tags: Vec<String>,

    /// Override the default visibility timeout for this consumer.
    pub visibility_timeout_ms: Option<u64>,
}

/// Information about a consumer group member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    /// Consumer identifier.
    pub consumer_id: ConsumerId,

    /// Group identifier.
    pub group_id: ConsumerGroupId,

    /// Fencing token for this membership.
    pub fencing_token: u64,

    /// Session ID.
    pub session_id: u64,

    /// Generation ID when joined.
    pub generation_id: u64,

    /// Custom metadata.
    pub metadata: Option<String>,

    /// Tags.
    pub tags: Vec<String>,

    /// Join time (Unix ms).
    pub joined_at_ms: u64,

    /// Last heartbeat time (Unix ms).
    pub last_heartbeat_ms: u64,

    /// Heartbeat deadline (Unix ms).
    pub heartbeat_deadline_ms: u64,

    /// Number of pending messages.
    pub pending_count: u32,

    /// Assigned partitions (in partitioned mode).
    pub assigned_partitions: Vec<PartitionId>,
}

/// Response from a heartbeat operation.
#[derive(Debug, Clone)]
pub struct HeartbeatResponse {
    /// Next heartbeat deadline (Unix ms).
    pub next_deadline_ms: u64,

    /// Whether a rebalance is in progress.
    pub rebalancing: bool,

    /// Current generation ID.
    pub generation_id: u64,

    /// Assigned partitions (may change during rebalance).
    pub assigned_partitions: Vec<PartitionId>,
}

// =============================================================================
// Pending Entry
// =============================================================================

/// A pending (in-flight) message entry.
///
/// Stored in the Pending Entries List (PEL) to track messages that have been
/// delivered to a consumer but not yet acknowledged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEntry {
    /// Message cursor (Raft log index).
    pub cursor: u64,

    /// Consumer ID that received this message.
    pub consumer_id: ConsumerId,

    /// Delivery attempt number (starts at 1).
    pub delivery_attempt: u32,

    /// When this message was delivered (Unix ms).
    pub delivered_at_ms: u64,

    /// Visibility timeout deadline (Unix ms).
    pub visibility_deadline_ms: u64,

    /// Partition this message belongs to (in partitioned mode).
    pub partition_id: PartitionId,

    /// Receipt handle for acknowledgment.
    pub receipt_handle: String,
}

impl PendingEntry {
    /// Check if the visibility timeout has expired.
    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms > self.visibility_deadline_ms
    }
}

// =============================================================================
// Dead Letter Entry
// =============================================================================

/// An entry in the dead letter queue.
///
/// Messages are moved to the DLQ when they exceed the maximum delivery attempts.
/// The entry preserves information about the original delivery for debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    /// Message cursor (Raft log index).
    pub cursor: u64,

    /// Consumer ID that last held the message.
    pub original_consumer_id: ConsumerId,

    /// Total delivery attempts before dead-lettering.
    pub delivery_attempts: u32,

    /// When first delivered (Unix ms).
    pub first_delivered_at_ms: u64,

    /// When moved to DLQ (Unix ms).
    pub dead_lettered_at_ms: u64,

    /// Partition for the message.
    pub partition_id: PartitionId,

    /// Reason for dead-lettering (e.g., "max_attempts_exceeded").
    pub reason: String,
}

// =============================================================================
// Message Types
// =============================================================================

/// A message delivered to a consumer group member.
#[derive(Debug, Clone)]
pub struct GroupMessage {
    /// Original event.
    pub event: Event,

    /// Partition this came from (in partitioned mode).
    pub partition_id: PartitionId,

    /// Receipt handle for acknowledgment.
    pub receipt_handle: String,

    /// Delivery attempt number (starts at 1).
    pub delivery_count: u32,

    /// Visibility timeout deadline (Unix ms).
    pub visibility_deadline_ms: u64,
}

impl GroupMessage {
    /// Check if visibility timeout has expired.
    pub fn is_visibility_expired(&self, now_ms: u64) -> bool {
        now_ms > self.visibility_deadline_ms
    }

    /// Get remaining visibility time in milliseconds.
    pub fn remaining_visibility_ms(&self, now_ms: u64) -> u64 {
        self.visibility_deadline_ms.saturating_sub(now_ms)
    }

    /// Get message cursor.
    pub fn cursor(&self) -> Cursor {
        self.event.cursor
    }
}

/// Committed offset for a group/partition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommittedOffset {
    /// Group ID.
    pub group_id: ConsumerGroupId,

    /// Partition ID (0 for competing mode).
    pub partition_id: PartitionId,

    /// Committed cursor position.
    pub cursor: Cursor,

    /// When this offset was committed (Unix ms).
    pub committed_at_ms: u64,

    /// Optional metadata associated with this commit.
    pub metadata: Option<String>,
}

// =============================================================================
// Acknowledgment Types
// =============================================================================

/// Result of an acknowledge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckResult {
    /// Message successfully acknowledged and removed.
    Success,

    /// Message not found (already acked or expired).
    NotFound,

    /// Receipt handle invalid or expired.
    InvalidHandle,

    /// Fencing token mismatch (stale consumer).
    Fenced,
}

impl AckResult {
    /// Check if the acknowledgment was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

/// Result of a negative acknowledge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NackResult {
    /// Message returned to queue for redelivery.
    Requeued,

    /// Message moved to dead letter queue.
    DeadLettered,

    /// Message not found.
    NotFound,

    /// Receipt handle invalid.
    InvalidHandle,
}

/// Batch acknowledgment request.
#[derive(Debug, Clone, Default)]
pub struct BatchAckRequest {
    /// Receipt handles to acknowledge.
    pub receipt_handles: Vec<String>,
}

impl BatchAckRequest {
    /// Create a new batch ack request.
    pub fn new(receipt_handles: Vec<String>) -> Self {
        Self { receipt_handles }
    }
}

/// Batch acknowledgment result.
#[derive(Debug, Clone, Default)]
pub struct BatchAckResult {
    /// Number of messages successfully acknowledged.
    pub success_count: u32,

    /// Number of messages that failed acknowledgment.
    pub failure_count: u32,

    /// Individual failure details (receipt_handle -> result).
    pub failures: HashMap<String, AckResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumer_group_id_validation() {
        // Valid IDs
        assert!(ConsumerGroupId::new("my-group").is_ok());
        assert!(ConsumerGroupId::new("my_group_123").is_ok());
        assert!(ConsumerGroupId::new("a").is_ok());

        // Invalid IDs
        assert!(ConsumerGroupId::new("").is_err());
        assert!(ConsumerGroupId::new("my group").is_err()); // space
        assert!(ConsumerGroupId::new("my.group").is_err()); // dot
        assert!(ConsumerGroupId::new("my/group").is_err()); // slash

        // Too long
        let long_id = "a".repeat(ConsumerGroupId::MAX_LENGTH + 1);
        assert!(ConsumerGroupId::new(long_id).is_err());
    }

    #[test]
    fn test_consumer_id_validation() {
        // Valid IDs
        assert!(ConsumerId::new("consumer-1").is_ok());
        assert!(ConsumerId::new("consumer_2_test").is_ok());

        // Invalid IDs
        assert!(ConsumerId::new("").is_err());
        assert!(ConsumerId::new("consumer 1").is_err());
    }

    #[test]
    fn test_partition_id() {
        let p1 = PartitionId::new(0);
        let p2 = PartitionId::from(1u32);

        assert_eq!(p1.value(), 0);
        assert_eq!(p2.value(), 1);
        assert!(p1 < p2);
    }

    #[test]
    fn test_assignment_mode_display() {
        assert_eq!(AssignmentMode::Competing.to_string(), "competing");
        assert_eq!(AssignmentMode::Partitioned.to_string(), "partitioned");
    }

    #[test]
    fn test_ack_policy_display() {
        assert_eq!(AckPolicy::Explicit.to_string(), "explicit");
        assert_eq!(AckPolicy::AutoAck.to_string(), "auto-ack");
    }

    #[test]
    fn test_config_builder() {
        let config = ConsumerGroupConfig::builder()
            .group_id_unchecked("test-group")
            .pattern(TopicPattern::all())
            .assignment_mode(AssignmentMode::Competing)
            .ack_policy(AckPolicy::Explicit)
            .visibility_timeout_ms(30_000)
            .max_delivery_attempts(5)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.group_id.as_str(), "test-group");
        assert_eq!(config.assignment_mode, AssignmentMode::Competing);
        assert_eq!(config.ack_policy, AckPolicy::Explicit);
    }

    #[test]
    fn test_config_validation() {
        // Visibility timeout too large
        let config = ConsumerGroupConfig {
            visibility_timeout_ms: MAX_VISIBILITY_TIMEOUT_MS + 1,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Max delivery attempts too large
        let config = ConsumerGroupConfig {
            max_delivery_attempts: MAX_DELIVERY_ATTEMPTS + 1,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Too many partitions
        let config = ConsumerGroupConfig {
            assignment_mode: AssignmentMode::Partitioned,
            partition_count: MAX_PARTITIONS_PER_GROUP + 1,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_consumer_state_expiry() {
        let state = ConsumerState {
            consumer_id: ConsumerId::new_unchecked("c1"),
            group_id: ConsumerGroupId::new_unchecked("g1"),
            assigned_partitions: vec![],
            session_id: 1,
            fencing_token: 1,
            pending_count: 0,
            metadata: None,
            tags: vec![],
            joined_at_ms: 1000,
            last_heartbeat_ms: 1000,
        };

        // Not expired yet
        assert!(!state.is_expired(30_000, 1000));
        assert!(!state.is_expired(30_000, 30_000));

        // Expired
        assert!(state.is_expired(30_000, 31_001));
    }

    #[test]
    fn test_pending_entry_expiry() {
        let entry = PendingEntry {
            cursor: 100,
            consumer_id: ConsumerId::new_unchecked("c1"),
            delivery_attempt: 1,
            delivered_at_ms: 1000,
            visibility_deadline_ms: 31_000,
            partition_id: PartitionId::new(0),
            receipt_handle: "handle".to_string(),
        };

        assert!(!entry.is_expired(30_000));
        assert!(!entry.is_expired(31_000));
        assert!(entry.is_expired(31_001));
    }

    #[test]
    fn test_ack_result() {
        assert!(AckResult::Success.is_success());
        assert!(!AckResult::NotFound.is_success());
        assert!(!AckResult::InvalidHandle.is_success());
        assert!(!AckResult::Fenced.is_success());
    }

    #[test]
    fn test_group_state_type_display() {
        assert_eq!(GroupStateType::Empty.to_string(), "empty");
        assert_eq!(GroupStateType::Stable.to_string(), "stable");
        assert_eq!(GroupStateType::PreparingRebalance.to_string(), "preparing-rebalance");
    }
}
