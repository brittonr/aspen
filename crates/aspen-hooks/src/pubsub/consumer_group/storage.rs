//! Storage helper functions for consumer groups.
//!
//! Common operations for persisting consumer group state to the key-value store.
//! All operations go through Raft consensus via the KeyValueStore trait.

use aspen_core::KeyValueStore;
use aspen_core::kv::BatchOperation;
use aspen_core::kv::ReadRequest;
use aspen_core::kv::ScanRequest;
use aspen_core::kv::WriteCommand;
use aspen_core::kv::WriteRequest;

use super::super::cursor::Cursor;
use super::constants::MAX_CONSUMER_GROUPS;
use super::constants::MAX_CONSUMERS_PER_GROUP;
use super::error::ConsumerGroupError;
use super::error::Result;
use super::keys::ConsumerGroupKeys;
use super::types::CommittedOffset;
use super::types::ConsumerGroupId;
use super::types::ConsumerId;
use super::types::ConsumerState;
use super::types::DeadLetterEntry;
use super::types::GroupState;
use super::types::GroupStateType;
use super::types::PartitionId;
use super::types::PendingEntry;

/// Load group state from storage.
///
/// # Errors
///
/// - `GroupNotFound` if the group does not exist
/// - `KvStoreFailed` if storage access fails
/// - `SerializationFailed` if deserialization fails
pub async fn load_group_state<S: KeyValueStore + ?Sized>(store: &S, group_id: &ConsumerGroupId) -> Result<GroupState> {
    let key = ConsumerGroupKeys::group_state_key(group_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    let result = store.read(ReadRequest::new(key_str)).await?;

    match result.kv {
        Some(kv) => rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
            message: format!("failed to deserialize group state: {}", e),
        }),
        None => Err(ConsumerGroupError::GroupNotFound {
            group_id: group_id.as_str().to_string(),
        }),
    }
}

/// Try to load group state, returning None if not found.
pub async fn try_load_group_state<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
) -> Result<Option<GroupState>> {
    let key = ConsumerGroupKeys::group_state_key(group_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    let result = store.read(ReadRequest::new(key_str)).await?;

    match result.kv {
        Some(kv) => {
            let state =
                rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
                    message: format!("failed to deserialize group state: {}", e),
                })?;
            Ok(Some(state))
        }
        None => Ok(None),
    }
}

/// Save group state to storage.
///
/// # Errors
///
/// - `KvStoreFailed` if storage access fails
/// - `SerializationFailed` if serialization fails
pub async fn save_group_state<S: KeyValueStore + ?Sized>(store: &S, state: &GroupState) -> Result<()> {
    let key = ConsumerGroupKeys::group_state_key(&state.group_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);
    let value = rmp_serde::to_vec(state).map_err(|e| ConsumerGroupError::SerializationFailed {
        message: format!("failed to serialize group state: {}", e),
    })?;
    let value_str = String::from_utf8_lossy(&value).into_owned();

    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: key_str,
                value: value_str,
            },
        })
        .await?;

    Ok(())
}

/// Load consumer state from storage.
///
/// # Errors
///
/// - `ConsumerNotFound` if the consumer does not exist
/// - `KvStoreFailed` if storage access fails
/// - `SerializationFailed` if deserialization fails
pub async fn load_consumer_state<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    consumer_id: &ConsumerId,
) -> Result<ConsumerState> {
    let key = ConsumerGroupKeys::consumer_key(group_id, consumer_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    let result = store.read(ReadRequest::new(key_str)).await?;

    match result.kv {
        Some(kv) => rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
            message: format!("failed to deserialize consumer state: {}", e),
        }),
        None => Err(ConsumerGroupError::ConsumerNotFound {
            group_id: group_id.as_str().to_string(),
            consumer_id: consumer_id.as_str().to_string(),
        }),
    }
}

/// Try to load consumer state, returning None if not found.
pub async fn try_load_consumer_state<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    consumer_id: &ConsumerId,
) -> Result<Option<ConsumerState>> {
    let key = ConsumerGroupKeys::consumer_key(group_id, consumer_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    let result = store.read(ReadRequest::new(key_str)).await?;

    match result.kv {
        Some(kv) => {
            let state =
                rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
                    message: format!("failed to deserialize consumer state: {}", e),
                })?;
            Ok(Some(state))
        }
        None => Ok(None),
    }
}

