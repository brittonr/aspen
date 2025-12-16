//! Property-based tests for network layer.
//!
//! This module tests properties of:
//! - RPC message serialization idempotency
//! - TUI RPC message roundtrip
//! - Gossip message handling
//!
//! These tests complement the fuzz targets by verifying semantic correctness
//! through property-based testing.

use proptest::prelude::*;

use aspen::tui_rpc::{TuiRpcRequest, TuiRpcResponse};

// Strategy to generate valid keys (non-empty, bounded size)
fn valid_key() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z][a-zA-Z0-9_]{0,99}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

// Strategy to generate valid values (bounded size)
fn valid_value() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..1000)
}

// Strategy to generate node IDs
fn valid_node_id() -> impl Strategy<Value = u64> {
    1u64..1000u64
}

// Strategy to generate membership lists
fn valid_membership() -> impl Strategy<Value = Vec<u64>> {
    prop::collection::vec(valid_node_id(), 1..10)
}

// Strategy to generate addresses
fn valid_address() -> impl Strategy<Value = String> {
    (
        0u8..255u8,
        0u8..255u8,
        0u8..255u8,
        0u8..255u8,
        1024u16..65535u16,
    )
        .prop_map(|(a, b, c, d, port)| format!("{}.{}.{}.{}:{}", a, b, c, d, port))
}

// Test 1: TUI RPC Request serialization roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_tui_request_roundtrip_simple_variants(
        seed in 0u8..20u8,
    ) {
        // Test simple variants (no data)
        let request = match seed % 10 {
            0 => TuiRpcRequest::Ping,
            1 => TuiRpcRequest::GetHealth,
            2 => TuiRpcRequest::GetRaftMetrics,
            3 => TuiRpcRequest::GetLeader,
            4 => TuiRpcRequest::GetNodeInfo,
            5 => TuiRpcRequest::GetClusterTicket,
            6 => TuiRpcRequest::InitCluster,
            7 => TuiRpcRequest::TriggerSnapshot,
            8 => TuiRpcRequest::GetClusterState,
            _ => TuiRpcRequest::Ping,
        };

        // Serialize
        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize request");

        // Deserialize
        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize request");

        // Re-serialize and compare bytes (idempotency)
        let bytes2 = postcard::to_stdvec(&roundtripped)
            .expect("Failed to re-serialize request");

        prop_assert_eq!(bytes, bytes2, "Serialization should be idempotent");
    }
}

// Test 2: TUI RPC Request with data roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_tui_request_read_key_roundtrip(
        key in valid_key(),
    ) {
        let request = TuiRpcRequest::ReadKey { key: key.clone() };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize ReadKey request");

        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize ReadKey request");

        match roundtripped {
            TuiRpcRequest::ReadKey { key: roundtripped_key } => {
                prop_assert_eq!(key, roundtripped_key);
            }
            _ => prop_assert!(false, "Wrong variant after roundtrip"),
        }
    }
}

// Test 3: TUI RPC Request WriteKey roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_tui_request_write_key_roundtrip(
        key in valid_key(),
        value in valid_value(),
    ) {
        let request = TuiRpcRequest::WriteKey {
            key: key.clone(),
            value: value.clone(),
        };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize WriteKey request");

        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize WriteKey request");

        match roundtripped {
            TuiRpcRequest::WriteKey {
                key: roundtripped_key,
                value: roundtripped_value,
            } => {
                prop_assert_eq!(key, roundtripped_key);
                prop_assert_eq!(value, roundtripped_value);
            }
            _ => prop_assert!(false, "Wrong variant after roundtrip"),
        }
    }
}

// Test 4: TUI RPC Request AddLearner roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_tui_request_add_learner_roundtrip(
        node_id in valid_node_id(),
        addr in valid_address(),
    ) {
        let request = TuiRpcRequest::AddLearner {
            node_id,
            addr: addr.clone(),
        };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize AddLearner request");

        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize AddLearner request");

        match roundtripped {
            TuiRpcRequest::AddLearner {
                node_id: roundtripped_id,
                addr: roundtripped_addr,
            } => {
                prop_assert_eq!(node_id, roundtripped_id);
                prop_assert_eq!(addr, roundtripped_addr);
            }
            _ => prop_assert!(false, "Wrong variant after roundtrip"),
        }
    }
}

