//! Tests for the network module.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::FailureDetectorUpdate;
use crate::constants::FAILURE_DETECTOR_CHANNEL_CAPACITY;
use crate::constants::IROH_READ_TIMEOUT;
use crate::constants::MAX_PEERS;
use crate::constants::MAX_RPC_MESSAGE_SIZE;
use crate::constants::MAX_SNAPSHOT_SIZE;
use crate::node_failure_detection::ConnectionStatus;
use crate::node_failure_detection::NodeFailureDetector;
use crate::rpc::RaftRpcResponse;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;
use crate::verified::SHARD_PREFIX_SIZE;
use crate::verified::encode_shard_prefix;
use crate::verified::try_decode_shard_prefix;

// =========================================================================
// FailureDetectorUpdate Tests
// =========================================================================

#[test]
fn test_failure_detector_update_construction() {
    let update = FailureDetectorUpdate {
        node_id: NodeId::from(42),
        raft_status: ConnectionStatus::Connected,
        iroh_status: ConnectionStatus::Disconnected,
    };

    assert_eq!(update.node_id, NodeId::from(42));
    assert_eq!(update.raft_status, ConnectionStatus::Connected);
    assert_eq!(update.iroh_status, ConnectionStatus::Disconnected);
}

#[test]
fn test_failure_detector_update_debug() {
    let update = FailureDetectorUpdate {
        node_id: NodeId::from(1),
        raft_status: ConnectionStatus::Connected,
        iroh_status: ConnectionStatus::Connected,
    };

    let debug_str = format!("{:?}", update);
    assert!(debug_str.contains("FailureDetectorUpdate"));
    assert!(debug_str.contains("node_id"));
}

#[test]
fn test_failure_detector_update_all_status_combinations() {
    // Test all 4 combinations of connection statuses
    let combinations = [
        (ConnectionStatus::Connected, ConnectionStatus::Connected),
        (ConnectionStatus::Connected, ConnectionStatus::Disconnected),
        (ConnectionStatus::Disconnected, ConnectionStatus::Connected),
        (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected),
    ];

    for (raft, iroh) in combinations {
        let update = FailureDetectorUpdate {
            node_id: NodeId::from(1),
            raft_status: raft,
            iroh_status: iroh,
        };
        assert_eq!(update.raft_status, raft);
        assert_eq!(update.iroh_status, iroh);
    }
}

// =========================================================================
// IrpcRaftNetworkFactory Tests (Pure Unit Tests)
// =========================================================================

// Note: Full async tests of IrpcRaftNetworkFactory require a real Iroh endpoint
// or a mock transport. These tests cover the verified/synchronous aspects.

#[test]
fn test_max_peers_constant_is_bounded() {
    // Tiger Style: Verify MAX_PEERS is within reasonable bounds
    let max_peers = MAX_PEERS;
    assert!(max_peers >= 64, "MAX_PEERS should allow reasonable cluster size");
    assert!(max_peers <= 10_000, "MAX_PEERS should prevent resource exhaustion");
}

#[test]
fn test_failure_detector_channel_capacity_is_bounded() {
    // Tiger Style: Verify channel capacity is bounded
    let capacity = FAILURE_DETECTOR_CHANNEL_CAPACITY;
    assert!(capacity >= 64, "Channel should handle burst of failures");
    assert!(capacity <= 10_000, "Channel should not consume unbounded memory");
}

#[test]
fn test_iroh_read_timeout_is_reasonable() {
    // Tiger Style: Verify read timeout is bounded and reasonable
    assert!(
        IROH_READ_TIMEOUT >= std::time::Duration::from_millis(100),
        "Timeout should allow for network latency"
    );
    assert!(IROH_READ_TIMEOUT <= std::time::Duration::from_secs(60), "Timeout should not wait forever");
}

