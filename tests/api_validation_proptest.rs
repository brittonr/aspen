//! Property-based tests for API validation and types.
//!
//! Tests validate_write_command(), WriteCommand, ScanRequest, and other
//! API types for size validation, serialization, and boundary conditions.
//!
//! Target: Increase api/mod.rs coverage from 0% to 50%+

mod support;

use proptest::prelude::*;

use aspen::api::{
    ClusterNode, ClusterState, DEFAULT_SCAN_LIMIT, DeleteRequest, DeleteResult, MAX_SCAN_RESULTS,
    ReadRequest, ReadResult, ScanEntry, ScanRequest, ScanResult, WriteCommand, WriteRequest,
    WriteResult, validate_write_command,
};
use aspen::raft::constants::{MAX_KEY_SIZE, MAX_SETMULTI_KEYS, MAX_VALUE_SIZE};

// Test data generators

/// Generate a valid key (within size limit)
fn valid_key() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_:/-]{1,100}".prop_map(|s| s)
}

/// Generate a valid value (within size limit)
fn valid_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_:/-]{1,1000}".prop_map(|s| s)
}

/// Generate a key that exceeds MAX_KEY_SIZE
fn oversized_key() -> impl Strategy<Value = String> {
    // MAX_KEY_SIZE is 1024 bytes, generate 1025+ byte key
    Just("x".repeat(MAX_KEY_SIZE as usize + 1))
}

/// Generate a value that exceeds MAX_VALUE_SIZE
fn oversized_value() -> impl Strategy<Value = String> {
    // MAX_VALUE_SIZE is 1MB, generate 1MB + 1 byte value
    Just("y".repeat(MAX_VALUE_SIZE as usize + 1))
}

/// Generate valid key-value pairs for SetMulti
fn valid_pairs(max_count: usize) -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec((valid_key(), valid_value()), 1..=max_count)
}

/// Generate valid keys for DeleteMulti
fn valid_keys(max_count: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(valid_key(), 1..=max_count)
}

// Property tests for validate_write_command

