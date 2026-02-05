//! Key encoding for consumer group data in the KV store.
//!
//! Consumer group state is stored under the `__pubsub/groups/` prefix with
//! the following key structure:
//!
//! - Group state: `__pubsub/groups/{group_id}/state`
//! - Consumer state: `__pubsub/groups/{group_id}/consumers/{consumer_id}`
//! - Committed offset: `__pubsub/groups/{group_id}/offsets/{partition_id}`
//! - Pending entry: `__pubsub/groups/{group_id}/pending/{cursor}`
//! - Pending by deadline: `__pubsub/groups/{group_id}/pending_by_deadline/{deadline}/{cursor}`
//! - Dead letter entry: `__pubsub/groups/{group_id}/dlq/{cursor}`

use super::super::constants::PUBSUB_GROUPS_PREFIX;
use super::types::ConsumerGroupId;
use super::types::ConsumerId;
use super::types::PartitionId;

/// Key builder for consumer group data.
pub struct ConsumerGroupKeys;

impl ConsumerGroupKeys {
    /// Build the key for group state.
    pub fn group_state_key(group_id: &ConsumerGroupId) -> Vec<u8> {
        format!("{}{}/state", PUBSUB_GROUPS_PREFIX, group_id.as_str()).into_bytes()
    }

    /// Build the key for a consumer's state.
    pub fn consumer_key(group_id: &ConsumerGroupId, consumer_id: &ConsumerId) -> Vec<u8> {
        format!("{}{}/consumers/{}", PUBSUB_GROUPS_PREFIX, group_id.as_str(), consumer_id.as_str()).into_bytes()
    }

    /// Build the key prefix for all consumers in a group.
    pub fn consumers_prefix(group_id: &ConsumerGroupId) -> Vec<u8> {
        format!("{}{}/consumers/", PUBSUB_GROUPS_PREFIX, group_id.as_str()).into_bytes()
    }