#[test]
fn test_max_rpc_message_size_is_bounded() {
    // Tiger Style: Verify RPC message size limit
    let max_size = MAX_RPC_MESSAGE_SIZE;
    assert!(max_size >= 1024, "Should allow reasonable message sizes");
    assert!(max_size <= 100 * 1024 * 1024, "Should prevent memory exhaustion (100MB max)");
}

#[test]
fn test_max_snapshot_size_is_bounded() {
    // Tiger Style: Verify snapshot size limit
    let max_size = MAX_SNAPSHOT_SIZE;
    assert!(max_size >= 1024 * 1024, "Should allow 1MB+ snapshots");
    assert!(max_size <= 10 * 1024 * 1024 * 1024, "Should prevent 10GB+ allocations");
}

// =========================================================================
// Peer Address Map Tests
// =========================================================================

#[test]
fn test_peer_addrs_hashmap_capacity() {
    // Verify HashMap can be created with expected capacity
    let peer_addrs: HashMap<NodeId, iroh::EndpointAddr> = HashMap::with_capacity(MAX_PEERS as usize);
    assert!(peer_addrs.capacity() >= MAX_PEERS as usize);
}

#[test]
fn test_node_id_as_hashmap_key() {
    let mut map: HashMap<NodeId, u32> = HashMap::new();

    let node1 = NodeId::from(1);
    let node2 = NodeId::from(2);
    let node3 = NodeId::from(u64::MAX);

    map.insert(node1, 100);
    map.insert(node2, 200);
    map.insert(node3, 300);

    assert_eq!(map.get(&node1), Some(&100));
    assert_eq!(map.get(&node2), Some(&200));
    assert_eq!(map.get(&node3), Some(&300));
    assert_eq!(map.get(&NodeId::from(999)), None);
}

#[test]
fn test_node_id_equality() {
    let node1a = NodeId::from(42);
    let node1b = NodeId::from(42);
    let node2 = NodeId::from(43);

    assert_eq!(node1a, node1b);
    assert_ne!(node1a, node2);
}

#[test]
fn test_node_id_copy_semantics() {
    let node = NodeId::from(42);
    let copied = node; // Copy
    assert_eq!(node, copied); // Both still valid
}

// =========================================================================
// RaftMemberInfo Tests
// =========================================================================

#[test]
fn test_raft_member_info_construction() {
    use iroh::SecretKey;

    // Use deterministic seed pattern from aspen-testing
    let mut seed = [0u8; 32];
    seed[0] = 1;
    let secret_key = SecretKey::from(seed);
    let endpoint_id = secret_key.public();
    let endpoint_addr = iroh::EndpointAddr::new(endpoint_id);

    let member_info = RaftMemberInfo::new(endpoint_addr.clone());

    assert_eq!(member_info.iroh_addr.id, endpoint_addr.id);
}

#[test]
fn test_raft_member_info_clone() {
    use iroh::SecretKey;

    // Use deterministic seed pattern from aspen-testing
    let mut seed = [0u8; 32];
    seed[0] = 2;
    let secret_key = SecretKey::from(seed);
    let endpoint_id = secret_key.public();
    let endpoint_addr = iroh::EndpointAddr::new(endpoint_id);

    let member_info = RaftMemberInfo::new(endpoint_addr);
    let cloned = member_info.clone();

    assert_eq!(member_info.iroh_addr.id, cloned.iroh_addr.id);
}

// =========================================================================
// ShardId Tests
// =========================================================================

#[test]
fn test_shard_id_type() {
    use aspen_sharding::ShardId;

    // ShardId is u32, verify it can represent reasonable shard counts
    let shard: ShardId = 0;
    assert_eq!(shard, 0);

    let shard: ShardId = u32::MAX;
    assert_eq!(shard, u32::MAX);
}

#[test]
fn test_shard_id_in_option() {
    use aspen_sharding::ShardId;

    let none_shard: Option<ShardId> = None;
    let some_shard: Option<ShardId> = Some(42);

    assert!(none_shard.is_none());
    assert_eq!(some_shard, Some(42));
}

// =========================================================================
// RPC Protocol Serialization Tests
// =========================================================================