proptest! {
    /// Valid Set commands should pass validation.
    #[test]
    fn test_valid_set_command_passes(
        key in valid_key(),
        value in valid_value()
    ) {
        let cmd = WriteCommand::Set { key, value };
        prop_assert!(validate_write_command(&cmd).is_ok());
    }

    /// Valid Delete commands should pass validation.
    #[test]
    fn test_valid_delete_command_passes(
        key in valid_key()
    ) {
        let cmd = WriteCommand::Delete { key };
        prop_assert!(validate_write_command(&cmd).is_ok());
    }

    /// Valid SetMulti commands within batch limit should pass.
    #[test]
    fn test_valid_setmulti_command_passes(
        pairs in valid_pairs(10)
    ) {
        let cmd = WriteCommand::SetMulti { pairs };
        prop_assert!(validate_write_command(&cmd).is_ok());
    }

    /// Valid DeleteMulti commands within batch limit should pass.
    #[test]
    fn test_valid_deletemulti_command_passes(
        keys in valid_keys(10)
    ) {
        let cmd = WriteCommand::DeleteMulti { keys };
        prop_assert!(validate_write_command(&cmd).is_ok());
    }

    /// Oversized key in Set command should fail validation.
    #[test]
    fn test_oversized_key_set_fails(
        key in oversized_key(),
        value in valid_value()
    ) {
        let cmd = WriteCommand::Set { key, value };
        let result = validate_write_command(&cmd);
        prop_assert!(result.is_err());
        match result {
            Err(aspen::api::KeyValueStoreError::KeyTooLarge { size, max }) => {
                prop_assert!(size > MAX_KEY_SIZE as usize);
                prop_assert_eq!(max, MAX_KEY_SIZE);
            }
            _ => prop_assert!(false, "Expected KeyTooLarge error"),
        }
    }

    /// Oversized value in Set command should fail validation.
    #[test]
    fn test_oversized_value_set_fails(
        key in valid_key(),
        value in oversized_value()
    ) {
        let cmd = WriteCommand::Set { key, value };
        let result = validate_write_command(&cmd);
        prop_assert!(result.is_err());
        match result {
            Err(aspen::api::KeyValueStoreError::ValueTooLarge { size, max }) => {
                prop_assert!(size > MAX_VALUE_SIZE as usize);
                prop_assert_eq!(max, MAX_VALUE_SIZE);
            }
            _ => prop_assert!(false, "Expected ValueTooLarge error"),
        }
    }

    /// Oversized key in Delete command should fail validation.
    #[test]
    fn test_oversized_key_delete_fails(
        key in oversized_key()
    ) {
        let cmd = WriteCommand::Delete { key };
        let result = validate_write_command(&cmd);
        prop_assert!(result.is_err());
        match result {
            Err(aspen::api::KeyValueStoreError::KeyTooLarge { .. }) => {}
            _ => prop_assert!(false, "Expected KeyTooLarge error"),
        }
    }

    /// WriteCommand Set serializes/deserializes correctly (JSON).
    #[test]
    fn test_write_command_set_json_roundtrip(
        key in valid_key(),
        value in valid_value()
    ) {
        let cmd = WriteCommand::Set { key, value };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(cmd, deserialized);
    }

    /// WriteCommand Set serializes/deserializes correctly (bincode).
    #[test]
    fn test_write_command_set_bincode_roundtrip(
        key in valid_key(),
        value in valid_value()
    ) {
        let cmd = WriteCommand::Set { key, value };
        let serialized = bincode::serialize(&cmd).expect("serialize");
        let deserialized: WriteCommand = bincode::deserialize(&serialized).expect("deserialize");
        prop_assert_eq!(cmd, deserialized);
    }

    /// WriteCommand SetMulti serializes/deserializes correctly.
    #[test]
    fn test_write_command_setmulti_json_roundtrip(
        pairs in valid_pairs(5)
    ) {
        let cmd = WriteCommand::SetMulti { pairs };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(cmd, deserialized);
    }

    /// WriteCommand Delete serializes/deserializes correctly.
    #[test]
    fn test_write_command_delete_json_roundtrip(
        key in valid_key()
    ) {
        let cmd = WriteCommand::Delete { key };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(cmd, deserialized);
    }

    /// WriteCommand DeleteMulti serializes/deserializes correctly.
    #[test]
    fn test_write_command_deletemulti_json_roundtrip(
        keys in valid_keys(5)
    ) {
        let cmd = WriteCommand::DeleteMulti { keys };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(cmd, deserialized);
    }

    /// WriteRequest wrapper serializes/deserializes correctly.
    #[test]
    fn test_write_request_json_roundtrip(
        key in valid_key(),
        value in valid_value()
    ) {
        let request = WriteRequest {
            command: WriteCommand::Set { key, value },
        };
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: WriteRequest = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(request, deserialized);
    }

    /// WriteResult wrapper serializes/deserializes correctly.
    #[test]
    fn test_write_result_json_roundtrip(
        key in valid_key(),
        value in valid_value()
    ) {
        let result = WriteResult {
            command: WriteCommand::Set { key, value },
        };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: WriteResult = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(result, deserialized);
    }

    /// ReadRequest serializes/deserializes correctly.
    #[test]
    fn test_read_request_json_roundtrip(
        key in valid_key()
    ) {
        let request = ReadRequest { key };
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: ReadRequest = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(request, deserialized);
    }

    /// ReadResult serializes/deserializes correctly.
    #[test]
    fn test_read_result_json_roundtrip(
        key in valid_key(),
        value in valid_value()
    ) {
        let result = ReadResult { key, value };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: ReadResult = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(result, deserialized);
    }

    /// DeleteRequest serializes/deserializes correctly.
    #[test]
    fn test_delete_request_json_roundtrip(
        key in valid_key()
    ) {
        let request = DeleteRequest { key };
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: DeleteRequest = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(request, deserialized);
    }

    /// DeleteResult serializes/deserializes correctly.
    #[test]
    fn test_delete_result_json_roundtrip(
        key in valid_key(),
        deleted in any::<bool>()
    ) {
        let result = DeleteResult { key, deleted };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: DeleteResult = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(result, deserialized);
    }

    /// ScanRequest serializes/deserializes correctly.
    #[test]
    fn test_scan_request_json_roundtrip(
        prefix in "[a-z]{0,20}",
        limit in prop::option::of(1u32..1000),
        continuation_token in prop::option::of("[a-zA-Z0-9]{10,30}")
    ) {
        let request = ScanRequest {
            prefix,
            limit,
            continuation_token,
        };
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: ScanRequest = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(request, deserialized);
    }

    /// ScanEntry serializes/deserializes correctly.
    #[test]
    fn test_scan_entry_json_roundtrip(
        key in valid_key(),
        value in valid_value()
    ) {
        let entry = ScanEntry { key, value };
        let serialized = serde_json::to_string(&entry).expect("serialize");
        let deserialized: ScanEntry = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(entry, deserialized);
    }

    /// ScanResult serializes/deserializes correctly.
    #[test]
    fn test_scan_result_json_roundtrip(
        count in 0u32..100,
        is_truncated in any::<bool>(),
        continuation_token in prop::option::of("[a-zA-Z0-9]{10,30}")
    ) {
        let entries: Vec<ScanEntry> = (0..count.min(10))
            .map(|i| ScanEntry {
                key: format!("key_{}", i),
                value: format!("value_{}", i),
            })
            .collect();
        let result = ScanResult {
            entries,
            count,
            is_truncated,
            continuation_token,
        };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: ScanResult = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(result, deserialized);
    }

    /// ClusterNode with simple address serializes correctly.
    #[test]
    fn test_cluster_node_json_roundtrip(
        id in 0u64..1000,
        addr in "[a-z0-9]{10,30}",
        raft_addr in prop::option::of("[a-z0-9.:]{10,30}")
    ) {
        let node = ClusterNode::new(id, addr, raft_addr);
        let serialized = serde_json::to_string(&node).expect("serialize");
        let deserialized: ClusterNode = serde_json::from_str(&serialized).expect("deserialize");
        // Compare fields (iroh_addr will be None for both)
        prop_assert_eq!(node.id, deserialized.id);
        prop_assert_eq!(node.addr, deserialized.addr);
        prop_assert_eq!(node.raft_addr, deserialized.raft_addr);
    }

    /// ClusterState serializes correctly.
    #[test]
    fn test_cluster_state_json_roundtrip(
        num_nodes in 0usize..5,
        num_members in 0usize..5,
        num_learners in 0usize..3
    ) {
        let nodes: Vec<ClusterNode> = (0..num_nodes)
            .map(|i| ClusterNode::new(i as u64, format!("node_{}", i), None))
            .collect();
        let members: Vec<u64> = (0..num_members).map(|i| i as u64).collect();
        let learners: Vec<ClusterNode> = (0..num_learners)
            .map(|i| ClusterNode::new(100 + i as u64, format!("learner_{}", i), None))
            .collect();

        let state = ClusterState {
            nodes,
            members,
            learners,
        };
        let serialized = serde_json::to_string(&state).expect("serialize");
        let deserialized: ClusterState = serde_json::from_str(&serialized).expect("deserialize");
        prop_assert_eq!(state, deserialized);
    }
}

