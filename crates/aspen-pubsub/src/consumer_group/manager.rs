//! Consumer group management.
//!
//! Provides the ConsumerGroupManager trait for creating, deleting, and managing
//! consumer groups and their members.

use std::sync::Arc;

use async_trait::async_trait;
use aspen_core::KeyValueStore;

use crate::consumer_group::constants::MAX_CONSUMER_GROUPS;
use crate::consumer_group::consumer::GroupConsumer;
use crate::consumer_group::consumer::RaftGroupConsumer;
use crate::consumer_group::error::ConsumerGroupError;
use crate::consumer_group::error::Result;
use crate::consumer_group::keys::ConsumerGroupKeys;
use crate::consumer_group::pending::RaftPendingEntriesList;
use crate::consumer_group::storage;
use crate::consumer_group::types::AckResult;
use crate::consumer_group::types::BatchAckRequest;
use crate::consumer_group::types::BatchAckResult;
use crate::consumer_group::types::ConsumerGroupConfig;
use crate::consumer_group::types::ConsumerGroupId;
use crate::consumer_group::types::ConsumerId;
use crate::consumer_group::types::ConsumerState;
use crate::consumer_group::types::GroupState;
use crate::consumer_group::types::GroupStateType;
use crate::consumer_group::types::HeartbeatResponse;
use crate::consumer_group::types::JoinOptions;
use crate::consumer_group::types::MemberInfo;
use crate::consumer_group::types::NackResult;

/// Manages consumer group lifecycle and membership.
///
/// Provides operations for creating, deleting, and managing consumer groups.
/// Also handles consumer join/leave operations.
#[async_trait]
pub trait ConsumerGroupManager: Send + Sync {
    // =========================================================================
    // Group Lifecycle
    // =========================================================================

    /// Create a new consumer group with the given configuration.
    ///
    /// # Errors
    ///
    /// - `GroupAlreadyExists` if group_id is taken
    /// - `TooManyGroups` if MAX_CONSUMER_GROUPS exceeded
    /// - `InvalidConfig` if configuration validation fails
    async fn create_group(&self, config: ConsumerGroupConfig) -> Result<GroupState>;

    /// Delete a consumer group and all associated state.
    ///
    /// Removes all consumers, pending entries, offsets, and DLQ entries.
    async fn delete_group(&self, group_id: &ConsumerGroupId) -> Result<()>;

    /// Get the current state of a consumer group.
    async fn get_group(&self, group_id: &ConsumerGroupId) -> Result<GroupState>;

    /// List all consumer groups with optional pagination.
    async fn list_groups(
        &self,
        limit: u32,
        continuation_token: Option<String>,
    ) -> Result<(Vec<GroupState>, Option<String>)>;

    // =========================================================================
    // Consumer Membership
    // =========================================================================

    /// Join a consumer group.
    async fn join(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        options: JoinOptions,
    ) -> Result<MemberInfo>;

    /// Leave a consumer group gracefully.
    async fn leave(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<()>;

    /// Send heartbeat to maintain group membership.
    async fn heartbeat(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<HeartbeatResponse>;

    /// Get all members of a consumer group.
    async fn get_members(&self, group_id: &ConsumerGroupId) -> Result<Vec<MemberInfo>>;

    /// Get a specific consumer's state.
    async fn get_consumer(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
    ) -> Result<ConsumerState>;

    // =========================================================================
    // Message Operations
    // =========================================================================

    /// Acknowledge a message.
    async fn ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<AckResult>;

    /// Negative acknowledge a message.
    async fn nack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<NackResult>;

    /// Batch acknowledge messages.
    async fn batch_ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        request: BatchAckRequest,
        fencing_token: u64,
    ) -> Result<BatchAckResult>;
}

/// Default implementation of ConsumerGroupManager.
pub struct DefaultConsumerGroupManager<K: KeyValueStore + ?Sized + 'static> {
    /// KV store for persistence.
    store: Arc<K>,
    /// Group consumer for membership operations.
    consumer: Arc<RaftGroupConsumer<K, RaftPendingEntriesList<K>>>,
    /// Pending entries manager.
    pending: Arc<RaftPendingEntriesList<K>>,
}

impl<K: KeyValueStore + ?Sized + 'static> DefaultConsumerGroupManager<K> {
    /// Create a new consumer group manager.
    ///
    /// # Arguments
    ///
    /// * `store` - KV store for persistence
    /// * `receipt_secret` - 32-byte secret for receipt handle signing
    pub fn new(store: Arc<K>, receipt_secret: [u8; 32]) -> Self {
        let pending = Arc::new(RaftPendingEntriesList::new(store.clone(), receipt_secret));
        let consumer = Arc::new(RaftGroupConsumer::new(store.clone(), pending.clone()));

        Self {
            store,
            consumer,
            pending,
        }
    }

    /// Get the pending entries manager for use by background tasks.
    pub fn pending_manager(&self) -> Arc<RaftPendingEntriesList<K>> {
        self.pending.clone()
    }
}

