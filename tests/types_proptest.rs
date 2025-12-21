//! Property-based tests for raft/types.rs
//!
//! Tests NodeId parsing, Display/FromStr roundtrip, ordering,
//! and AppRequest/AppResponse serialization.
//!
//! Target: Increase types.rs coverage from 18% to 60%+

mod support;

use bolero::check;
use std::str::FromStr;

use aspen::raft::types::{AppRequest, AppResponse, NodeId, RaftMemberInfo};
use support::bolero_generators::{BalancedAppRequest, InvalidNodeIdString, ValidNodeIdString};

/// NodeId Display -> FromStr roundtrip should be identity.
#[test]
fn test_node_id_display_fromstr_roundtrip() {
    check!()
        .with_iterations(1000)
        .with_type::<u64>()
        .for_each(|id| {
            let node_id = NodeId::from(*id);
            let displayed = node_id.to_string();
            let parsed: NodeId = displayed.parse().expect("Display output should parse");
            assert_eq!(node_id, parsed);
        });
}

/// NodeId ordering should match underlying u64 ordering.
#[test]
fn test_node_id_ordering() {
    check!()
        .with_iterations(1000)
        .with_type::<(u64, u64)>()
        .for_each(|(a, b)| {
            let node_a = NodeId::from(*a);
            let node_b = NodeId::from(*b);

            assert_eq!(a < b, node_a < node_b);
            assert_eq!(a == b, node_a == node_b);
            assert_eq!(a > b, node_a > node_b);
        });
}

/// Valid NodeId strings should parse successfully.
#[test]
fn test_node_id_valid_string_parsing() {
    check!()
        .with_iterations(1000)
        .with_type::<ValidNodeIdString>()
        .for_each(|s| {
            let result = NodeId::from_str(&s.0);
            assert!(result.is_ok(), "Valid string '{}' should parse", s.0);

            // Verify roundtrip
            let node_id = result.unwrap();
            let displayed = node_id.to_string();
            assert_eq!(s.0, displayed, "Roundtrip should preserve value");
        });
}

/// Invalid NodeId strings should fail parsing.
#[test]
fn test_node_id_invalid_string_parsing() {
    check!()
        .with_iterations(1000)
        .with_type::<InvalidNodeIdString>()
        .for_each(|s| {
            let result = NodeId::from_str(&s.0);
            assert!(
                result.is_err(),
                "Invalid string '{}' should fail parsing",
                s.0
            );
        });
}

/// NodeId From<u64>/Into<u64> roundtrip should be identity.
#[test]
fn test_node_id_u64_conversion_roundtrip() {
    check!()
        .with_iterations(1000)
        .with_type::<u64>()
        .for_each(|id| {
            let node_id = NodeId::from(*id);
            let back: u64 = node_id.into();
            assert_eq!(*id, back);
        });
}

/// AppRequest serde JSON roundtrip.
#[test]
fn test_app_request_serde_json_roundtrip() {
    check!()
        .with_iterations(1000)
        .with_type::<BalancedAppRequest>()
        .for_each(|request| {
            let serialized = serde_json::to_string(&request.0).expect("Should serialize");
            let deserialized: AppRequest =
                serde_json::from_str(&serialized).expect("Should deserialize");

            // Compare by re-serializing (since AppRequest doesn't derive PartialEq)
            let reserialized = serde_json::to_string(&deserialized).expect("Should re-serialize");
            assert_eq!(serialized, reserialized);
        });
}

/// AppRequest bincode roundtrip (compact binary format).
#[test]
fn test_app_request_bincode_roundtrip() {
    check!()
        .with_iterations(1000)
        .with_type::<BalancedAppRequest>()
        .for_each(|request| {
            let serialized = bincode::serialize(&request.0).expect("Should serialize");
            let deserialized: AppRequest =
                bincode::deserialize(&serialized).expect("Should deserialize");

            // Compare by re-serializing
            let reserialized = bincode::serialize(&deserialized).expect("Should re-serialize");
            assert_eq!(serialized, reserialized);
        });
}