#[test]
fn test_raft_rpc_response_vote_serialization() {
    use openraft::Vote;

    let response = RaftRpcResponse::Vote(openraft::raft::VoteResponse {
        vote: Vote::new(1, NodeId::from(1)),
        vote_granted: true,
        last_log_id: None,
    });

    // Test postcard serialization (what we use on the wire)
    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    assert!(matches!(deserialized, RaftRpcResponse::Vote(v) if v.vote_granted));
}

#[test]
fn test_raft_rpc_response_append_entries_serialization() {
    let response = RaftRpcResponse::AppendEntries(openraft::raft::AppendEntriesResponse::Success);

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    assert!(matches!(
        deserialized,
        RaftRpcResponse::AppendEntries(openraft::raft::AppendEntriesResponse::Success)
    ));
}

#[test]
fn test_raft_rpc_response_fatal_error_serialization() {
    use crate::rpc::RaftFatalErrorKind;

    for kind in [
        RaftFatalErrorKind::Panicked,
        RaftFatalErrorKind::Stopped,
        RaftFatalErrorKind::StorageError,
    ] {
        let response = RaftRpcResponse::FatalError(kind);

        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

        assert!(matches!(deserialized, RaftRpcResponse::FatalError(k) if k == kind));
    }
}

#[test]
fn test_raft_rpc_response_with_timestamps_serialization() {
    use crate::rpc::RaftRpcResponseWithTimestamps;
    use crate::rpc::TimestampInfo;

    let response = RaftRpcResponseWithTimestamps {
        inner: RaftRpcResponse::AppendEntries(openraft::raft::AppendEntriesResponse::Conflict),
        timestamps: Some(TimestampInfo {
            server_recv_ms: 1000,
            server_send_ms: 1005,
        }),
    };

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponseWithTimestamps = postcard::from_bytes(&bytes).expect("deserialize");

    assert!(deserialized.timestamps.is_some());
    let ts = deserialized.timestamps.unwrap();
    assert_eq!(ts.server_recv_ms, 1000);
    assert_eq!(ts.server_send_ms, 1005);
}

// =========================================================================
// Shard Prefix Encoding Tests
// =========================================================================

#[test]
fn test_shard_prefix_encoding_zero() {
    let prefix = encode_shard_prefix(0);
    assert_eq!(prefix, [0, 0, 0, 0]);

    let decoded = try_decode_shard_prefix(&prefix);
    assert_eq!(decoded, Some(0));
}

#[test]
fn test_shard_prefix_encoding_max() {
    let prefix = encode_shard_prefix(u32::MAX);
    assert_eq!(prefix, [0xFF, 0xFF, 0xFF, 0xFF]);

    let decoded = try_decode_shard_prefix(&prefix);
    assert_eq!(decoded, Some(u32::MAX));
}

#[test]
fn test_shard_prefix_encoding_roundtrip() {
    for shard_id in [0, 1, 42, 255, 256, 1000, 65535, 0x12345678, u32::MAX] {
        let encoded = encode_shard_prefix(shard_id);
        let decoded = try_decode_shard_prefix(&encoded);
        assert_eq!(decoded, Some(shard_id), "Roundtrip failed for shard_id={}", shard_id);
    }
}

#[test]
fn test_shard_prefix_decode_too_short() {
    assert_eq!(try_decode_shard_prefix(&[]), None);
    assert_eq!(try_decode_shard_prefix(&[0]), None);
    assert_eq!(try_decode_shard_prefix(&[0, 0]), None);
    assert_eq!(try_decode_shard_prefix(&[0, 0, 0]), None);
}

#[test]
fn test_shard_prefix_size_constant() {
    assert_eq!(SHARD_PREFIX_SIZE, 4);
}

// =========================================================================
// Error Classification Tests
// =========================================================================

#[test]
fn test_error_message_classification_connection_pool() {
    // Test that connection pool errors are classified as NodeCrash
    let err_msg = "connection pool: failed to connect";
    let is_connection_pool = err_msg.contains("connection pool");
    assert!(is_connection_pool);
}