/// Save consumer state to storage.
pub async fn save_consumer_state<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    state: &ConsumerState,
) -> Result<()> {
    let key = ConsumerGroupKeys::consumer_key(group_id, &state.consumer_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);
    let value = rmp_serde::to_vec(state).map_err(|e| ConsumerGroupError::SerializationFailed {
        message: format!("failed to serialize consumer state: {}", e),
    })?;
    let value_str = String::from_utf8_lossy(&value).into_owned();

    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: key_str,
                value: value_str,
            },
        })
        .await?;

    Ok(())
}

/// Delete consumer state from storage.
pub async fn delete_consumer_state<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    consumer_id: &ConsumerId,
) -> Result<()> {
    let key = ConsumerGroupKeys::consumer_key(group_id, consumer_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    store
        .write(WriteRequest {
            command: WriteCommand::Delete { key: key_str },
        })
        .await?;

    Ok(())
}

/// Count the number of consumer groups.
///
/// Used for enforcing MAX_CONSUMER_GROUPS limit.
pub async fn count_groups<S: KeyValueStore + ?Sized>(store: &S) -> Result<u32> {
    let (start, _end) = ConsumerGroupKeys::all_groups_range();
    let prefix = ConsumerGroupKeys::key_to_string(&start);

    let result = store
        .scan(ScanRequest {
            prefix,
            limit: Some((MAX_CONSUMER_GROUPS + 1) as u32),
            continuation_token: None,
        })
        .await?;

    // Count only state keys (filter out consumers, pending, etc.)
    let count =
        result.entries.iter().filter(|e| ConsumerGroupKeys::is_group_state_key(e.key.as_bytes())).count() as u32;

    Ok(count)
}

/// Count the number of consumers in a group.
pub async fn count_consumers<S: KeyValueStore + ?Sized>(store: &S, group_id: &ConsumerGroupId) -> Result<u32> {
    let (start, _end) = ConsumerGroupKeys::consumers_range(group_id);
    let prefix = ConsumerGroupKeys::key_to_string(&start);

    let result = store
        .scan(ScanRequest {
            prefix,
            limit: Some(MAX_CONSUMERS_PER_GROUP + 1),
            continuation_token: None,
        })
        .await?;

    Ok(result.count)
}

/// List all consumers in a group.
pub async fn list_consumers<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
) -> Result<Vec<ConsumerState>> {
    let (start, _end) = ConsumerGroupKeys::consumers_range(group_id);
    let prefix = ConsumerGroupKeys::key_to_string(&start);

    let result = store
        .scan(ScanRequest {
            prefix,
            limit: Some(MAX_CONSUMERS_PER_GROUP),
            continuation_token: None,
        })
        .await?;

    let mut consumers = Vec::with_capacity(result.entries.len());
    for kv in result.entries {
        let state: ConsumerState =
            rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
                message: format!("failed to deserialize consumer state: {}", e),
            })?;
        consumers.push(state);
    }

    Ok(consumers)
}

/// Atomically update group state with new type and generation.
pub async fn transition_group_state<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    new_state: GroupStateType,
    new_generation: Option<u64>,
    now_ms: u64,
) -> Result<GroupState> {
    let mut group = load_group_state(store, group_id).await?;
    group.state = new_state;
    if let Some(generation) = new_generation {
        group.generation_id = generation;
    }
    group.updated_at_ms = now_ms;
    save_group_state(store, &group).await?;
    Ok(group)
}