// Unit tests for boundary conditions

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn test_key_exactly_at_limit() {
        let key = "k".repeat(MAX_KEY_SIZE as usize);
        let cmd = WriteCommand::Set {
            key,
            value: "v".to_string(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_key_one_over_limit() {
        let key = "k".repeat(MAX_KEY_SIZE as usize + 1);
        let cmd = WriteCommand::Set {
            key,
            value: "v".to_string(),
        };
        assert!(validate_write_command(&cmd).is_err());
    }

    #[test]
    fn test_value_exactly_at_limit() {
        let value = "v".repeat(MAX_VALUE_SIZE as usize);
        let cmd = WriteCommand::Set {
            key: "k".to_string(),
            value,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_value_one_over_limit() {
        let value = "v".repeat(MAX_VALUE_SIZE as usize + 1);
        let cmd = WriteCommand::Set {
            key: "k".to_string(),
            value,
        };
        assert!(validate_write_command(&cmd).is_err());
    }

    #[test]
    fn test_setmulti_exactly_at_batch_limit() {
        let pairs: Vec<(String, String)> = (0..MAX_SETMULTI_KEYS)
            .map(|i| (format!("key_{}", i), format!("value_{}", i)))
            .collect();
        let cmd = WriteCommand::SetMulti { pairs };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_setmulti_one_over_batch_limit() {
        let pairs: Vec<(String, String)> = (0..=MAX_SETMULTI_KEYS)
            .map(|i| (format!("key_{}", i), format!("value_{}", i)))
            .collect();
        let cmd = WriteCommand::SetMulti { pairs };
        match validate_write_command(&cmd) {
            Err(aspen::api::KeyValueStoreError::BatchTooLarge { size, max }) => {
                assert_eq!(size, MAX_SETMULTI_KEYS as usize + 1);
                assert_eq!(max, MAX_SETMULTI_KEYS);
            }
            _ => panic!("Expected BatchTooLarge error"),
        }
    }

    #[test]
    fn test_deletemulti_exactly_at_batch_limit() {
        let keys: Vec<String> = (0..MAX_SETMULTI_KEYS)
            .map(|i| format!("key_{}", i))
            .collect();
        let cmd = WriteCommand::DeleteMulti { keys };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_deletemulti_one_over_batch_limit() {
        let keys: Vec<String> = (0..=MAX_SETMULTI_KEYS)
            .map(|i| format!("key_{}", i))
            .collect();
        let cmd = WriteCommand::DeleteMulti { keys };
        match validate_write_command(&cmd) {
            Err(aspen::api::KeyValueStoreError::BatchTooLarge { size, max }) => {
                assert_eq!(size, MAX_SETMULTI_KEYS as usize + 1);
                assert_eq!(max, MAX_SETMULTI_KEYS);
            }
            _ => panic!("Expected BatchTooLarge error"),
        }
    }

    #[test]
    fn test_setmulti_with_oversized_key_in_batch() {
        let oversized_key = "k".repeat(MAX_KEY_SIZE as usize + 1);
        let pairs = vec![
            ("valid_key".to_string(), "valid_value".to_string()),
            (oversized_key, "another_value".to_string()),
        ];
        let cmd = WriteCommand::SetMulti { pairs };
        match validate_write_command(&cmd) {
            Err(aspen::api::KeyValueStoreError::KeyTooLarge { .. }) => {}
            _ => panic!("Expected KeyTooLarge error"),
        }
    }

    #[test]
    fn test_setmulti_with_oversized_value_in_batch() {
        let oversized_value = "v".repeat(MAX_VALUE_SIZE as usize + 1);
        let pairs = vec![
            ("valid_key".to_string(), "valid_value".to_string()),
            ("another_key".to_string(), oversized_value),
        ];
        let cmd = WriteCommand::SetMulti { pairs };
        match validate_write_command(&cmd) {
            Err(aspen::api::KeyValueStoreError::ValueTooLarge { .. }) => {}
            _ => panic!("Expected ValueTooLarge error"),
        }
    }

    #[test]
    fn test_deletemulti_with_oversized_key_in_batch() {
        let oversized_key = "k".repeat(MAX_KEY_SIZE as usize + 1);
        let keys = vec!["valid_key".to_string(), oversized_key];
        let cmd = WriteCommand::DeleteMulti { keys };
        match validate_write_command(&cmd) {
            Err(aspen::api::KeyValueStoreError::KeyTooLarge { .. }) => {}
            _ => panic!("Expected KeyTooLarge error"),
        }
    }

    #[test]
    fn test_empty_key_is_valid() {
        // Empty key is technically valid (size 0 <= 1024)
        let cmd = WriteCommand::Set {
            key: "".to_string(),
            value: "value".to_string(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_empty_value_is_valid() {
        let cmd = WriteCommand::Set {
            key: "key".to_string(),
            value: "".to_string(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_empty_setmulti_is_valid() {
        // Empty batch should be valid (0 <= 100)
        let cmd = WriteCommand::SetMulti { pairs: vec![] };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_empty_deletemulti_is_valid() {
        let cmd = WriteCommand::DeleteMulti { keys: vec![] };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_scan_constants() {
        // Verify scan constants are sensible
        assert_eq!(DEFAULT_SCAN_LIMIT, 1000);
        assert_eq!(MAX_SCAN_RESULTS, 10_000);
        // Runtime check to ensure constants remain valid
        let default: u32 = DEFAULT_SCAN_LIMIT;
        let max: u32 = MAX_SCAN_RESULTS;
        assert!(default <= max);
    }

    #[test]
    fn test_cluster_node_new_sets_fields() {
        let node = ClusterNode::new(42, "test_addr", Some("raft:5000".to_string()));
        assert_eq!(node.id, 42);
        assert_eq!(node.addr, "test_addr");
        assert_eq!(node.raft_addr, Some("raft:5000".to_string()));
        assert!(node.iroh_addr.is_none());
    }

    #[test]
    fn test_cluster_state_default() {
        let state = ClusterState::default();
        assert!(state.nodes.is_empty());
        assert!(state.members.is_empty());
        assert!(state.learners.is_empty());
    }

    #[test]
    fn test_utf8_multibyte_key_size_check() {
        // Emoji is 4 bytes in UTF-8
        // 256 emojis = 1024 bytes = exactly at limit
        let key = "🔥".repeat(256);
        assert_eq!(key.len(), 1024);
        let cmd = WriteCommand::Set {
            key,
            value: "v".to_string(),
        };
        assert!(validate_write_command(&cmd).is_ok());

        // 257 emojis = 1028 bytes = over limit
        let oversized_key = "🔥".repeat(257);
        assert_eq!(oversized_key.len(), 1028);
        let cmd = WriteCommand::Set {
            key: oversized_key,
            value: "v".to_string(),
        };
        assert!(validate_write_command(&cmd).is_err());
    }
}