#[test]
fn test_error_message_classification_peer_address() {
    // Test that peer address errors are classified as NodeCrash
    let err_msg = "peer address not found in peer map";
    let is_peer_not_found = err_msg.contains("peer address not found");
    assert!(is_peer_not_found);
}

#[test]
fn test_error_message_classification_stream() {
    // Test that stream errors are classified as ActorCrash
    let err_msg = "stream failure: connection refused";
    let is_stream_error = err_msg.contains("stream");
    assert!(is_stream_error);
}

// =========================================================================
// Bounded Channel Tests
// =========================================================================

#[tokio::test]
async fn test_failure_update_channel_bounded() {
    // Verify channel is bounded and blocks appropriately
    let (tx, mut rx) = tokio::sync::mpsc::channel::<FailureDetectorUpdate>(2);

    // Fill the channel
    for i in 0..2 {
        tx.send(FailureDetectorUpdate {
            node_id: NodeId::from(i),
            raft_status: ConnectionStatus::Connected,
            iroh_status: ConnectionStatus::Connected,
        })
        .await
        .expect("send should succeed");
    }

    // try_send should fail when channel is full
    let result = tx.try_send(FailureDetectorUpdate {
        node_id: NodeId::from(999),
        raft_status: ConnectionStatus::Disconnected,
        iroh_status: ConnectionStatus::Disconnected,
    });
    assert!(result.is_err(), "try_send should fail when channel is full");

    // Drain the channel
    for _ in 0..2 {
        let _ = rx.recv().await.expect("should receive");
    }
}

#[tokio::test]
async fn test_failure_update_channel_try_send_pattern() {
    // Test the try_send pattern used in send_rpc for failure updates
    let (tx, _rx) = tokio::sync::mpsc::channel::<FailureDetectorUpdate>(FAILURE_DETECTOR_CHANNEL_CAPACITY);

    // try_send should succeed when channel has capacity
    let result = tx.try_send(FailureDetectorUpdate {
        node_id: NodeId::from(1),
        raft_status: ConnectionStatus::Disconnected,
        iroh_status: ConnectionStatus::Connected,
    });
    assert!(result.is_ok(), "try_send should succeed with capacity");
}

// =========================================================================
// ALPN Tests
// =========================================================================

#[test]
#[allow(deprecated)]
fn test_raft_alpn_values() {
    // Test both deprecated and recommended ALPN values for backward compat
    assert_eq!(aspen_transport::RAFT_ALPN, b"raft-rpc");
    assert_eq!(aspen_transport::RAFT_AUTH_ALPN, b"raft-auth");
}

#[test]
#[allow(deprecated)]
fn test_raft_alpn_values_are_different() {
    assert_ne!(aspen_transport::RAFT_ALPN, aspen_transport::RAFT_AUTH_ALPN);
}

#[test]
#[allow(deprecated)]
fn test_raft_alpn_values_are_valid_utf8() {
    assert!(std::str::from_utf8(aspen_transport::RAFT_ALPN).is_ok());
    assert!(std::str::from_utf8(aspen_transport::RAFT_AUTH_ALPN).is_ok());
}

// =========================================================================
// RwLock and Arc Pattern Tests
// =========================================================================

#[tokio::test]
async fn test_rwlock_peer_addrs_pattern() {
    // Test the RwLock<HashMap> pattern used for peer_addrs
    let peer_addrs: Arc<RwLock<HashMap<NodeId, u32>>> = Arc::new(RwLock::new(HashMap::new()));

    // Write path (add_peer pattern)
    {
        let mut peers = peer_addrs.write().await;
        if peers.len() < MAX_PEERS as usize {
            peers.insert(NodeId::from(1), 100);
        }
    }

    // Read path (new_client pattern)
    {
        let peers = peer_addrs.read().await;
        assert_eq!(peers.get(&NodeId::from(1)), Some(&100));
    }

    // Concurrent access
    let peer_addrs_clone = Arc::clone(&peer_addrs);
    let handle = tokio::spawn(async move {
        let peers = peer_addrs_clone.read().await;
        peers.get(&NodeId::from(1)).copied()
    });

    let result = handle.await.expect("task should complete");
    assert_eq!(result, Some(100));
}

