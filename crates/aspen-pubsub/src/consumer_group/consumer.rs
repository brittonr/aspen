//! Consumer interface for consumer groups.
//!
//! Provides the GroupConsumer trait for receiving, acknowledging, and
//! processing messages from consumer groups.

use std::sync::Arc;
use std::time::Duration;

use aspen_core::KeyValueStore;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::consumer_group::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS;
use crate::consumer_group::constants::MAX_BATCH_ACK_SIZE;
use crate::consumer_group::constants::MAX_BATCH_RECEIVE_SIZE;
use crate::consumer_group::constants::MAX_PENDING_PER_CONSUMER;
use crate::consumer_group::error::ConsumerGroupError;
use crate::consumer_group::error::Result;
use crate::consumer_group::fencing::generate_fencing_token;
use crate::consumer_group::fencing::next_session_id;
use crate::consumer_group::fencing::validate_fencing;
use crate::consumer_group::pending::PendingEntriesManager;
use crate::consumer_group::storage;
use crate::consumer_group::types::AckResult;
use crate::consumer_group::types::BatchAckRequest;
use crate::consumer_group::types::BatchAckResult;
use crate::consumer_group::types::CommittedOffset;
use crate::consumer_group::types::ConsumerGroupId;
use crate::consumer_group::types::ConsumerId;
use crate::consumer_group::types::ConsumerState;
use crate::consumer_group::types::GroupMessage;
use crate::consumer_group::types::GroupStateType;
use crate::consumer_group::types::HeartbeatResponse;
use crate::consumer_group::types::JoinOptions;
use crate::consumer_group::types::MemberInfo;
use crate::consumer_group::types::NackResult;
use crate::consumer_group::types::PartitionId;
use crate::cursor::Cursor;

/// Consumer operations for interacting with a consumer group.
///
/// A GroupConsumer represents a single consumer's membership in a consumer group.
/// It provides methods for receiving messages, acknowledging processing, and
/// maintaining liveness through heartbeats.
///
/// # Lifecycle
///
/// 1. **Join**: Consumer registers with the group via `join_group()`
/// 2. **Receive**: Consumer pulls messages via `receive()`
/// 3. **Process + Ack**: Consumer acknowledges via `ack()` or `batch_ack()`
/// 4. **Heartbeat**: Consumer maintains membership via `heartbeat()`
/// 5. **Leave**: Consumer gracefully exits via `leave_group()`
///
/// # Fencing
///
/// All operations validate fencing tokens to prevent stale consumers from
/// interfering with active ones.
#[async_trait]
pub trait GroupConsumer: Send + Sync {
    /// Join a consumer group and receive initial state.
    ///
    /// Creates a new consumer session with monotonically increasing session_id
    /// and a fresh fencing_token for stale consumer detection.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The consumer group to join
    /// * `consumer_id` - Unique identifier for this consumer within the group
    /// * `options` - Optional join configuration (metadata, tags, visibility timeout)
    ///
    /// # Returns
    ///
    /// The consumer's membership information including fencing token.
    async fn join_group(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        options: JoinOptions,
    ) -> Result<MemberInfo>;

    /// Gracefully leave the consumer group.
    ///
    /// Releases pending messages. Idempotent - safe to call multiple times.
    async fn leave_group(&self, group_id: &ConsumerGroupId, consumer_id: &ConsumerId, fencing_token: u64)
    -> Result<()>;