#[async_trait]
impl<K: KeyValueStore + ?Sized + 'static> ConsumerGroupManager for DefaultConsumerGroupManager<K> {
    async fn create_group(&self, config: ConsumerGroupConfig) -> Result<GroupState> {
        let now_ms = storage::now_unix_ms();

        // Validate configuration
        config.validate()?;

        // Check if group already exists
        if storage::try_load_group_state(&*self.store, &config.group_id)
            .await?
            .is_some()
        {
            return Err(ConsumerGroupError::GroupAlreadyExists {
                group_id: config.group_id.as_str().to_string(),
            });
        }

        // Check group count limit
        let group_count = storage::count_groups(&*self.store).await?;
        if group_count as usize >= MAX_CONSUMER_GROUPS {
            return Err(ConsumerGroupError::TooManyGroups { count: group_count as usize });
        }

        // Create group state
        let state = GroupState {
            group_id: config.group_id.clone(),
            pattern_str: config.pattern_str.clone(),
            state: GroupStateType::Empty,
            generation_id: 0,
            assignment_mode: config.assignment_mode,
            ack_policy: config.ack_policy,
            visibility_timeout_ms: config.visibility_timeout_ms,
            max_delivery_attempts: config.max_delivery_attempts,
            partition_count: config.partition_count,
            leader_id: None,
            member_count: 0,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        // Save group state
        storage::save_group_state(&*self.store, &state).await?;

        Ok(state)
    }

    async fn delete_group(&self, group_id: &ConsumerGroupId) -> Result<()> {
        // Verify group exists
        let _state = storage::load_group_state(&*self.store, group_id).await?;

        // Delete all consumers first
        let consumers = storage::list_consumers(&*self.store, group_id).await?;
        for consumer in consumers {
            storage::delete_consumer_state(&*self.store, group_id, &consumer.consumer_id).await?;
        }

        // Delete group state
        let key = ConsumerGroupKeys::group_state_key(group_id);
        let key_str = ConsumerGroupKeys::key_to_string(&key);

        self.store
            .write(aspen_core::kv::WriteRequest {
                command: aspen_core::kv::WriteCommand::Delete { key: key_str },
            })
            .await?;

        // TODO: Also delete pending entries, offsets, and DLQ entries
        // This requires scanning and deleting all keys with the group prefix

        Ok(())
    }

    async fn get_group(&self, group_id: &ConsumerGroupId) -> Result<GroupState> {
        storage::load_group_state(&*self.store, group_id).await
    }

    async fn list_groups(
        &self,
        limit: u32,
        continuation_token: Option<String>,
    ) -> Result<(Vec<GroupState>, Option<String>)> {
        let (start, _end) = ConsumerGroupKeys::all_groups_range();
        let prefix = ConsumerGroupKeys::key_to_string(&start);

        let result = self
            .store
            .scan(aspen_core::kv::ScanRequest {
                prefix,
                limit: Some(limit),
                continuation_token,
            })
            .await?;

        let mut groups = Vec::new();
        for kv in result.entries {
            // Only process state keys
            if ConsumerGroupKeys::is_group_state_key(kv.key.as_bytes()) {
                let state: GroupState = rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| {
                    ConsumerGroupError::SerializationFailed {
                        message: format!("failed to deserialize group state: {}", e),
                    }
                })?;
                groups.push(state);
            }
        }

        Ok((groups, result.continuation_token))
    }

    async fn join(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        options: JoinOptions,
    ) -> Result<MemberInfo> {
        self.consumer.join_group(group_id, consumer_id, options).await
    }

    async fn leave(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<()> {
        self.consumer
            .leave_group(group_id, consumer_id, fencing_token)
            .await
    }

    async fn heartbeat(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        fencing_token: u64,
    ) -> Result<HeartbeatResponse> {
        self.consumer
            .heartbeat(group_id, consumer_id, fencing_token)
            .await
    }

    async fn get_members(&self, group_id: &ConsumerGroupId) -> Result<Vec<MemberInfo>> {
        // Verify group exists
        let group_state = storage::load_group_state(&*self.store, group_id).await?;

        let consumers = storage::list_consumers(&*self.store, group_id).await?;

        let members: Vec<MemberInfo> = consumers
            .into_iter()
            .map(|c| MemberInfo {
                consumer_id: c.consumer_id.clone(),
                group_id: group_id.clone(),
                fencing_token: c.fencing_token,
                session_id: c.session_id,
                generation_id: group_state.generation_id,
                metadata: c.metadata.clone(),
                tags: c.tags.clone(),
                joined_at_ms: c.joined_at_ms,
                last_heartbeat_ms: c.last_heartbeat_ms,
                heartbeat_deadline_ms: c.last_heartbeat_ms
                    + crate::consumer_group::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS,
                pending_count: c.pending_count,
                assigned_partitions: c.assigned_partitions,
            })
            .collect();

        Ok(members)
    }

    async fn get_consumer(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
    ) -> Result<ConsumerState> {
        storage::load_consumer_state(&*self.store, group_id, consumer_id).await
    }

    async fn ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<AckResult> {
        self.consumer
            .ack(group_id, consumer_id, receipt_handle, fencing_token)
            .await
    }

    async fn nack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        receipt_handle: &str,
        fencing_token: u64,
    ) -> Result<NackResult> {
        self.consumer
            .nack(group_id, consumer_id, receipt_handle, fencing_token)
            .await
    }

    async fn batch_ack(
        &self,
        group_id: &ConsumerGroupId,
        consumer_id: &ConsumerId,
        request: BatchAckRequest,
        fencing_token: u64,
    ) -> Result<BatchAckResult> {
        self.consumer
            .batch_ack(group_id, consumer_id, request, fencing_token)
            .await
    }
}

#[cfg(test)]
mod tests {
    // Integration tests will be in a separate file since they require a KV store
}