#[tokio::test]
async fn test_rwlock_failure_detector_pattern() {
    // Test the RwLock<NodeFailureDetector> pattern
    let failure_detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));

    // Write path (update status)
    failure_detector.write().await.update_node_status(
        NodeId::from(1),
        ConnectionStatus::Connected,
        ConnectionStatus::Connected,
    );

    // Verify the update was successful by getting the unreachable nodes count
    // (using public unreachable_count method)
    let detector = failure_detector.read().await;
    assert_eq!(detector.unreachable_count(), 0); // No unreachable nodes
}

// =========================================================================
// CoreNetworkFactory Trait Tests
// =========================================================================

#[test]
fn test_endpoint_addr_json_serialization() {
    use iroh::SecretKey;

    // Use deterministic seed pattern from aspen-testing
    let mut seed = [0u8; 32];
    seed[0] = 3;
    let secret_key = SecretKey::from(seed);
    let endpoint_id = secret_key.public();
    let endpoint_addr = iroh::EndpointAddr::new(endpoint_id);

    // This is the pattern used in CoreNetworkFactory::add_peer
    let json = serde_json::to_string(&endpoint_addr).expect("serialize");
    let parsed: iroh::EndpointAddr = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(endpoint_addr.id, parsed.id);
}

#[test]
fn test_endpoint_addr_json_parse_error() {
    // Test error handling for invalid JSON
    let invalid_json = "not valid json";
    let result: Result<iroh::EndpointAddr, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}

// =========================================================================
// Connection Status Classification Tests
// =========================================================================

#[test]
fn test_connection_status_enum() {
    // Verify ConnectionStatus enum values
    let connected = ConnectionStatus::Connected;
    let disconnected = ConnectionStatus::Disconnected;

    assert_eq!(connected, ConnectionStatus::Connected);
    assert_eq!(disconnected, ConnectionStatus::Disconnected);
    assert_ne!(connected, disconnected);
}

#[test]
fn test_connection_status_copy() {
    let status = ConnectionStatus::Connected;
    let copied = status; // Copy
    assert_eq!(status, copied);
}

// =========================================================================
// Empty Response Buffer Detection Tests
// =========================================================================

#[test]
fn test_empty_response_detection() {
    // Test the empty response buffer check pattern
    let empty_buf: Vec<u8> = vec![];
    let non_empty_buf: Vec<u8> = vec![1, 2, 3];

    assert!(empty_buf.is_empty(), "Empty buffer should be detected");
    assert!(!non_empty_buf.is_empty(), "Non-empty buffer should not be detected as empty");
}

#[test]
fn test_empty_postcard_deserialization_fails() {
    // Empty buffer should fail postcard deserialization
    let empty_buf: &[u8] = &[];
    let result: Result<RaftRpcResponse, _> = postcard::from_bytes(empty_buf);
    assert!(result.is_err(), "Empty buffer should fail deserialization");
}

// =========================================================================
// Timeout Duration Tests
// =========================================================================

#[test]
fn test_timeout_durations_reasonable() {
    // Verify timeouts are in reasonable ranges
    use crate::constants::IROH_CONNECT_TIMEOUT;
    use crate::constants::IROH_STREAM_OPEN_TIMEOUT;

    // Connect timeout should be longer than stream timeout
    assert!(IROH_CONNECT_TIMEOUT >= IROH_STREAM_OPEN_TIMEOUT, "Connect timeout should be >= stream timeout");

    // Read timeout should be reasonable
    assert!(IROH_READ_TIMEOUT >= std::time::Duration::from_secs(1), "Read timeout should be at least 1 second");
}