// Test 5: TUI RPC Request ChangeMembership roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_tui_request_change_membership_roundtrip(
        members in valid_membership(),
    ) {
        let request = TuiRpcRequest::ChangeMembership {
            members: members.clone(),
        };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize ChangeMembership request");

        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize ChangeMembership request");

        match roundtripped {
            TuiRpcRequest::ChangeMembership {
                members: roundtripped_members,
            } => {
                prop_assert_eq!(members, roundtripped_members);
            }
            _ => prop_assert!(false, "Wrong variant after roundtrip"),
        }
    }
}

// Test 6: TUI RPC Response roundtrip
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_tui_response_pong_roundtrip(
        seed in 0u8..10u8,
    ) {
        let response = match seed % 3 {
            0 => TuiRpcResponse::Pong,
            1 => TuiRpcResponse::Leader(Some(seed as u64)),
            _ => TuiRpcResponse::Leader(None),
        };

        let bytes = postcard::to_stdvec(&response)
            .expect("Failed to serialize response");

        let roundtripped: TuiRpcResponse = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize response");

        // Re-serialize to verify idempotency
        let bytes2 = postcard::to_stdvec(&roundtripped)
            .expect("Failed to re-serialize response");

        prop_assert_eq!(bytes, bytes2);
    }
}

// Test 7: Serialization size bounds
proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn test_serialized_size_bounded(
        key in valid_key(),
        value in valid_value(),
    ) {
        let request = TuiRpcRequest::WriteKey {
            key: key.clone(),
            value: value.clone(),
        };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize request");

        // Serialized size should be bounded by input size + overhead
        // postcard uses variable-length encoding, so overhead is small
        let max_expected_size = key.len() + value.len() + 100; // 100 bytes overhead

        prop_assert!(
            bytes.len() <= max_expected_size,
            "Serialized size {} exceeds expected max {}",
            bytes.len(),
            max_expected_size
        );
    }
}

// Test 8: Empty key handling
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_empty_value_handling(
        key in valid_key(),
    ) {
        let request = TuiRpcRequest::WriteKey {
            key: key.clone(),
            value: vec![], // Empty value
        };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize request with empty value");

        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize request with empty value");

        match roundtripped {
            TuiRpcRequest::WriteKey { value, .. } => {
                prop_assert!(value.is_empty());
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }
}

// Test 9: Large membership list
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_large_membership_list(
        num_members in 50usize..100usize,
    ) {
        let members: Vec<u64> = (1..=num_members as u64).collect();

        let request = TuiRpcRequest::ChangeMembership {
            members: members.clone(),
        };

        let bytes = postcard::to_stdvec(&request)
            .expect("Failed to serialize large membership");

        let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
            .expect("Failed to deserialize large membership");

        match roundtripped {
            TuiRpcRequest::ChangeMembership {
                members: roundtripped_members,
            } => {
                prop_assert_eq!(members.len(), roundtripped_members.len());
                for (orig, rt) in members.iter().zip(roundtripped_members.iter()) {
                    prop_assert_eq!(orig, rt);
                }
            }
            _ => prop_assert!(false, "Wrong variant"),
        }
    }
}

// Test 10: Binary value with special bytes
proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn test_binary_value_special_bytes(
        key in valid_key(),
    ) {
        // Values with special bytes that might cause issues
        let special_values = vec![
            vec![0x00], // Null byte
            vec![0x00, 0x00, 0x00], // Multiple nulls
            vec![0xff], // High byte
            vec![0xff, 0xff, 0xff], // Multiple high bytes
            (0u8..=255u8).collect::<Vec<_>>(), // All bytes
        ];

        for value in special_values {
            let request = TuiRpcRequest::WriteKey {
                key: key.clone(),
                value: value.clone(),
            };

            let bytes = postcard::to_stdvec(&request)
                .expect("Failed to serialize special bytes");

            let roundtripped: TuiRpcRequest = postcard::from_bytes(&bytes)
                .expect("Failed to deserialize special bytes");

            match roundtripped {
                TuiRpcRequest::WriteKey { value: rt_value, .. } => {
                    prop_assert_eq!(value, rt_value);
                }
                _ => prop_assert!(false, "Wrong variant"),
            }
        }
    }
}
