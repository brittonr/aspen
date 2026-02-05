//! Consumer client for interacting with consumer groups.
//!
//! Provides a high-level interface for consumers to join groups,
//! receive messages, and acknowledge processing.

use std::sync::Arc;

use aspen_core::KeyValueStore;

use super::error::ConsumerGroupError;
use super::error::Result;
use super::fencing::generate_fencing_token;
use super::pending::DeliveryParams;
use super::pending::PendingEntriesManager;
use super::storage;
use super::types::AckResult;
use super::types::BatchAckRequest;
use super::types::BatchAckResult;
use super::types::ConsumerGroupId;
use super::types::ConsumerId;
use super::types::ConsumerState;
use super::types::GroupStateType;
use super::types::HeartbeatResponse;
use super::types::JoinOptions;
use super::types::MemberInfo;
use super::types::NackResult;
use super::types::PartitionId;

/// Consumer client for a consumer group.
///
/// Manages the consumer's membership in a group and provides
/// methods for receiving and acknowledging messages.
pub struct Consumer<K: KeyValueStore + ?Sized, P: PendingEntriesManager> {
    store: Arc<K>,
    pending_manager: Arc<P>,
    group_id: ConsumerGroupId,
    consumer_id: ConsumerId,
    fencing_token: u64,
    session_id: u64,
}

impl<K: KeyValueStore + ?Sized + 'static, P: PendingEntriesManager + 'static> Consumer<K, P> {
    /// Join a consumer group.
    ///
    /// Creates a new consumer instance and registers it with the group.
    ///
    /// # Arguments
    ///
    /// * `store` - KV store for state persistence
    /// * `pending_manager` - Pending entries manager
    /// * `group_id` - The group to join
    /// * `consumer_id` - Unique ID for this consumer
    /// * `options` - Optional join parameters
    ///
    /// # Errors
    ///
    /// - `GroupNotFound` if the group doesn't exist
    /// - `ConsumerAlreadyExists` if a consumer with this ID is already in the group
    /// - `TooManyConsumers` if the group is at capacity
    pub async fn join(
        store: Arc<K>,
        pending_manager: Arc<P>,
        group_id: ConsumerGroupId,
        consumer_id: ConsumerId,
        options: JoinOptions,
    ) -> Result<Self> {
        let now_ms = storage::now_unix_ms();

        // Load group state
        let mut group_state = storage::load_group_state(&*store, &group_id).await?;

        // Check group is accepting new members
        if group_state.state == GroupStateType::Dead {
            return Err(ConsumerGroupError::InvalidGroupState {
                group_id: group_id.as_str().to_string(),
                state: group_state.state.to_string(),
                operation: "join".to_string(),
            });
        }

        // Check if consumer already exists
        if storage::try_load_consumer_state(&*store, &group_id, &consumer_id).await?.is_some() {
            return Err(ConsumerGroupError::ConsumerAlreadyExists {
                group_id: group_id.as_str().to_string(),
                consumer_id: consumer_id.as_str().to_string(),
            });
        }

        // Check consumer count limit
        let current_count = storage::count_consumers(&*store, &group_id).await?;
        if current_count >= super::constants::MAX_CONSUMERS_PER_GROUP {
            return Err(ConsumerGroupError::TooManyConsumers {
                group_id: group_id.as_str().to_string(),
                count: current_count,
            });
        }

        // Generate fencing token and session ID
        let fencing_token = generate_fencing_token();
        let session_id = now_ms;

        // Create consumer state
        let consumer_state = ConsumerState {
            consumer_id: consumer_id.clone(),
            group_id: group_id.clone(),
            assigned_partitions: Vec::new(),
            session_id,
            fencing_token,
            pending_count: 0,
            metadata: options.metadata,
            tags: options.tags,
            joined_at_ms: now_ms,
            last_heartbeat_ms: now_ms,
        };

        // Save consumer state
        storage::save_consumer_state(&*store, &group_id, &consumer_state).await?;

        // Update group state
        group_state.member_count += 1;
        group_state.updated_at_ms = now_ms;

        // If first member, set as leader and transition to Stable
        if group_state.member_count == 1 {
            group_state.leader_id = Some(consumer_id.clone());
            group_state.state = GroupStateType::Stable;
        }

        storage::save_group_state(&*store, &group_state).await?;

        Ok(Self {
            store,
            pending_manager,
            group_id,
            consumer_id,
            fencing_token,
            session_id,
        })
    }