    /// Build the key range for scanning all consumers in a group.
    pub fn consumers_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let prefix = Self::consumers_prefix(group_id);
        let mut end = prefix.clone();
        // Increment last byte to get exclusive end
        if let Some(last) = end.last_mut() {
            *last = last.saturating_add(1);
        }
        (prefix, end)
    }

    /// Build the key for a committed offset.
    pub fn offset_key(group_id: &ConsumerGroupId, partition_id: PartitionId) -> Vec<u8> {
        format!("{}{}/offsets/{:08x}", PUBSUB_GROUPS_PREFIX, group_id.as_str(), partition_id.value()).into_bytes()
    }

    /// Build the key for a pending entry (by cursor).
    pub fn pending_key(group_id: &ConsumerGroupId, cursor: u64) -> Vec<u8> {
        format!("{}{}/pending/{:016x}", PUBSUB_GROUPS_PREFIX, group_id.as_str(), cursor).into_bytes()
    }

    /// Build the key prefix for all pending entries in a group.
    pub fn pending_prefix(group_id: &ConsumerGroupId) -> Vec<u8> {
        format!("{}{}/pending/", PUBSUB_GROUPS_PREFIX, group_id.as_str()).into_bytes()
    }

    /// Build the key for pending entry by deadline (for expiry scanning).
    pub fn pending_by_deadline_key(group_id: &ConsumerGroupId, deadline_ms: u64, cursor: u64) -> Vec<u8> {
        format!(
            "{}{}/pending_by_deadline/{:016x}/{:016x}",
            PUBSUB_GROUPS_PREFIX,
            group_id.as_str(),
            deadline_ms,
            cursor
        )
        .into_bytes()
    }

    /// Build the key prefix for expired pending entries.
    pub fn pending_by_deadline_prefix(group_id: &ConsumerGroupId) -> Vec<u8> {
        format!("{}{}/pending_by_deadline/", PUBSUB_GROUPS_PREFIX, group_id.as_str()).into_bytes()
    }

    /// Build the key range for scanning expired pending entries.
    ///
    /// Returns keys where deadline < before_deadline_ms.
    pub fn expired_pending_range(group_id: &ConsumerGroupId, before_deadline_ms: u64) -> (Vec<u8>, Vec<u8>) {
        let prefix = Self::pending_by_deadline_prefix(group_id);
        let start = prefix.clone();
        let end =
            format!("{}{}/pending_by_deadline/{:016x}", PUBSUB_GROUPS_PREFIX, group_id.as_str(), before_deadline_ms)
                .into_bytes();
        (start, end)
    }

    /// Build the key for a dead letter queue entry.
    pub fn dlq_key(group_id: &ConsumerGroupId, cursor: u64) -> Vec<u8> {
        format!("{}{}/dlq/{:016x}", PUBSUB_GROUPS_PREFIX, group_id.as_str(), cursor).into_bytes()
    }

    /// Build the key prefix for all DLQ entries in a group.
    pub fn dlq_prefix(group_id: &ConsumerGroupId) -> Vec<u8> {
        format!("{}{}/dlq/", PUBSUB_GROUPS_PREFIX, group_id.as_str()).into_bytes()
    }

    /// Build the key prefix for all data related to a group.
    pub fn group_prefix(group_id: &ConsumerGroupId) -> Vec<u8> {
        format!("{}{}/", PUBSUB_GROUPS_PREFIX, group_id.as_str()).into_bytes()
    }

    /// Build the key prefix for all consumer groups.
    pub fn all_groups_prefix() -> Vec<u8> {
        PUBSUB_GROUPS_PREFIX.as_bytes().to_vec()
    }

    /// Build the key range for scanning all groups.
    pub fn all_groups_range() -> (Vec<u8>, Vec<u8>) {
        let prefix = Self::all_groups_prefix();
        let mut end = prefix.clone();
        if let Some(last) = end.last_mut() {
            *last = last.saturating_add(1);
        }
        (prefix, end)
    }

    /// Convert a key to a string (for KV store operations).
    pub fn key_to_string(key: &[u8]) -> String {
        String::from_utf8_lossy(key).into_owned()
    }

    /// Check if a key is a group state key.
    pub fn is_group_state_key(key: &[u8]) -> bool {
        key.ends_with(b"/state")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_state_key() {
        let group_id = ConsumerGroupId::new_unchecked("my-group");
        let key = ConsumerGroupKeys::group_state_key(&group_id);
        let key_str = ConsumerGroupKeys::key_to_string(&key);

        assert!(key_str.starts_with("__pubsub/groups/"));
        assert!(key_str.contains("my-group"));
        assert!(key_str.ends_with("/state"));
    }

    #[test]
    fn test_consumer_key() {
        let group_id = ConsumerGroupId::new_unchecked("g1");
        let consumer_id = ConsumerId::new_unchecked("c1");
        let key = ConsumerGroupKeys::consumer_key(&group_id, &consumer_id);
        let key_str = ConsumerGroupKeys::key_to_string(&key);

        assert!(key_str.contains("g1"));
        assert!(key_str.contains("consumers"));
        assert!(key_str.contains("c1"));
    }

    #[test]
    fn test_offset_key() {
        let group_id = ConsumerGroupId::new_unchecked("g1");
        let partition_id = PartitionId::new(5);
        let key = ConsumerGroupKeys::offset_key(&group_id, partition_id);
        let key_str = ConsumerGroupKeys::key_to_string(&key);

        assert!(key_str.contains("offsets"));
        assert!(key_str.contains("00000005")); // partition ID in hex
    }

    #[test]
    fn test_pending_key_ordering() {
        let group_id = ConsumerGroupId::new_unchecked("g1");

        let key1 = ConsumerGroupKeys::pending_key(&group_id, 100);
        let key2 = ConsumerGroupKeys::pending_key(&group_id, 200);

        // Keys should be lexicographically ordered by cursor
        assert!(key1 < key2);
    }

    #[test]
    fn test_pending_by_deadline_ordering() {
        let group_id = ConsumerGroupId::new_unchecked("g1");

        let key1 = ConsumerGroupKeys::pending_by_deadline_key(&group_id, 1000, 1);
        let key2 = ConsumerGroupKeys::pending_by_deadline_key(&group_id, 2000, 1);
        let key3 = ConsumerGroupKeys::pending_by_deadline_key(&group_id, 2000, 2);

        // Keys should be ordered by deadline, then cursor
        assert!(key1 < key2);
        assert!(key2 < key3);
    }

    #[test]
    fn test_is_group_state_key() {
        let group_id = ConsumerGroupId::new_unchecked("g1");
        let state_key = ConsumerGroupKeys::group_state_key(&group_id);
        let consumer_key = ConsumerGroupKeys::consumer_key(&group_id, &ConsumerId::new_unchecked("c1"));

        assert!(ConsumerGroupKeys::is_group_state_key(&state_key));
        assert!(!ConsumerGroupKeys::is_group_state_key(&consumer_key));
    }
}
