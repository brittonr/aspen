//! Property-based tests for network layer.
//!
//! This module tests properties of:
//! - RPC message serialization idempotency
//! - TUI RPC message roundtrip
//! - Gossip message handling
//!
//! These tests complement the fuzz targets by verifying semantic correctness
//! through property-based testing.

mod support;

use bolero::check;

use aspen::client_rpc::{ClientRpcRequest, ClientRpcResponse};
use support::bolero_generators::{MembershipList, ValidApiKey, ValidIpAddr};

// Test 1: TUI RPC Request serialization roundtrip
#[test]
fn test_tui_request_roundtrip_simple_variants() {
    check!()
        .with_iterations(100)
        .with_type::<u8>()
        .for_each(|seed| {
            // Test simple variants (no data)
            let request = match seed % 10 {
                0 => ClientRpcRequest::Ping,
                1 => ClientRpcRequest::GetHealth,
                2 => ClientRpcRequest::GetRaftMetrics,
                3 => ClientRpcRequest::GetLeader,
                4 => ClientRpcRequest::GetNodeInfo,
                5 => ClientRpcRequest::GetClusterTicket,
                6 => ClientRpcRequest::InitCluster,
                7 => ClientRpcRequest::TriggerSnapshot,
                8 => ClientRpcRequest::GetClusterState,
                _ => ClientRpcRequest::Ping,
            };

            // Serialize
            let bytes = postcard::to_stdvec(&request).expect("Failed to serialize request");

            // Deserialize
            let roundtripped: ClientRpcRequest =
                postcard::from_bytes(&bytes).expect("Failed to deserialize request");

            // Re-serialize and compare bytes (idempotency)
            let bytes2 =
                postcard::to_stdvec(&roundtripped).expect("Failed to re-serialize request");

            assert_eq!(bytes, bytes2, "Serialization should be idempotent");
        });
}

// Test 2: TUI RPC Request with data roundtrip
#[test]
fn test_tui_request_read_key_roundtrip() {
    check!()
        .with_iterations(50)
        .with_type::<ValidApiKey>()
        .for_each(|key| {
            let request = ClientRpcRequest::ReadKey { key: key.0.clone() };

            let bytes = postcard::to_stdvec(&request).expect("Failed to serialize ReadKey request");

            let roundtripped: ClientRpcRequest =
                postcard::from_bytes(&bytes).expect("Failed to deserialize ReadKey request");

            match roundtripped {
                ClientRpcRequest::ReadKey {
                    key: roundtripped_key,
                } => {
                    assert_eq!(key.0, roundtripped_key);
                }
                _ => panic!("Wrong variant after roundtrip"),
            }
        });
}

// Test 3: TUI RPC Request WriteKey roundtrip
#[test]
fn test_tui_request_write_key_roundtrip() {
    check!()
        .with_iterations(50)
        .with_type::<(ValidApiKey, Vec<u8>)>()
        .for_each(|(key, value)| {
            let value: Vec<u8> = value.iter().take(1000).copied().collect();

            let request = ClientRpcRequest::WriteKey {
                key: key.0.clone(),
                value: value.clone(),
            };

            let bytes =
                postcard::to_stdvec(&request).expect("Failed to serialize WriteKey request");

            let roundtripped: ClientRpcRequest =
                postcard::from_bytes(&bytes).expect("Failed to deserialize WriteKey request");

            match roundtripped {
                ClientRpcRequest::WriteKey {
                    key: roundtripped_key,
                    value: roundtripped_value,
                } => {
                    assert_eq!(key.0, roundtripped_key);
                    assert_eq!(value, roundtripped_value);
                }
                _ => panic!("Wrong variant after roundtrip"),
            }
        });
}

// Test 4: TUI RPC Request AddLearner roundtrip
#[test]
fn test_tui_request_add_learner_roundtrip() {
    check!()
        .with_iterations(50)
        .with_type::<(u64, ValidIpAddr)>()
        .for_each(|(node_id, addr)| {
            let node_id = 1 + (*node_id % 999);

            let request = ClientRpcRequest::AddLearner {
                node_id,
                addr: addr.0.clone(),
            };

            let bytes =
                postcard::to_stdvec(&request).expect("Failed to serialize AddLearner request");

            let roundtripped: ClientRpcRequest =
                postcard::from_bytes(&bytes).expect("Failed to deserialize AddLearner request");

            match roundtripped {
                ClientRpcRequest::AddLearner {
                    node_id: roundtripped_id,
                    addr: roundtripped_addr,
                } => {
                    assert_eq!(node_id, roundtripped_id);
                    assert_eq!(addr.0, roundtripped_addr);
                }
                _ => panic!("Wrong variant after roundtrip"),
            }
        });
}