    /// Get the consumer's membership information.
    pub async fn member_info(&self) -> Result<MemberInfo> {
        let state = storage::load_consumer_state(&*self.store, &self.group_id, &self.consumer_id).await?;
        let group_state = storage::load_group_state(&*self.store, &self.group_id).await?;

        Ok(MemberInfo {
            consumer_id: state.consumer_id,
            group_id: state.group_id,
            fencing_token: state.fencing_token,
            session_id: state.session_id,
            generation_id: group_state.generation_id,
            metadata: state.metadata,
            tags: state.tags,
            joined_at_ms: state.joined_at_ms,
            last_heartbeat_ms: state.last_heartbeat_ms,
            heartbeat_deadline_ms: state.last_heartbeat_ms + super::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS,
            pending_count: state.pending_count,
            assigned_partitions: state.assigned_partitions,
        })
    }

    /// Send a heartbeat to maintain group membership.
    ///
    /// Consumers must call this periodically to avoid being removed
    /// from the group.
    pub async fn heartbeat(&self) -> Result<HeartbeatResponse> {
        let now_ms = storage::now_unix_ms();

        // Load consumer state
        let mut state = storage::load_consumer_state(&*self.store, &self.group_id, &self.consumer_id).await?;

        // Validate fencing token
        if state.fencing_token != self.fencing_token {
            return Err(ConsumerGroupError::FencingTokenMismatch {
                consumer_id: self.consumer_id.as_str().to_string(),
                expected: state.fencing_token,
                actual: self.fencing_token,
            });
        }

        // Update heartbeat timestamp
        state.last_heartbeat_ms = now_ms;
        storage::save_consumer_state(&*self.store, &self.group_id, &state).await?;

        // Load group state for response
        let group_state = storage::load_group_state(&*self.store, &self.group_id).await?;

        Ok(HeartbeatResponse {
            next_deadline_ms: now_ms + super::constants::CONSUMER_HEARTBEAT_INTERVAL_MS,
            rebalancing: matches!(
                group_state.state,
                GroupStateType::PreparingRebalance | GroupStateType::CompletingRebalance
            ),
            generation_id: group_state.generation_id,
            assigned_partitions: state.assigned_partitions,
        })
    }

    /// Acknowledge a message.
    pub async fn ack(&self, receipt_handle: &str) -> Result<AckResult> {
        self.pending_manager.ack(&self.group_id, receipt_handle).await
    }

    /// Negative acknowledge a message.
    pub async fn nack(&self, receipt_handle: &str) -> Result<NackResult> {
        let group_state = storage::load_group_state(&*self.store, &self.group_id).await?;
        self.pending_manager.nack(&self.group_id, receipt_handle, group_state.max_delivery_attempts).await
    }

    /// Batch acknowledge multiple messages.
    pub async fn batch_ack(&self, request: BatchAckRequest) -> Result<BatchAckResult> {
        self.pending_manager.batch_ack(&self.group_id, request).await
    }

    /// Mark a message as delivered to this consumer.
    ///
    /// This is typically called by the message dispatcher when assigning
    /// a message to this consumer.
    pub async fn mark_delivered(&self, cursor: u64, partition_id: PartitionId) -> Result<String> {
        let group_state = storage::load_group_state(&*self.store, &self.group_id).await?;

        let params = DeliveryParams {
            group_id: &self.group_id,
            consumer_id: &self.consumer_id,
            cursor,
            partition_id,
            visibility_timeout_ms: group_state.visibility_timeout_ms,
            fencing_token: self.fencing_token,
            now_ms: storage::now_unix_ms(),
        };

        self.pending_manager.mark_delivered(params).await
    }

    /// Leave the consumer group.
    ///
    /// Removes this consumer from the group and releases any pending messages.
    pub async fn leave(&self) -> Result<()> {
        let now_ms = storage::now_unix_ms();

        // Delete consumer state
        storage::delete_consumer_state(&*self.store, &self.group_id, &self.consumer_id).await?;

        // Update group state
        let mut group_state = storage::load_group_state(&*self.store, &self.group_id).await?;
        group_state.member_count = group_state.member_count.saturating_sub(1);
        group_state.updated_at_ms = now_ms;

        // If this was the leader, clear leader
        if group_state.leader_id.as_ref() == Some(&self.consumer_id) {
            group_state.leader_id = None;
        }

        // If no members left, transition to Empty
        if group_state.member_count == 0 {
            group_state.state = GroupStateType::Empty;
        }

        storage::save_group_state(&*self.store, &group_state).await?;

        Ok(())
    }

    /// Get the consumer's fencing token.
    pub fn fencing_token(&self) -> u64 {
        self.fencing_token
    }

    /// Get the consumer's session ID.
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Get the consumer ID.
    pub fn consumer_id(&self) -> &ConsumerId {
        &self.consumer_id
    }

    /// Get the group ID.
    pub fn group_id(&self) -> &ConsumerGroupId {
        &self.group_id
    }
}

#[cfg(test)]
mod tests {
    // Consumer tests require integration testing infrastructure
}