/// AppRequest Display should contain key for Set/Delete operations.
#[test]
fn test_app_request_display_contains_key() {
    check!()
        .with_iterations(1000)
        .with_type::<BalancedAppRequest>()
        .for_each(|request| {
            let displayed = request.0.to_string();

            match &request.0 {
                AppRequest::Set { key, value } => {
                    assert!(displayed.contains(key), "Display should contain key");
                    assert!(displayed.contains(value), "Display should contain value");
                    assert!(
                        displayed.contains("Set"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::SetMulti { pairs } => {
                    assert!(
                        displayed.contains("SetMulti"),
                        "Display should contain variant name"
                    );
                    // First pair's key should be present
                    if let Some((k, _)) = pairs.first() {
                        assert!(displayed.contains(k), "Display should contain first key");
                    }
                }
                AppRequest::Delete { key } => {
                    assert!(displayed.contains(key), "Display should contain key");
                    assert!(
                        displayed.contains("Delete"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::DeleteMulti { keys } => {
                    assert!(
                        displayed.contains("DeleteMulti"),
                        "Display should contain variant name"
                    );
                    if let Some(k) = keys.first() {
                        assert!(displayed.contains(k), "Display should contain first key");
                    }
                }
                AppRequest::CompareAndSwap { key, new_value, .. } => {
                    assert!(displayed.contains(key), "Display should contain key");
                    assert!(
                        displayed.contains(new_value),
                        "Display should contain new_value"
                    );
                    assert!(
                        displayed.contains("CompareAndSwap"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::CompareAndDelete { key, expected } => {
                    assert!(displayed.contains(key), "Display should contain key");
                    assert!(
                        displayed.contains(expected),
                        "Display should contain expected"
                    );
                    assert!(
                        displayed.contains("CompareAndDelete"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::Batch { operations } => {
                    assert!(
                        displayed.contains("Batch"),
                        "Display should contain variant name"
                    );
                    // Should contain operation count
                    assert!(displayed.contains(&operations.len().to_string()));
                }
                AppRequest::ConditionalBatch {
                    conditions,
                    operations,
                } => {
                    assert!(
                        displayed.contains("ConditionalBatch"),
                        "Display should contain variant name"
                    );
                    // Should contain operation and condition counts
                    assert!(displayed.contains(&operations.len().to_string()));
                    assert!(displayed.contains(&conditions.len().to_string()));
                }
                // TTL variants - not generated by BalancedAppRequest
                AppRequest::SetWithTTL { key, value, .. } => {
                    assert!(displayed.contains(key), "Display should contain key");
                    assert!(displayed.contains(value), "Display should contain value");
                    assert!(
                        displayed.contains("SetWithTTL"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::SetMultiWithTTL { .. } => {
                    assert!(
                        displayed.contains("SetMultiWithTTL"),
                        "Display should contain variant name"
                    );
                }
                // Lease variants - not generated by BalancedAppRequest
                AppRequest::SetWithLease { key, value, .. } => {
                    assert!(displayed.contains(key), "Display should contain key");
                    assert!(displayed.contains(value), "Display should contain value");
                    assert!(
                        displayed.contains("SetWithLease"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::SetMultiWithLease { .. } => {
                    assert!(
                        displayed.contains("SetMultiWithLease"),
                        "Display should contain variant name"
                    );
                }
                AppRequest::LeaseGrant {
                    lease_id,
                    ttl_seconds,
                } => {
                    assert!(
                        displayed.contains("LeaseGrant"),
                        "Display should contain variant name"
                    );
                    assert!(displayed.contains(&lease_id.to_string()));
                    assert!(displayed.contains(&ttl_seconds.to_string()));
                }
                AppRequest::LeaseRevoke { lease_id } => {
                    assert!(
                        displayed.contains("LeaseRevoke"),
                        "Display should contain variant name"
                    );
                    assert!(displayed.contains(&lease_id.to_string()));
                }
                AppRequest::LeaseKeepalive { lease_id } => {
                    assert!(
                        displayed.contains("LeaseKeepalive"),
                        "Display should contain variant name"
                    );
                    assert!(displayed.contains(&lease_id.to_string()));
                }
            }
        });
}

/// AppResponse serde JSON roundtrip.
#[test]
fn test_app_response_serde_json_roundtrip() {
    check!()
        .with_iterations(1000)
        .with_type::<(Option<String>, Option<bool>, Option<bool>)>()
        .for_each(|(value, deleted, cas_succeeded)| {
            let response = AppResponse {
                value: value.clone(),
                deleted: *deleted,
                cas_succeeded: *cas_succeeded,
                ..Default::default()
            };
            let serialized = serde_json::to_string(&response).expect("Should serialize");
            let deserialized: AppResponse =
                serde_json::from_str(&serialized).expect("Should deserialize");

            assert_eq!(response.value, deserialized.value);
            assert_eq!(response.deleted, deserialized.deleted);
            assert_eq!(response.cas_succeeded, deserialized.cas_succeeded);
        });
}

/// NodeId hash should be consistent with equality.
#[test]
fn test_node_id_hash_consistency() {
    check!()
        .with_iterations(1000)
        .with_type::<(u64, u64)>()
        .for_each(|(a, b)| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};

            // Limit range to make collisions more likely to test
            let a = a % 1000;
            let b = b % 1000;

            let node_a = NodeId::from(a);
            let node_b = NodeId::from(b);

            let hash_a = {
                let mut hasher = DefaultHasher::new();
                node_a.hash(&mut hasher);
                hasher.finish()
            };

            let hash_b = {
                let mut hasher = DefaultHasher::new();
                node_b.hash(&mut hasher);
                hasher.finish()
            };

            // Equal values must have equal hashes
            if node_a == node_b {
                assert_eq!(hash_a, hash_b, "Equal NodeIds must have equal hashes");
            }
        });
}

/// NodeId::new constructor should work for any u64.
#[test]
fn test_node_id_new_any_u64() {
    check!()
        .with_iterations(1000)
        .with_type::<u64>()
        .for_each(|id| {
            let node_id = NodeId::new(*id);
            assert_eq!(node_id.0, *id);
            let back: u64 = node_id.into();
            assert_eq!(*id, back);
        });
}

/// NodeId Default should be 0.
#[test]
fn test_node_id_default() {
    // This is a single test case, not property-based
    let default_node = NodeId::default();
    assert_eq!(default_node.0, 0);
}

// Non-proptest unit tests for specific edge cases
#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_node_id_boundary_values() {
        // Zero
        let zero = NodeId::from(0);
        assert_eq!(zero.to_string(), "0");
        assert_eq!("0".parse::<NodeId>().unwrap(), zero);

        // Max u64
        let max = NodeId::from(u64::MAX);
        assert_eq!(max.to_string(), u64::MAX.to_string());
        assert_eq!(u64::MAX.to_string().parse::<NodeId>().unwrap(), max);

        // One
        let one = NodeId::from(1);
        assert_eq!(one.to_string(), "1");
    }

    #[test]
    fn test_node_id_parse_error_types() {
        // Empty string
        assert!(NodeId::from_str("").is_err());

        // Non-numeric
        assert!(NodeId::from_str("abc").is_err());

        // Negative
        assert!(NodeId::from_str("-1").is_err());

        // Overflow
        assert!(NodeId::from_str("18446744073709551616").is_err()); // u64::MAX + 1

        // Decimal
        assert!(NodeId::from_str("1.5").is_err());

        // With spaces (leading/trailing not trimmed by FromStr)
        assert!(NodeId::from_str(" 123").is_err());
        assert!(NodeId::from_str("123 ").is_err());
    }

    #[test]
    fn test_app_request_display_formatting() {
        let set = AppRequest::Set {
            key: "foo".to_string(),
            value: "bar".to_string(),
        };
        let display = set.to_string();
        assert!(display.contains("foo"));
        assert!(display.contains("bar"));
        assert!(display.contains("Set"));

        let delete = AppRequest::Delete {
            key: "baz".to_string(),
        };
        let display = delete.to_string();
        assert!(display.contains("baz"));
        assert!(display.contains("Delete"));

        let set_multi = AppRequest::SetMulti {
            pairs: vec![
                ("k1".to_string(), "v1".to_string()),
                ("k2".to_string(), "v2".to_string()),
            ],
        };
        let display = set_multi.to_string();
        assert!(display.contains("SetMulti"));
        assert!(display.contains("k1"));
        assert!(display.contains("v1"));

        let delete_multi = AppRequest::DeleteMulti {
            keys: vec!["a".to_string(), "b".to_string()],
        };
        let display = delete_multi.to_string();
        assert!(display.contains("DeleteMulti"));
        assert!(display.contains("a"));
        assert!(display.contains("b"));
    }

    #[test]
    fn test_app_response_fields() {
        let response = AppResponse {
            value: Some("test".to_string()),
            deleted: Some(true),
            ..Default::default()
        };
        assert_eq!(response.value, Some("test".to_string()));
        assert_eq!(response.deleted, Some(true));
        assert!(response.cas_succeeded.is_none());

        let empty = AppResponse::default();
        assert!(empty.value.is_none());
        assert!(empty.deleted.is_none());
        assert!(empty.cas_succeeded.is_none());

        let cas_response = AppResponse {
            value: Some("new_value".to_string()),
            cas_succeeded: Some(true),
            ..Default::default()
        };
        assert_eq!(cas_response.cas_succeeded, Some(true));
    }

    #[test]
    fn test_raft_member_info_default() {
        // Default creates a valid RaftMemberInfo with zero seed
        let default_info = RaftMemberInfo::default();
        let display = default_info.to_string();
        assert!(display.contains("RaftMemberInfo"));
    }

    #[test]
    fn test_node_id_clone_and_copy() {
        let original = NodeId::from(42);
        let copied = original; // Copy trait - no clone needed

        assert_eq!(original, copied);
        // Verify both are still accessible (proves Copy, not move)
        assert_eq!(original.0, 42);
        assert_eq!(copied.0, 42);
    }

    #[test]
    fn test_node_id_debug_format() {
        let node = NodeId::from(123);
        let debug = format!("{:?}", node);
        assert!(debug.contains("NodeId"));
        assert!(debug.contains("123"));
    }
}
