//! Key encoding for consumer groups.
//!
//! Uses FoundationDB-style Tuple and Subspace primitives from aspen-layer
//! to create order-preserving keys that enable efficient prefix scans.
//!
//! # Key Layout
//!
//! All consumer group keys are stored under `__pubsub/groups/`:
//!
//! | Key Pattern | Purpose |
//! |-------------|---------|
//! | `__pubsub/groups/{group_id}/state` | Group metadata |
//! | `__pubsub/groups/{group_id}/consumers/{consumer_id}` | Consumer state |
//! | `__pubsub/groups/{group_id}/partitions/{partition_id}` | Partition assignment |
//! | `__pubsub/groups/{group_id}/offsets/{partition_id}` | Committed offset |
//! | `__pubsub/groups/{group_id}/pending/{cursor}` | Pending entry |
//! | `__pubsub/groups/{group_id}/pending_by_deadline/{deadline_ms}/{cursor}` | Timeout index |
//! | `__pubsub/groups/{group_id}/dlq/{cursor}` | Dead letter entry |

use aspen_layer::Subspace;
use aspen_layer::Tuple;

use crate::consumer_group::constants::CONSUMER_GROUP_PREFIX;
use crate::consumer_group::types::ConsumerGroupId;
use crate::consumer_group::types::ConsumerId;
use crate::consumer_group::types::PartitionId;

/// Key namespace markers.
mod namespace {
    pub const STATE: &str = "state";
    pub const CONSUMERS: &str = "consumers";
    pub const PARTITIONS: &str = "partitions";
    pub const OFFSETS: &str = "offsets";
    pub const PENDING: &str = "pending";
    pub const PENDING_BY_DEADLINE: &str = "pending_by_deadline";
    pub const DLQ: &str = "dlq";
}

/// Key encoding utilities for consumer groups.
///
/// All methods return byte vectors suitable for use with the KV store.
/// Keys are encoded using FoundationDB-style tuples for lexicographic ordering.
pub struct ConsumerGroupKeys;

impl ConsumerGroupKeys {
    // =========================================================================
    // Subspace Constructors
    // =========================================================================

    /// Get the root subspace for all consumer groups.
    pub fn root_subspace() -> Subspace {
        Subspace::new(Tuple::new().push(CONSUMER_GROUP_PREFIX))
    }

    /// Get the subspace for a specific consumer group.
    pub fn group_subspace(group_id: &ConsumerGroupId) -> Subspace {
        Self::root_subspace().subspace(&Tuple::new().push(group_id.as_str()))
    }

    // =========================================================================
    // Group State Keys
    // =========================================================================

    /// Key for group state metadata.
    ///
    /// Format: `__pubsub/groups/{group_id}/state`
    pub fn group_state_key(group_id: &ConsumerGroupId) -> Vec<u8> {
        Self::group_subspace(group_id).pack(&Tuple::new().push(namespace::STATE))
    }

    /// Check if a key is a group state key.
    pub fn is_group_state_key(key: &[u8]) -> bool {
        // Check if the key ends with the state suffix
        let root = Self::root_subspace();
        if !root.contains(key) {
            return false;
        }

        // Try to unpack and check structure
        if let Ok(tuple) = root.unpack(key) {
            // Expected: [group_id, "state"]
            if tuple.len() == 2
                && let Some(aspen_layer::Element::String(s)) = tuple.get(1)
            {
                return s == namespace::STATE;
            }
        }
        false
    }

    // =========================================================================
    // Consumer Keys
    // =========================================================================

    /// Key for a consumer's state.
    ///
    /// Format: `__pubsub/groups/{group_id}/consumers/{consumer_id}`
    pub fn consumer_key(group_id: &ConsumerGroupId, consumer_id: &ConsumerId) -> Vec<u8> {
        Self::group_subspace(group_id).pack(&Tuple::new().push(namespace::CONSUMERS).push(consumer_id.as_str()))
    }

