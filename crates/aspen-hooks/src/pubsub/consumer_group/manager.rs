//! Consumer group manager for creating and managing groups.
//!
//! Provides administrative operations for consumer groups including
//! creation, deletion, and configuration management.

use std::sync::Arc;

use aspen_core::KeyValueStore;

use super::constants::MAX_CONSUMER_GROUPS;
use super::error::ConsumerGroupError;
use super::error::Result;
use super::keys::ConsumerGroupKeys;
use super::storage;
use super::types::ConsumerGroupConfig;
use super::types::ConsumerGroupId;
use super::types::GroupState;
use super::types::GroupStateType;

/// Manager for consumer group lifecycle operations.
pub struct ConsumerGroupManager<K: KeyValueStore + ?Sized> {
    store: Arc<K>,
}

impl<K: KeyValueStore + ?Sized + 'static> ConsumerGroupManager<K> {
    /// Create a new consumer group manager.
    pub fn new(store: Arc<K>) -> Self {
        Self { store }
    }

    /// Create a new consumer group.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the new group
    ///
    /// # Errors
    ///
    /// - `GroupAlreadyExists` if a group with this ID exists
    /// - `TooManyGroups` if the cluster limit is reached
    /// - `InvalidConfig` if configuration validation fails
    pub async fn create_group(&self, config: ConsumerGroupConfig) -> Result<GroupState> {
        config.validate()?;

        let now_ms = storage::now_unix_ms();

        // Check if group already exists
        if storage::try_load_group_state(&*self.store, &config.group_id).await?.is_some() {
            return Err(ConsumerGroupError::GroupAlreadyExists {
                group_id: config.group_id.as_str().to_string(),
            });
        }

        // Check group count limit
        let group_count = storage::count_groups(&*self.store).await?;
        if group_count as usize >= MAX_CONSUMER_GROUPS {
            return Err(ConsumerGroupError::TooManyGroups {
                count: group_count as usize,
            });
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

        storage::save_group_state(&*self.store, &state).await?;

        Ok(state)
    }

    /// Get a consumer group's state.
    pub async fn get_group(&self, group_id: &ConsumerGroupId) -> Result<GroupState> {
        storage::load_group_state(&*self.store, group_id).await
    }

    /// Delete a consumer group.
    ///
    /// This removes all group state including consumers, pending entries,
    /// offsets, and DLQ entries.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group to delete
    /// * `force` - If true, delete even if group has active consumers
    ///
    /// # Errors
    ///
    /// - `GroupNotFound` if the group doesn't exist
    /// - `InvalidGroupState` if group has consumers and force is false
    pub async fn delete_group(&self, group_id: &ConsumerGroupId, force: bool) -> Result<()> {
        // Load group state
        let group_state = storage::load_group_state(&*self.store, group_id).await?;

        // Check for active consumers
        if group_state.member_count > 0 && !force {
            return Err(ConsumerGroupError::InvalidGroupState {
                group_id: group_id.as_str().to_string(),
                state: format!("{} active consumers", group_state.member_count),
                operation: "delete".to_string(),
            });
        }

        // Delete all group data
        let prefix = ConsumerGroupKeys::group_prefix(group_id);
        let prefix_str = ConsumerGroupKeys::key_to_string(&prefix);
        storage::delete_keys_with_prefix(&*self.store, &prefix_str).await?;

        Ok(())
    }

    /// List all consumer groups.
    pub async fn list_groups(&self) -> Result<Vec<GroupState>> {
        let (start, _end) = ConsumerGroupKeys::all_groups_range();
        let prefix = ConsumerGroupKeys::key_to_string(&start);

        let result = self
            .store
            .scan(aspen_core::kv::ScanRequest {
                prefix,
                limit_results: Some(MAX_CONSUMER_GROUPS as u32),
                continuation_token: None,
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

        Ok(groups)
    }

    /// Update a group's configuration.
    ///
    /// Only some fields can be updated after creation.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The group to update
    /// * `visibility_timeout_ms` - New visibility timeout (if Some)
    /// * `max_delivery_attempts` - New max delivery attempts (if Some)
    pub async fn update_group(
        &self,
        group_id: &ConsumerGroupId,
        visibility_timeout_ms: Option<u64>,
        max_delivery_attempts: Option<u32>,
    ) -> Result<GroupState> {
        let now_ms = storage::now_unix_ms();

        let mut state = storage::load_group_state(&*self.store, group_id).await?;

        if let Some(timeout) = visibility_timeout_ms {
            if timeout < super::constants::MIN_VISIBILITY_TIMEOUT_MS {
                return Err(ConsumerGroupError::VisibilityTimeoutTooSmall { timeout_ms: timeout });
            }
            if timeout > super::constants::MAX_VISIBILITY_TIMEOUT_MS {
                return Err(ConsumerGroupError::VisibilityTimeoutTooLarge { timeout_ms: timeout });
            }
            state.visibility_timeout_ms = timeout;
        }

        if let Some(attempts) = max_delivery_attempts {
            if attempts < super::constants::MIN_DELIVERY_ATTEMPTS {
                return Err(ConsumerGroupError::MaxDeliveryAttemptsTooSmall { attempts });
            }
            if attempts > super::constants::MAX_DELIVERY_ATTEMPTS {
                return Err(ConsumerGroupError::MaxDeliveryAttemptsTooLarge { attempts });
            }
            state.max_delivery_attempts = attempts;
        }

        state.updated_at_ms = now_ms;
        storage::save_group_state(&*self.store, &state).await?;

        Ok(state)
    }

    /// Get group statistics.
    pub async fn get_group_stats(&self, group_id: &ConsumerGroupId) -> Result<GroupStats> {
        let state = storage::load_group_state(&*self.store, group_id).await?;
        let consumers = storage::list_consumers(&*self.store, group_id).await?;

        let total_pending: u32 = consumers.iter().map(|c| c.pending_count).sum();

        Ok(GroupStats {
            group_id: group_id.clone(),
            state: state.state,
            generation_id: state.generation_id,
            member_count: state.member_count,
            total_pending,
            leader_id: state.leader_id,
            created_at_ms: state.created_at_ms,
            updated_at_ms: state.updated_at_ms,
        })
    }
}

/// Statistics for a consumer group.
#[derive(Debug, Clone)]
pub struct GroupStats {
    /// Group identifier.
    pub group_id: ConsumerGroupId,
    /// Current state.
    pub state: GroupStateType,
    /// Current generation ID.
    pub generation_id: u64,
    /// Number of active members.
    pub member_count: u32,
    /// Total pending messages across all consumers.
    pub total_pending: u32,
    /// Current leader consumer ID.
    pub leader_id: Option<super::types::ConsumerId>,
    /// Group creation time.
    pub created_at_ms: u64,
    /// Last update time.
    pub updated_at_ms: u64,
}

#[cfg(test)]
mod tests {
    // Manager tests require integration testing infrastructure
}
