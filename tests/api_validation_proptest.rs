//! Property-based tests for API validation and types.
//!
//! Tests validate_write_command(), WriteCommand, ScanRequest, and other
//! API types for size validation, serialization, and boundary conditions.
//!
//! Target: Increase api/mod.rs coverage from 0% to 50%+

mod support;

use aspen::api::ClusterNode;
use aspen::api::ClusterState;
use aspen::api::DEFAULT_SCAN_LIMIT;
use aspen::api::DeleteRequest;
use aspen::api::DeleteResult;
use aspen::api::KeyValueWithRevision;
use aspen::api::MAX_SCAN_RESULTS;
use aspen::api::ReadRequest;
use aspen::api::ReadResult;
use aspen::api::ScanRequest;
use aspen::api::ScanResult;
use aspen::api::WriteCommand;
use aspen::api::WriteRequest;
use aspen::api::WriteResult;
use aspen::api::validate_write_command;
use aspen::raft::constants::MAX_KEY_SIZE;
use aspen::raft::constants::MAX_SETMULTI_KEYS;
use aspen::raft::constants::MAX_VALUE_SIZE;
use bolero::check;
use support::bolero_generators::OptionalContinuationToken;
use support::bolero_generators::OptionalRaftAddr;
use support::bolero_generators::OptionalScanLimit;
use support::bolero_generators::OversizedKey;
use support::bolero_generators::OversizedValue;
use support::bolero_generators::ScanPrefix;
use support::bolero_generators::SimpleAddr;
use support::bolero_generators::ValidApiKey;
use support::bolero_generators::ValidApiKeys;
use support::bolero_generators::ValidApiPairs;
use support::bolero_generators::ValidApiValue;

// Property tests for validate_write_command

/// Valid Set commands should pass validation.
#[test]
fn test_valid_set_command_passes() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let cmd = WriteCommand::Set {
            key: key.0.clone(),
            value: value.0.clone(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    });
}

/// Valid Delete commands should pass validation.
#[test]
fn test_valid_delete_command_passes() {
    check!().with_iterations(1000).with_type::<ValidApiKey>().for_each(|key| {
        let cmd = WriteCommand::Delete { key: key.0.clone() };
        assert!(validate_write_command(&cmd).is_ok());
    });
}

/// Valid SetMulti commands within batch limit should pass.
#[test]
fn test_valid_setmulti_command_passes() {
    check!().with_iterations(500).with_type::<ValidApiPairs>().for_each(|pairs| {
        let cmd = WriteCommand::SetMulti { pairs: pairs.0.clone() };
        assert!(validate_write_command(&cmd).is_ok());
    });
}

/// Valid DeleteMulti commands within batch limit should pass.
#[test]
fn test_valid_deletemulti_command_passes() {
    check!().with_iterations(500).with_type::<ValidApiKeys>().for_each(|keys| {
        let cmd = WriteCommand::DeleteMulti { keys: keys.0.clone() };
        assert!(validate_write_command(&cmd).is_ok());
    });
}

/// Oversized key in Set command should fail validation.
#[test]
fn test_oversized_key_set_fails() {
    check!().with_iterations(100).with_type::<(OversizedKey, ValidApiValue)>().for_each(|(key, value)| {
        let cmd = WriteCommand::Set {
            key: key.0.clone(),
            value: value.0.clone(),
        };
        let result = validate_write_command(&cmd);
        assert!(result.is_err());
        match result {
            Err(aspen::api::KeyValueStoreError::KeyTooLarge { size, max }) => {
                assert!(size > MAX_KEY_SIZE as usize);
                assert_eq!(max, MAX_KEY_SIZE);
            }
            _ => panic!("Expected KeyTooLarge error"),
        }
    });
}

/// Oversized value in Set command should fail validation.
#[test]
fn test_oversized_value_set_fails() {
    check!()
        .with_iterations(10) // Fewer iterations due to large allocations
        .with_type::<(ValidApiKey, OversizedValue)>()
        .for_each(|(key, value)| {
            let cmd = WriteCommand::Set {
                key: key.0.clone(),
                value: value.0.clone(),
            };
            let result = validate_write_command(&cmd);
            assert!(result.is_err());
            match result {
                Err(aspen::api::KeyValueStoreError::ValueTooLarge { size, max }) => {
                    assert!(size > MAX_VALUE_SIZE as usize);
                    assert_eq!(max, MAX_VALUE_SIZE);
                }
                _ => panic!("Expected ValueTooLarge error"),
            }
        });
}