    /// Send a heartbeat to maintain group membership.
    ///
    /// Must be called at least once per CONSUMER_HEARTBEAT_TIMEOUT_MS to
    /// prevent session expiration.
    async fn heartbeat(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<HeartbeatResponse>;

    /// Receive a batch of messages.
    ///
    /// Messages are returned with visibility timeout applied. The consumer
    /// must call `ack()` before the timeout expires or the message may
    /// become available for redelivery.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The consumer group
    /// * `consumer_id` - The consumer
    /// * `max_messages` - Maximum messages to return (capped at MAX_BATCH_RECEIVE_SIZE)
    /// * `wait_timeout` - How long to wait if no messages are immediately available
    /// * `fencing_token` - Current session's fencing token
    async fn receive(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        max_messages: u32,
        wait_timeout: Duration,
        fencing_token: u64,
    ) -> Result<Vec<GroupMessage>>;

    /// Acknowledge successful processing of a message.
    ///
    /// Removes the message from the pending entries list.
    async fn ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<AckResult>;

    /// Reject a message, returning it to the queue for redelivery.
    async fn nack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<NackResult>;

    /// Acknowledge multiple messages in a single operation.
    async fn batch_ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        request: BatchAckRequest,
        fencing_token: u64,
    ) -> Result<BatchAckResult>;

    /// Commit the consumer's position for a partition.
    ///
    /// Used in partitioned mode to track progress.
    async fn commit_offset(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        partition_id: PartitionId,
        cursor: Cursor,
        fencing_token: u64,
    ) -> Result<CommittedOffset>;

    /// Get the current consumer state.
    async fn get_consumer_state(&self, group_id: &ConsumerGroupId, consumer_id: &ConsumerId) -> Result<ConsumerState>;
}

/// Raft-backed implementation of GroupConsumer.
///
/// This implementation is stateless and can be shared across consumers.
/// All state is stored in the KV store.
pub struct RaftGroupConsumer<K: KeyValueStore + ?Sized, P: PendingEntriesManager> {
    /// KV store for persistence through Raft.
    store: Arc<K>,
    /// Pending entries manager.
    pending: Arc<P>,
    /// Cached group configuration (refreshed on operations).
    _group_cache: RwLock<std::collections::HashMap<String, crate::consumer_group::types::GroupState>>,
}