/// Save a pending entry to both indexes atomically.
pub async fn save_pending_entry<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    entry: &PendingEntry,
) -> Result<()> {
    let pending_key = ConsumerGroupKeys::pending_key(group_id, entry.cursor);
    let deadline_key = ConsumerGroupKeys::pending_by_deadline_key(group_id, entry.visibility_deadline_ms, entry.cursor);

    let value = rmp_serde::to_vec(entry).map_err(|e| ConsumerGroupError::SerializationFailed {
        message: format!("failed to serialize pending entry: {}", e),
    })?;
    let value_str = String::from_utf8_lossy(&value).into_owned();

    // Atomic batch write to both indexes
    store
        .write(WriteRequest {
            command: WriteCommand::Batch {
                operations: vec![
                    BatchOperation::Set {
                        key: ConsumerGroupKeys::key_to_string(&pending_key),
                        value: value_str.clone(),
                    },
                    BatchOperation::Set {
                        key: ConsumerGroupKeys::key_to_string(&deadline_key),
                        value: value_str,
                    },
                ],
            },
        })
        .await?;

    Ok(())
}

/// Delete a pending entry from both indexes atomically.
pub async fn delete_pending_entry<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    entry: &PendingEntry,
) -> Result<()> {
    let pending_key = ConsumerGroupKeys::pending_key(group_id, entry.cursor);
    let deadline_key = ConsumerGroupKeys::pending_by_deadline_key(group_id, entry.visibility_deadline_ms, entry.cursor);

    // Atomic batch delete from both indexes
    store
        .write(WriteRequest {
            command: WriteCommand::Batch {
                operations: vec![
                    BatchOperation::Delete {
                        key: ConsumerGroupKeys::key_to_string(&pending_key),
                    },
                    BatchOperation::Delete {
                        key: ConsumerGroupKeys::key_to_string(&deadline_key),
                    },
                ],
            },
        })
        .await?;

    Ok(())
}

/// Load a pending entry by cursor.
pub async fn load_pending_entry<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    cursor: u64,
) -> Result<Option<PendingEntry>> {
    let key = ConsumerGroupKeys::pending_key(group_id, cursor);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    let result = store.read(ReadRequest::new(key_str)).await?;

    match result.kv {
        Some(kv) => {
            let entry =
                rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
                    message: format!("failed to deserialize pending entry: {}", e),
                })?;
            Ok(Some(entry))
        }
        None => Ok(None),
    }
}

/// Move a pending entry to the dead letter queue atomically.
pub async fn move_to_dlq<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    entry: &PendingEntry,
    reason: &str,
    now_ms: u64,
) -> Result<()> {
    let pending_key = ConsumerGroupKeys::pending_key(group_id, entry.cursor);
    let deadline_key = ConsumerGroupKeys::pending_by_deadline_key(group_id, entry.visibility_deadline_ms, entry.cursor);
    let dlq_key = ConsumerGroupKeys::dlq_key(group_id, entry.cursor);

    let dlq_entry = DeadLetterEntry {
        cursor: entry.cursor,
        original_consumer_id: entry.consumer_id.clone(),
        delivery_attempts: entry.delivery_attempt,
        first_delivered_at_ms: entry.delivered_at_ms,
        dead_lettered_at_ms: now_ms,
        partition_id: entry.partition_id,
        reason: reason.to_string(),
    };

    let dlq_value = rmp_serde::to_vec(&dlq_entry).map_err(|e| ConsumerGroupError::SerializationFailed {
        message: format!("failed to serialize DLQ entry: {}", e),
    })?;
    let dlq_value_str = String::from_utf8_lossy(&dlq_value).into_owned();

    // Atomic: delete from pending indexes + add to DLQ
    store
        .write(WriteRequest {
            command: WriteCommand::Batch {
                operations: vec![
                    BatchOperation::Delete {
                        key: ConsumerGroupKeys::key_to_string(&pending_key),
                    },
                    BatchOperation::Delete {
                        key: ConsumerGroupKeys::key_to_string(&deadline_key),
                    },
                    BatchOperation::Set {
                        key: ConsumerGroupKeys::key_to_string(&dlq_key),
                        value: dlq_value_str,
                    },
                ],
            },
        })
        .await?;

    Ok(())
}