/// Oversized key in Delete command should fail validation.
#[test]
fn test_oversized_key_delete_fails() {
    check!().with_iterations(100).with_type::<OversizedKey>().for_each(|key| {
        let cmd = WriteCommand::Delete { key: key.0.clone() };
        let result = validate_write_command(&cmd);
        assert!(result.is_err());
        match result {
            Err(aspen::api::KeyValueStoreError::KeyTooLarge { .. }) => {}
            _ => panic!("Expected KeyTooLarge error"),
        }
    });
}

/// WriteCommand Set serializes/deserializes correctly (JSON).
#[test]
fn test_write_command_set_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let cmd = WriteCommand::Set {
            key: key.0.clone(),
            value: value.0.clone(),
        };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(cmd, deserialized);
    });
}

/// WriteCommand Set serializes/deserializes correctly (bincode).
#[test]
fn test_write_command_set_bincode_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let cmd = WriteCommand::Set {
            key: key.0.clone(),
            value: value.0.clone(),
        };
        let serialized = bincode::serialize(&cmd).expect("serialize");
        let deserialized: WriteCommand = bincode::deserialize(&serialized).expect("deserialize");
        assert_eq!(cmd, deserialized);
    });
}

/// WriteCommand SetMulti serializes/deserializes correctly.
#[test]
fn test_write_command_setmulti_json_roundtrip() {
    check!().with_iterations(500).with_type::<ValidApiPairs>().for_each(|pairs| {
        let cmd = WriteCommand::SetMulti { pairs: pairs.0.clone() };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(cmd, deserialized);
    });
}

/// WriteCommand Delete serializes/deserializes correctly.
#[test]
fn test_write_command_delete_json_roundtrip() {
    check!().with_iterations(1000).with_type::<ValidApiKey>().for_each(|key| {
        let cmd = WriteCommand::Delete { key: key.0.clone() };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(cmd, deserialized);
    });
}

/// WriteCommand DeleteMulti serializes/deserializes correctly.
#[test]
fn test_write_command_deletemulti_json_roundtrip() {
    check!().with_iterations(500).with_type::<ValidApiKeys>().for_each(|keys| {
        let cmd = WriteCommand::DeleteMulti { keys: keys.0.clone() };
        let serialized = serde_json::to_string(&cmd).expect("serialize");
        let deserialized: WriteCommand = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(cmd, deserialized);
    });
}

/// WriteRequest wrapper serializes/deserializes correctly.
#[test]
fn test_write_request_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let request = WriteRequest {
            command: WriteCommand::Set {
                key: key.0.clone(),
                value: value.0.clone(),
            },
        };
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: WriteRequest = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(request, deserialized);
    });
}

/// WriteResult wrapper serializes/deserializes correctly.
#[test]
fn test_write_result_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let result = WriteResult {
            command: Some(WriteCommand::Set {
                key: key.0.clone(),
                value: value.0.clone(),
            }),
            ..Default::default()
        };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: WriteResult = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(result, deserialized);
    });
}

/// ReadRequest serializes/deserializes correctly.
#[test]
fn test_read_request_json_roundtrip() {
    check!().with_iterations(1000).with_type::<ValidApiKey>().for_each(|key| {
        let request = ReadRequest::new(key.0.clone());
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: ReadRequest = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(request, deserialized);
    });
}

/// ReadResult serializes/deserializes correctly.
#[test]
fn test_read_result_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let result = ReadResult {
            kv: Some(KeyValueWithRevision {
                key: key.0.clone(),
                value: value.0.clone(),
                version: 1,
                create_revision: 0,
                mod_revision: 0,
            }),
        };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: ReadResult = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(result, deserialized);
    });
}

/// DeleteRequest serializes/deserializes correctly.
#[test]
fn test_delete_request_json_roundtrip() {
    check!().with_iterations(1000).with_type::<ValidApiKey>().for_each(|key| {
        let request = DeleteRequest { key: key.0.clone() };
        let serialized = serde_json::to_string(&request).expect("serialize");
        let deserialized: DeleteRequest = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(request, deserialized);
    });
}

/// DeleteResult serializes/deserializes correctly.
#[test]
fn test_delete_result_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, bool)>().for_each(|(key, deleted)| {
        let result = DeleteResult {
            key: key.0.clone(),
            deleted: *deleted,
        };
        let serialized = serde_json::to_string(&result).expect("serialize");
        let deserialized: DeleteResult = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(result, deserialized);
    });
}