// Test 5: TUI RPC Request ChangeMembership roundtrip
#[test]
fn test_tui_request_change_membership_roundtrip() {
    check!()
        .with_iterations(50)
        .with_type::<MembershipList>()
        .for_each(|members| {
            let request = ClientRpcRequest::ChangeMembership {
                members: members.0.clone(),
            };

            let bytes = postcard::to_stdvec(&request)
                .expect("Failed to serialize ChangeMembership request");

            let roundtripped: ClientRpcRequest = postcard::from_bytes(&bytes)
                .expect("Failed to deserialize ChangeMembership request");

            match roundtripped {
                ClientRpcRequest::ChangeMembership {
                    members: roundtripped_members,
                } => {
                    assert_eq!(members.0, roundtripped_members);
                }
                _ => panic!("Wrong variant after roundtrip"),
            }
        });
}

// Test 6: TUI RPC Response roundtrip
#[test]
fn test_tui_response_pong_roundtrip() {
    check!()
        .with_iterations(50)
        .with_type::<u8>()
        .for_each(|seed| {
            let response = match seed % 3 {
                0 => ClientRpcResponse::Pong,
                1 => ClientRpcResponse::Leader(Some(*seed as u64)),
                _ => ClientRpcResponse::Leader(None),
            };

            let bytes = postcard::to_stdvec(&response).expect("Failed to serialize response");

            let roundtripped: ClientRpcResponse =
                postcard::from_bytes(&bytes).expect("Failed to deserialize response");

            // Re-serialize to verify idempotency
            let bytes2 =
                postcard::to_stdvec(&roundtripped).expect("Failed to re-serialize response");

            assert_eq!(bytes, bytes2);
        });
}

// Test 7: Serialization size bounds
#[test]
fn test_serialized_size_bounded() {
    check!()
        .with_iterations(30)
        .with_type::<(ValidApiKey, Vec<u8>)>()
        .for_each(|(key, value)| {
            let value: Vec<u8> = value.iter().take(1000).copied().collect();

            let request = ClientRpcRequest::WriteKey {
                key: key.0.clone(),
                value: value.clone(),
            };

            let bytes = postcard::to_stdvec(&request).expect("Failed to serialize request");

            // Serialized size should be bounded by input size + overhead
            // postcard uses variable-length encoding, so overhead is small
            let max_expected_size = key.0.len() + value.len() + 100; // 100 bytes overhead

            assert!(
                bytes.len() <= max_expected_size,
                "Serialized size {} exceeds expected max {}",
                bytes.len(),
                max_expected_size
            );
        });
}

// Test 8: Empty value handling
#[test]
fn test_empty_value_handling() {
    check!()
        .with_iterations(10)
        .with_type::<ValidApiKey>()
        .for_each(|key| {
            let request = ClientRpcRequest::WriteKey {
                key: key.0.clone(),
                value: vec![], // Empty value
            };

            let bytes = postcard::to_stdvec(&request)
                .expect("Failed to serialize request with empty value");

            let roundtripped: ClientRpcRequest = postcard::from_bytes(&bytes)
                .expect("Failed to deserialize request with empty value");

            match roundtripped {
                ClientRpcRequest::WriteKey { value, .. } => {
                    assert!(value.is_empty());
                }
                _ => panic!("Wrong variant"),
            }
        });
}

// Test 9: Large membership list
#[test]
fn test_large_membership_list() {
    check!()
        .with_iterations(10)
        .with_type::<u8>()
        .for_each(|num_members| {
            let num_members = 50 + (*num_members as usize % 50);
            let members: Vec<u64> = (1..=num_members as u64).collect();

            let request = ClientRpcRequest::ChangeMembership {
                members: members.clone(),
            };

            let bytes =
                postcard::to_stdvec(&request).expect("Failed to serialize large membership");

            let roundtripped: ClientRpcRequest =
                postcard::from_bytes(&bytes).expect("Failed to deserialize large membership");

            match roundtripped {
                ClientRpcRequest::ChangeMembership {
                    members: roundtripped_members,
                } => {
                    assert_eq!(members.len(), roundtripped_members.len());
                    for (orig, rt) in members.iter().zip(roundtripped_members.iter()) {
                        assert_eq!(orig, rt);
                    }
                }
                _ => panic!("Wrong variant"),
            }
        });
}

// Test 10: Binary value with special bytes
#[test]
fn test_binary_value_special_bytes() {
    check!()
        .with_iterations(30)
        .with_type::<ValidApiKey>()
        .for_each(|key| {
            // Values with special bytes that might cause issues
            let special_values = vec![
                vec![0x00],                        // Null byte
                vec![0x00, 0x00, 0x00],            // Multiple nulls
                vec![0xff],                        // High byte
                vec![0xff, 0xff, 0xff],            // Multiple high bytes
                (0u8..=255u8).collect::<Vec<_>>(), // All bytes
            ];

            for value in special_values {
                let request = ClientRpcRequest::WriteKey {
                    key: key.0.clone(),
                    value: value.clone(),
                };

                let bytes =
                    postcard::to_stdvec(&request).expect("Failed to serialize special bytes");

                let roundtripped: ClientRpcRequest =
                    postcard::from_bytes(&bytes).expect("Failed to deserialize special bytes");

                match roundtripped {
                    ClientRpcRequest::WriteKey {
                        value: rt_value, ..
                    } => {
                        assert_eq!(value, rt_value);
                    }
                    _ => panic!("Wrong variant"),
                }
            }
        });
}