    /// Prefix for all consumers in a group.
    ///
    /// Use for scanning all consumers: `scan(prefix, prefix_end)`
    pub fn consumers_prefix(group_id: &ConsumerGroupId) -> Vec<u8> {
        Self::group_subspace(group_id).pack(&Tuple::new().push(namespace::CONSUMERS))
    }

    /// Range for all consumers in a group.
    ///
    /// Returns (start, end) for a range scan.
    pub fn consumers_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::CONSUMERS));
        subspace.range()
    }

    // =========================================================================
    // Partition Keys
    // =========================================================================

    /// Key for a partition's assignment.
    ///
    /// Format: `__pubsub/groups/{group_id}/partitions/{partition_id}`
    pub fn partition_key(group_id: &ConsumerGroupId, partition_id: PartitionId) -> Vec<u8> {
        Self::group_subspace(group_id)
            .pack(&Tuple::new().push(namespace::PARTITIONS).push(i64::from(partition_id.value())))
    }

    /// Range for all partitions in a group.
    pub fn partitions_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::PARTITIONS));
        subspace.range()
    }

    // =========================================================================
    // Offset Keys
    // =========================================================================

    /// Key for a partition's committed offset.
    ///
    /// Format: `__pubsub/groups/{group_id}/offsets/{partition_id}`
    pub fn offset_key(group_id: &ConsumerGroupId, partition_id: PartitionId) -> Vec<u8> {
        Self::group_subspace(group_id)
            .pack(&Tuple::new().push(namespace::OFFSETS).push(i64::from(partition_id.value())))
    }

    /// Range for all offsets in a group.
    pub fn offsets_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::OFFSETS));
        subspace.range()
    }

    // =========================================================================
    // Pending Entry Keys
    // =========================================================================

    /// Key for a pending entry (by cursor).
    ///
    /// Format: `__pubsub/groups/{group_id}/pending/{cursor}`
    pub fn pending_key(group_id: &ConsumerGroupId, cursor: u64) -> Vec<u8> {
        Self::group_subspace(group_id).pack(&Tuple::new().push(namespace::PENDING).push(cursor as i64))
    }

    /// Range for all pending entries in a group.
    pub fn pending_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::PENDING));
        subspace.range()
    }

    /// Key for pending entry index by deadline.
    ///
    /// Format: `__pubsub/groups/{group_id}/pending_by_deadline/{deadline_ms}/{cursor}`
    ///
    /// This index enables efficient scanning for expired entries.
    pub fn pending_by_deadline_key(group_id: &ConsumerGroupId, deadline_ms: u64, cursor: u64) -> Vec<u8> {
        Self::group_subspace(group_id)
            .pack(&Tuple::new().push(namespace::PENDING_BY_DEADLINE).push(deadline_ms as i64).push(cursor as i64))
    }

    /// Range for all pending entries with deadline before a given time.
    ///
    /// This is the key range for finding expired entries.
    pub fn expired_pending_range(group_id: &ConsumerGroupId, before_deadline_ms: u64) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::PENDING_BY_DEADLINE));

        let start = subspace.raw_prefix().to_vec();
        let end = subspace.pack(&Tuple::new().push(before_deadline_ms as i64));

        (start, end)
    }

    /// Range for all pending_by_deadline entries.
    pub fn pending_by_deadline_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::PENDING_BY_DEADLINE));
        subspace.range()
    }

    // =========================================================================
    // Dead Letter Queue Keys
    // =========================================================================

    /// Key for a dead letter entry.
    ///
    /// Format: `__pubsub/groups/{group_id}/dlq/{cursor}`
    pub fn dlq_key(group_id: &ConsumerGroupId, cursor: u64) -> Vec<u8> {
        Self::group_subspace(group_id).pack(&Tuple::new().push(namespace::DLQ).push(cursor as i64))
    }

    /// Range for all DLQ entries in a group.
    pub fn dlq_range(group_id: &ConsumerGroupId) -> (Vec<u8>, Vec<u8>) {
        let subspace = Self::group_subspace(group_id).subspace(&Tuple::new().push(namespace::DLQ));
        subspace.range()
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Check if a key belongs to the consumer groups namespace.
    pub fn is_consumer_group_key(key: &[u8]) -> bool {
        Self::root_subspace().contains(key)
    }

    /// Convert a key to a string for the KV store.
    pub fn key_to_string(key: &[u8]) -> String {
        String::from_utf8_lossy(key).to_string()
    }

    /// Convert a string key back to bytes.
    pub fn string_to_key(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    /// Extract group ID from a key.
    ///
    /// Returns None if the key is not a consumer group key.
    pub fn extract_group_id(key: &[u8]) -> Option<ConsumerGroupId> {
        let root = Self::root_subspace();
        if !root.contains(key) {
            return None;
        }

        root.unpack(key).ok().and_then(|tuple| {
            tuple.get(0).and_then(|elem| match elem {
                aspen_layer::Element::String(s) => Some(ConsumerGroupId::new_unchecked(s.clone())),
                aspen_layer::Element::Bytes(b) => {
                    std::str::from_utf8(b).ok().map(|s| ConsumerGroupId::new_unchecked(s.to_string()))
                }
                _ => None,
            })
        })
    }

    /// List all groups by scanning for state keys.
    ///
    /// Returns the key range to scan for group state keys.
    pub fn all_groups_range() -> (Vec<u8>, Vec<u8>) {
        Self::root_subspace().range()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_id(s: &str) -> ConsumerGroupId {
        ConsumerGroupId::new_unchecked(s)
    }

    fn consumer_id(s: &str) -> ConsumerId {
        ConsumerId::new_unchecked(s)
    }

    #[test]
    fn test_group_state_key() {
        let gid = group_id("my-group");
        let key = ConsumerGroupKeys::group_state_key(&gid);

        assert!(ConsumerGroupKeys::is_consumer_group_key(&key));
        assert!(ConsumerGroupKeys::is_group_state_key(&key));

        // Extract group ID
        let extracted = ConsumerGroupKeys::extract_group_id(&key);
        assert_eq!(extracted.as_ref().map(|g| g.as_str()), Some("my-group"));
    }

    #[test]
    fn test_consumer_key() {
        let gid = group_id("my-group");
        let cid = consumer_id("consumer-1");
        let key = ConsumerGroupKeys::consumer_key(&gid, &cid);

        assert!(ConsumerGroupKeys::is_consumer_group_key(&key));
        assert!(!ConsumerGroupKeys::is_group_state_key(&key));
    }

    #[test]
    fn test_consumers_range() {
        let gid = group_id("my-group");
        let (start, end) = ConsumerGroupKeys::consumers_range(&gid);

        // Consumer keys should be within range
        let key1 = ConsumerGroupKeys::consumer_key(&gid, &consumer_id("a"));
        let key2 = ConsumerGroupKeys::consumer_key(&gid, &consumer_id("z"));

        assert!(key1 >= start);
        assert!(key1 < end);
        assert!(key2 >= start);
        assert!(key2 < end);

        // Other keys should not be in range
        let state_key = ConsumerGroupKeys::group_state_key(&gid);
        assert!(state_key < start || state_key >= end);
    }

    #[test]
    fn test_consumer_key_ordering() {
        let gid = group_id("my-group");

        let key_a = ConsumerGroupKeys::consumer_key(&gid, &consumer_id("aaa"));
        let key_b = ConsumerGroupKeys::consumer_key(&gid, &consumer_id("bbb"));
        let key_z = ConsumerGroupKeys::consumer_key(&gid, &consumer_id("zzz"));

        assert!(key_a < key_b);
        assert!(key_b < key_z);
    }

    #[test]
    fn test_partition_key() {
        let gid = group_id("my-group");
        let key0 = ConsumerGroupKeys::partition_key(&gid, PartitionId::new(0));
        let key1 = ConsumerGroupKeys::partition_key(&gid, PartitionId::new(1));
        let key10 = ConsumerGroupKeys::partition_key(&gid, PartitionId::new(10));

        // Keys should be ordered numerically
        assert!(key0 < key1);
        assert!(key1 < key10);
    }

    #[test]
    fn test_offset_key() {
        let gid = group_id("my-group");
        let key = ConsumerGroupKeys::offset_key(&gid, PartitionId::new(5));

        assert!(ConsumerGroupKeys::is_consumer_group_key(&key));

        // Check range
        let (start, end) = ConsumerGroupKeys::offsets_range(&gid);
        assert!(key >= start);
        assert!(key < end);
    }

    #[test]
    fn test_pending_key_ordering() {
        let gid = group_id("my-group");

        let key1 = ConsumerGroupKeys::pending_key(&gid, 100);
        let key2 = ConsumerGroupKeys::pending_key(&gid, 200);
        let key3 = ConsumerGroupKeys::pending_key(&gid, 1000);

        assert!(key1 < key2);
        assert!(key2 < key3);
    }

    #[test]
    fn test_pending_by_deadline_key_ordering() {
        let gid = group_id("my-group");

        // Earlier deadlines should sort before later deadlines
        let key1 = ConsumerGroupKeys::pending_by_deadline_key(&gid, 1000, 100);
        let key2 = ConsumerGroupKeys::pending_by_deadline_key(&gid, 2000, 50);
        let key3 = ConsumerGroupKeys::pending_by_deadline_key(&gid, 2000, 100);

        assert!(key1 < key2);
        assert!(key2 < key3); // Same deadline, cursor ordering
    }

    #[test]
    fn test_expired_pending_range() {
        let gid = group_id("my-group");
        let now_ms = 5000u64;

        let (start, end) = ConsumerGroupKeys::expired_pending_range(&gid, now_ms);

        // Entries with deadline before now should be in range
        let expired_key = ConsumerGroupKeys::pending_by_deadline_key(&gid, 3000, 100);
        assert!(expired_key >= start);
        assert!(expired_key < end);

        // Entries with deadline at or after now should NOT be in range
        let not_expired_key = ConsumerGroupKeys::pending_by_deadline_key(&gid, 6000, 100);
        assert!(not_expired_key >= end);
    }

    #[test]
    fn test_dlq_key() {
        let gid = group_id("my-group");
        let key = ConsumerGroupKeys::dlq_key(&gid, 12345);

        assert!(ConsumerGroupKeys::is_consumer_group_key(&key));

        let (start, end) = ConsumerGroupKeys::dlq_range(&gid);
        assert!(key >= start);
        assert!(key < end);
    }

    #[test]
    fn test_key_string_roundtrip() {
        let gid = group_id("test-group");
        let key = ConsumerGroupKeys::group_state_key(&gid);

        let key_str = ConsumerGroupKeys::key_to_string(&key);
        let key_back = ConsumerGroupKeys::string_to_key(&key_str);

        assert_eq!(key, key_back);
    }

    #[test]
    fn test_different_groups_isolated() {
        let gid1 = group_id("group-1");
        let gid2 = group_id("group-2");

        let key1 = ConsumerGroupKeys::consumer_key(&gid1, &consumer_id("c1"));
        let key2 = ConsumerGroupKeys::consumer_key(&gid2, &consumer_id("c1"));

        // Same consumer ID in different groups should have different keys
        assert_ne!(key1, key2);

        // Group-2 consumers should not be in group-1's range
        let (start1, end1) = ConsumerGroupKeys::consumers_range(&gid1);
        assert!(key2 < start1 || key2 >= end1);
    }

    #[test]
    fn test_non_consumer_group_key() {
        let random_key = b"some/other/key";
        assert!(!ConsumerGroupKeys::is_consumer_group_key(random_key));
        assert!(!ConsumerGroupKeys::is_group_state_key(random_key));
        assert!(ConsumerGroupKeys::extract_group_id(random_key).is_none());
    }
}