/// Scan for expired pending entries.
pub async fn scan_expired_pending<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    before_deadline_ms: u64,
    limit: u32,
) -> Result<Vec<PendingEntry>> {
    let (start, _end) = ConsumerGroupKeys::expired_pending_range(group_id, before_deadline_ms);
    let prefix = ConsumerGroupKeys::key_to_string(&start);

    let result = store
        .scan(ScanRequest {
            prefix,
            limit: Some(limit),
            continuation_token: None,
        })
        .await?;

    let mut entries = Vec::with_capacity(result.entries.len());
    for kv in result.entries {
        let entry: PendingEntry =
            rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
                message: format!("failed to deserialize pending entry: {}", e),
            })?;

        // Double-check deadline (scan may return entries at boundary)
        if entry.visibility_deadline_ms < before_deadline_ms {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Get current timestamp in milliseconds.
pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// =============================================================================
// Offset Storage
// =============================================================================

/// Load the committed offset for a group/partition.
///
/// Returns None if no offset has been committed yet.
pub async fn load_committed_offset<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    partition_id: PartitionId,
) -> Result<Option<CommittedOffset>> {
    let key = ConsumerGroupKeys::offset_key(group_id, partition_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);

    let result = store.read(ReadRequest::new(key_str)).await?;

    match result.kv {
        Some(kv) => {
            let offset =
                rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| ConsumerGroupError::SerializationFailed {
                    message: format!("failed to deserialize committed offset: {}", e),
                })?;
            Ok(Some(offset))
        }
        None => Ok(None),
    }
}

/// Save a committed offset for a group/partition.
pub async fn save_committed_offset<S: KeyValueStore + ?Sized>(
    store: &S,
    group_id: &ConsumerGroupId,
    partition_id: PartitionId,
    cursor: Cursor,
    now_ms: u64,
) -> Result<CommittedOffset> {
    let offset = CommittedOffset {
        group_id: group_id.clone(),
        partition_id,
        cursor,
        committed_at_ms: now_ms,
        metadata: None,
    };

    let key = ConsumerGroupKeys::offset_key(group_id, partition_id);
    let key_str = ConsumerGroupKeys::key_to_string(&key);
    let value = rmp_serde::to_vec(&offset).map_err(|e| ConsumerGroupError::SerializationFailed {
        message: format!("failed to serialize committed offset: {}", e),
    })?;
    let value_str = String::from_utf8_lossy(&value).into_owned();

    store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: key_str,
                value: value_str,
            },
        })
        .await?;

    Ok(offset)
}

// =============================================================================
// Batch Delete Operations
// =============================================================================

/// Maximum number of keys to delete in a single batch.
const BATCH_DELETE_SIZE: u32 = 100;

/// Delete all keys matching a prefix in batched operations.
///
/// Returns the total number of keys deleted.
pub async fn delete_keys_with_prefix<S: KeyValueStore + ?Sized>(store: &S, prefix: &str) -> Result<u32> {
    let mut deleted_count = 0u32;

    loop {
        // Scan for keys with the prefix
        let result = store
            .scan(ScanRequest {
                prefix: prefix.to_string(),
                limit: Some(BATCH_DELETE_SIZE),
                continuation_token: None,
            })
            .await?;

        if result.entries.is_empty() {
            break;
        }

        // Build batch delete operations
        let operations: Vec<BatchOperation> =
            result.entries.iter().map(|kv| BatchOperation::Delete { key: kv.key.clone() }).collect();

        let batch_size = operations.len() as u32;

        // Delete in a batch
        store
            .write(WriteRequest {
                command: WriteCommand::Batch { operations },
            })
            .await?;

        deleted_count = deleted_count.saturating_add(batch_size);

        // If we got fewer than limit, we're done
        if batch_size < BATCH_DELETE_SIZE {
            break;
        }
    }

    Ok(deleted_count)
}

/// Delete all keys in a range in batched operations.
///
/// Returns the total number of keys deleted.
pub async fn delete_keys_in_range<S: KeyValueStore + ?Sized>(store: &S, start: &[u8], _end: &[u8]) -> Result<u32> {
    let prefix = ConsumerGroupKeys::key_to_string(start);
    delete_keys_with_prefix(store, &prefix).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_unix_ms() {
        let now = now_unix_ms();
        // Should be after 2020-01-01
        assert!(now > 1577836800000);
    }
}