impl<K: KeyValueStore + ?Sized, P: PendingEntriesManager> RaftGroupConsumer<K, P> {
    /// Create a new group consumer.
    pub fn new(store: Arc<K>, pending: Arc<P>) -> Self {
        Self {
            store,
            pending,
            _group_cache: RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl<K: KeyValueStore + ?Sized + 'static, P: PendingEntriesManager + 'static> GroupConsumer
    for RaftGroupConsumer<K, P>
{
    async fn join_group(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        options: JoinOptions,
    ) -> Result<MemberInfo> {
        let now_ms = storage::now_unix_ms();

        // Load group state - verify group exists
        let group_state = storage::load_group_state(&*self.store, group_id).await?;

        // Check group is not dead
        if group_state.state == GroupStateType::Dead {
            return Err(ConsumerGroupError::GroupNotFound {
                group_id: group_id.as_str().to_string(),
            });
        }

        // Check capacity
        let consumer_count = storage::count_consumers(&*self.store, group_id).await?;
        if consumer_count >= crate::consumer_group::constants::MAX_CONSUMERS_PER_GROUP {
            return Err(ConsumerGroupError::TooManyConsumers {
                group_id: group_id.as_str().to_string(),
                count: consumer_count,
            });
        }

        // Check if consumer already exists (may be rejoining)
        let existing_state = storage::try_load_consumer_state(&*self.store, group_id, consumer_id).await?;

        // Generate new session and fencing token
        let session_id = next_session_id(existing_state.as_ref().map(|s| s.session_id));
        let fencing_token = generate_fencing_token();

        // Determine visibility timeout
        let _visibility_timeout_ms = options.visibility_timeout_ms.unwrap_or(group_state.visibility_timeout_ms);

        // Create consumer state
        let consumer_state = ConsumerState {
            consumer_id: consumer_id.clone(),
            group_id: group_id.clone(),
            assigned_partitions: vec![], // Competing mode - no partition assignment
            session_id,
            fencing_token,
            pending_count: 0,
            metadata: options.metadata,
            tags: options.tags,
            joined_at_ms: now_ms,
            last_heartbeat_ms: now_ms,
        };

        // Save consumer state
        storage::save_consumer_state(&*self.store, group_id, &consumer_state).await?;

        // Update group state if needed (first consumer, transition from Empty to Stable)
        if group_state.state == GroupStateType::Empty {
            let mut updated_group = group_state.clone();
            updated_group.state = GroupStateType::Stable;
            updated_group.member_count = 1;
            updated_group.updated_at_ms = now_ms;
            storage::save_group_state(&*self.store, &updated_group).await?;
        } else {
            // Just update member count
            let mut updated_group = group_state.clone();
            updated_group.member_count = consumer_count + 1;
            updated_group.updated_at_ms = now_ms;
            storage::save_group_state(&*self.store, &updated_group).await?;
        }

        // Build MemberInfo response
        Ok(MemberInfo {
            consumer_id: consumer_id.clone(),
            group_id: group_id.clone(),
            fencing_token,
            session_id,
            generation_id: group_state.generation_id,
            metadata: consumer_state.metadata.clone(),
            tags: consumer_state.tags.clone(),
            joined_at_ms: now_ms,
            last_heartbeat_ms: now_ms,
            heartbeat_deadline_ms: now_ms + CONSUMER_HEARTBEAT_TIMEOUT_MS,
            pending_count: 0,
            assigned_partitions: vec![],
        })
    }

    async fn leave_group(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<()> {
        let now_ms = storage::now_unix_ms();

        // Load and validate consumer state
        let consumer_state = match storage::try_load_consumer_state(&*self.store, group_id, consumer_id).await? {
            Some(s) => s,
            None => return Ok(()), // Already left - idempotent
        };

        // Validate fencing
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Delete consumer state
        storage::delete_consumer_state(&*self.store, group_id, consumer_id).await?;

        // Update group member count
        let group_state = storage::load_group_state(&*self.store, group_id).await?;
        let new_count = group_state.member_count.saturating_sub(1);

        let mut updated_group = group_state;
        updated_group.member_count = new_count;
        updated_group.updated_at_ms = now_ms;

        // Transition to Empty if no consumers left
        if new_count == 0 {
            updated_group.state = GroupStateType::Empty;
        }

        storage::save_group_state(&*self.store, &updated_group).await?;

        Ok(())
    }

    async fn heartbeat(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<HeartbeatResponse> {
        let now_ms = storage::now_unix_ms();

        // Load consumer state
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;

        // Validate fencing
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Check if session has expired
        if consumer_state.is_expired(CONSUMER_HEARTBEAT_TIMEOUT_MS, now_ms) {
            return Err(ConsumerGroupError::SessionExpired {
                consumer_id: consumer_id.as_str().to_string(),
                last_heartbeat_ms: now_ms - consumer_state.last_heartbeat_ms,
            });
        }

        // Update last heartbeat time
        let mut updated_state = consumer_state.clone();
        updated_state.last_heartbeat_ms = now_ms;
        storage::save_consumer_state(&*self.store, group_id, &updated_state).await?;

        // Load group state for rebalancing info
        let group_state = storage::load_group_state(&*self.store, group_id).await?;
        let rebalancing =
            matches!(group_state.state, GroupStateType::PreparingRebalance | GroupStateType::CompletingRebalance);

        Ok(HeartbeatResponse {
            next_deadline_ms: now_ms + CONSUMER_HEARTBEAT_TIMEOUT_MS,
            rebalancing,
            generation_id: group_state.generation_id,
            assigned_partitions: consumer_state.assigned_partitions,
        })
    }

    async fn receive(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        max_messages: u32,
        _wait_timeout: Duration,
        fencing_token: u64,
    ) -> Result<Vec<GroupMessage>> {
        // Validate batch size
        if max_messages > MAX_BATCH_RECEIVE_SIZE {
            return Err(ConsumerGroupError::ReceiveBatchTooLarge { size: max_messages });
        }

        let now_ms = storage::now_unix_ms();

        // Load and validate consumer state
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Check if session has expired
        if consumer_state.is_expired(CONSUMER_HEARTBEAT_TIMEOUT_MS, now_ms) {
            return Err(ConsumerGroupError::SessionExpired {
                consumer_id: consumer_id.as_str().to_string(),
                last_heartbeat_ms: now_ms - consumer_state.last_heartbeat_ms,
            });
        }

        // Check pending count limit
        if consumer_state.pending_count >= MAX_PENDING_PER_CONSUMER {
            return Err(ConsumerGroupError::TooManyPending {
                consumer_id: consumer_id.as_str().to_string(),
                count: consumer_state.pending_count,
            });
        }

        // Load group state
        let group_state = storage::load_group_state(&*self.store, group_id).await?;

        // Check for rebalancing
        if matches!(group_state.state, GroupStateType::PreparingRebalance | GroupStateType::CompletingRebalance) {
            return Err(ConsumerGroupError::Rebalancing {
                group_id: group_id.as_str().to_string(),
                generation: group_state.generation_id,
            });
        }

        // TODO: Implement actual message fetching from the pub/sub layer.
        // This requires integration with the EventStream subscriber.
        // For now, return empty - the actual implementation will:
        // 1. Subscribe to topics matching group_state.pattern
        // 2. Fetch messages from the committed offset or beginning
        // 3. For each message:
        //    a. If AutoAck: return immediately (no pending entry)
        //    b. If Explicit: create pending entry, return with receipt handle
        //
        // Competing mode: any available message
        // Partitioned mode: only from assigned partitions (Phase 2B)
        let _ack_policy = group_state.ack_policy;

        Ok(vec![])
    }

    async fn ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<AckResult> {
        // Validate consumer still exists and fencing token matches
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Delegate to pending entries manager
        self.pending.ack(group_id, receipt_handle).await
    }

    async fn nack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<NackResult> {
        // Validate consumer still exists and fencing token matches
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Load group for max delivery attempts
        let group_state = storage::load_group_state(&*self.store, group_id).await?;

        // Delegate to pending entries manager
        self.pending.nack(group_id, receipt_handle, group_state.max_delivery_attempts).await
    }

    async fn batch_ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        request: BatchAckRequest,
        fencing_token: u64,
    ) -> Result<BatchAckResult> {
        // Validate batch size
        if request.receipt_handles.len() > MAX_BATCH_ACK_SIZE {
            return Err(ConsumerGroupError::AckBatchTooLarge {
                size: request.receipt_handles.len(),
            });
        }

        // Validate consumer still exists and fencing token matches
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Delegate to pending entries manager
        self.pending.batch_ack(group_id, request).await
    }

    async fn commit_offset(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        partition_id: PartitionId,
        cursor: Cursor,
        fencing_token: u64,
    ) -> Result<CommittedOffset> {
        let now_ms = storage::now_unix_ms();

        // Validate consumer still exists and fencing token matches
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Create committed offset
        let offset = CommittedOffset {
            group_id: group_id.clone(),
            partition_id,
            cursor,
            committed_at_ms: now_ms,
            metadata: None,
        };

        // Save to storage
        let key = crate::consumer_group::keys::ConsumerGroupKeys::offset_key(group_id, partition_id);
        let key_str = crate::consumer_group::keys::ConsumerGroupKeys::key_to_string(&key);
        let value = rmp_serde::to_vec(&offset).map_err(|e| ConsumerGroupError::SerializationFailed {
            message: format!("failed to serialize offset: {}", e),
        })?;
        let value_str = String::from_utf8_lossy(&value).into_owned();

        self.store
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Set {
                    key: key_str,
                    value: value_str,
                },
            })
            .await?;

        Ok(offset)
    }

    async fn get_consumer_state(&self, group_id: &ConsumerGroupId, consumer_id: &ConsumerId) -> Result<ConsumerState> {
        storage::load_consumer_state(&*self.store, group_id, consumer_id).await
    }
}

#[cfg(test)]
mod tests {
    // Integration tests will be in a separate file since they require a KV store
}