/// ScanRequest serializes/deserializes correctly.
#[test]
fn test_scan_request_json_roundtrip() {
    check!()
        .with_iterations(1000)
        .with_type::<(ScanPrefix, OptionalScanLimit, OptionalContinuationToken)>()
        .for_each(|(prefix, limit, continuation_token)| {
            let request = ScanRequest {
                prefix: prefix.0.clone(),
                limit: limit.0,
                continuation_token: continuation_token.0.clone(),
            };
            let serialized = serde_json::to_string(&request).expect("serialize");
            let deserialized: ScanRequest = serde_json::from_str(&serialized).expect("deserialize");
            assert_eq!(request, deserialized);
        });
}

/// KeyValueWithRevision serializes/deserializes correctly.
#[test]
fn test_key_value_with_revision_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(ValidApiKey, ValidApiValue)>().for_each(|(key, value)| {
        let entry = KeyValueWithRevision {
            key: key.0.clone(),
            value: value.0.clone(),
            version: 1,
            create_revision: 0,
            mod_revision: 0,
        };
        let serialized = serde_json::to_string(&entry).expect("serialize");
        let deserialized: KeyValueWithRevision = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(entry, deserialized);
    });
}

/// ScanResult serializes/deserializes correctly.
#[test]
fn test_scan_result_json_roundtrip() {
    check!().with_iterations(500).with_type::<(u32, bool, OptionalContinuationToken)>().for_each(
        |(count, is_truncated, continuation_token)| {
            let count = count % 100;
            let entries: Vec<KeyValueWithRevision> = (0..count.min(10))
                .map(|i| KeyValueWithRevision {
                    key: format!("key_{}", i),
                    value: format!("value_{}", i),
                    version: 1,
                    create_revision: i as u64,
                    mod_revision: i as u64,
                })
                .collect();
            let result = ScanResult {
                entries,
                count,
                is_truncated: *is_truncated,
                continuation_token: continuation_token.0.clone(),
            };
            let serialized = serde_json::to_string(&result).expect("serialize");
            let deserialized: ScanResult = serde_json::from_str(&serialized).expect("deserialize");
            assert_eq!(result, deserialized);
        },
    );
}

/// ClusterNode with simple address serializes correctly.
#[test]
fn test_cluster_node_json_roundtrip() {
    check!().with_iterations(1000).with_type::<(u64, SimpleAddr, OptionalRaftAddr)>().for_each(
        |(id, addr, raft_addr)| {
            let id = id % 1000;
            let node = ClusterNode::new(id, addr.0.clone(), raft_addr.0.clone());
            let serialized = serde_json::to_string(&node).expect("serialize");
            let deserialized: ClusterNode = serde_json::from_str(&serialized).expect("deserialize");
            // Compare fields (node_addr will be None for both)
            assert_eq!(node.id, deserialized.id);
            assert_eq!(node.addr, deserialized.addr);
            assert_eq!(node.raft_addr, deserialized.raft_addr);
        },
    );
}

/// ClusterState serializes correctly.
#[test]
fn test_cluster_state_json_roundtrip() {
    check!()
        .with_iterations(500)
        .with_type::<(u8, u8, u8)>()
        .for_each(|(num_nodes, num_members, num_learners)| {
            let num_nodes = (*num_nodes % 5) as usize;
            let num_members = (*num_members % 5) as usize;
            let num_learners = (*num_learners % 3) as usize;

            let nodes: Vec<ClusterNode> =
                (0..num_nodes).map(|i| ClusterNode::new(i as u64, format!("node_{}", i), None)).collect();
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
            assert_eq!(state, deserialized);
        });
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
        let pairs: Vec<(String, String)> =
            (0..MAX_SETMULTI_KEYS).map(|i| (format!("key_{}", i), format!("value_{}", i))).collect();
        let cmd = WriteCommand::SetMulti { pairs };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_setmulti_one_over_batch_limit() {
        let pairs: Vec<(String, String)> =
            (0..=MAX_SETMULTI_KEYS).map(|i| (format!("key_{}", i), format!("value_{}", i))).collect();
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
        let keys: Vec<String> = (0..MAX_SETMULTI_KEYS).map(|i| format!("key_{}", i)).collect();
        let cmd = WriteCommand::DeleteMulti { keys };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn test_deletemulti_one_over_batch_limit() {
        let keys: Vec<String> = (0..=MAX_SETMULTI_KEYS).map(|i| format!("key_{}", i)).collect();
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
    fn test_empty_key_is_rejected() {
        // Tiger Style: Empty keys are rejected to prevent issues with prefix scans
        // and to ensure all keys have meaningful identifiers.
        let cmd = WriteCommand::Set {
            key: "".to_string(),
            value: "value".to_string(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(aspen::api::KeyValueStoreError::EmptyKey)));
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
        assert!(node.node_addr.is_none());
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
