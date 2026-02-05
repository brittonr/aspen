//! Tests for protocol handlers using mock Iroh infrastructure.
//!
//! This module tests the RPC serialization and deserialization logic used by
//! the Raft and TUI protocol handlers. Since the handlers are tightly coupled
//! to Iroh's Connection type, we test:
//!
//! 1. RPC message serialization/deserialization (postcard format)
//! 2. Stream-based request/response patterns
//! 3. Error handling for malformed messages
//! 4. Size limit enforcement
//!
//! The actual ProtocolHandler implementations are tested via integration tests
//! with real Iroh endpoints.

#![allow(deprecated)] // client_rpc is deprecated but still tested

mod support;

use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::MAX_CLIENT_MESSAGE_SIZE;
use aspen::raft::constants::MAX_RPC_MESSAGE_SIZE;
use aspen::raft::rpc::RaftRpcProtocol;
use aspen::raft::rpc::RaftVoteRequest;
use support::mock_iroh::CLIENT_ALPN;
use support::mock_iroh::MockIrohNetwork;
use support::mock_iroh::RAFT_ALPN;

/// Test that Raft RPC vote request serialization roundtrips correctly.
#[tokio::test]
async fn test_raft_vote_rpc_serialization() {
    use aspen::raft::types::NodeId;

    // Create a mock Vote request
    let vote_request = RaftVoteRequest {
        request: openraft::raft::VoteRequest {
            vote: openraft::Vote::new(1, NodeId::from(1)),
            last_log_id: None,
        },
    };

    let rpc = RaftRpcProtocol::Vote(vote_request);

    // Serialize to postcard format
    let bytes = postcard::to_stdvec(&rpc).expect("failed to serialize vote request");

    // Verify size is within limits
    assert!(bytes.len() <= MAX_RPC_MESSAGE_SIZE as usize, "serialized vote request exceeds MAX_RPC_MESSAGE_SIZE");

    // Deserialize back
    let deserialized: RaftRpcProtocol = postcard::from_bytes(&bytes).expect("failed to deserialize vote request");

    match deserialized {
        RaftRpcProtocol::Vote(_req) => {
            // Successfully roundtripped
        }
        _ => panic!("expected Vote variant"),
    }
}

/// Test that TUI RPC request serialization roundtrips correctly.
#[tokio::test]
async fn test_tui_rpc_serialization() {
    // Test various TUI request types
    let requests = vec![
        ClientRpcRequest::GetHealth,
        ClientRpcRequest::GetRaftMetrics,
        ClientRpcRequest::GetLeader,
        ClientRpcRequest::GetNodeInfo,
        ClientRpcRequest::GetClusterTicket,
        ClientRpcRequest::InitCluster,
        ClientRpcRequest::TriggerSnapshot,
        ClientRpcRequest::Ping,
        ClientRpcRequest::GetClusterState,
        ClientRpcRequest::ReadKey {
            key: "test-key".to_string(),
        },
        ClientRpcRequest::WriteKey {
            key: "test-key".to_string(),
            value: b"test-value".to_vec(),
        },
        ClientRpcRequest::AddLearner {
            node_id: 42,
            addr: "test-addr".to_string(),
        },
        ClientRpcRequest::ChangeMembership { members: vec![1, 2, 3] },
    ];

    for request in requests {
        let bytes = postcard::to_stdvec(&request).expect("failed to serialize TUI request");

        assert!(bytes.len() <= MAX_CLIENT_MESSAGE_SIZE, "serialized TUI request exceeds MAX_CLIENT_MESSAGE_SIZE");

        let deserialized: ClientRpcRequest = postcard::from_bytes(&bytes).expect("failed to deserialize TUI request");

        // Verify discriminants match (basic roundtrip check)
        assert_eq!(std::mem::discriminant(&request), std::mem::discriminant(&deserialized));
    }
}

/// Test Client response serialization roundtrips correctly.
#[tokio::test]
async fn test_client_response_serialization() {
    use aspen::client_rpc::*;

    let responses = vec![
        ClientRpcResponse::Health(HealthResponse {
            status: "healthy".to_string(),
            node_id: 1,
            raft_node_id: Some(1),
            uptime_seconds: 3600,
            is_initialized: true,
            membership_node_count: Some(3),
        }),
        ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
            node_id: 1,
            state: "Leader".to_string(),
            current_leader: Some(1),
            current_term: 5,
            last_log_index: Some(100),
            last_applied_index: Some(99),
            snapshot_index: Some(50),
            replication: None,
        }),
        ClientRpcResponse::Leader(Some(1)),
        ClientRpcResponse::NodeInfo(NodeInfoResponse {
            node_id: 1,
            endpoint_addr: "test-addr".to_string(),
        }),
        ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
            ticket: "aspen-ticket".to_string(),
            topic_id: "topic-id".to_string(),
            cluster_id: "cluster-id".to_string(),
            endpoint_id: "endpoint-id".to_string(),
            bootstrap_peers: Some(1),
        }),
        ClientRpcResponse::InitResult(InitResultResponse {
            success: true,
            error: None,
        }),
        ClientRpcResponse::ReadResult(ReadResultResponse {
            value: Some(b"test-value".to_vec()),
            found: true,
            error: None,
        }),
        ClientRpcResponse::WriteResult(WriteResultResponse {
            success: true,
            error: None,
        }),
        ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
            success: true,
            snapshot_index: Some(100),
            error: None,
        }),
        ClientRpcResponse::AddLearnerResult(AddLearnerResultResponse {
            success: true,
            error: None,
        }),
        ClientRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
            success: true,
            error: None,
        }),
        ClientRpcResponse::ClusterState(ClusterStateResponse {
            nodes: vec![NodeDescriptor {
                node_id: 1,
                endpoint_addr: "addr".to_string(),
                is_voter: true,
                is_learner: false,
                is_leader: true,
            }],
            leader_id: Some(1),
            this_node_id: 1,
        }),
        ClientRpcResponse::Pong,
        ClientRpcResponse::error("TEST_ERROR", "test error message"),
    ];

    for response in responses {
        let bytes = postcard::to_stdvec(&response).expect("failed to serialize TUI response");

        let deserialized: ClientRpcResponse = postcard::from_bytes(&bytes).expect("failed to deserialize TUI response");

        assert_eq!(std::mem::discriminant(&response), std::mem::discriminant(&deserialized));
    }
}

/// Test mock Iroh connection with RPC-like request/response pattern.
#[tokio::test]
async fn test_mock_rpc_pattern() {
    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Client connects to server
    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    // Simulate RPC: client sends request, server sends response

    // Client side: open stream and send request
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

    let request = ClientRpcRequest::Ping;
    let request_bytes = postcard::to_stdvec(&request).unwrap();
    send.write_all(&request_bytes).await.unwrap();
    send.finish().unwrap();

    // Server side: accept stream and read request
    let (mut server_send, mut server_recv) = server_conn.accept_bi().await.unwrap();

    let received_bytes = server_recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.unwrap();
    let received_request: ClientRpcRequest = postcard::from_bytes(&received_bytes).unwrap();

    assert!(matches!(received_request, ClientRpcRequest::Ping));

    // Server sends response
    let response = ClientRpcResponse::Pong;
    let response_bytes = postcard::to_stdvec(&response).unwrap();
    server_send.write_all(&response_bytes).await.unwrap();
    server_send.finish().unwrap();

    // Client receives response
    let response_received_bytes = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.unwrap();
    let received_response: ClientRpcResponse = postcard::from_bytes(&response_received_bytes).unwrap();

    assert!(matches!(received_response, ClientRpcResponse::Pong));
}

/// Test that oversized messages are rejected.
#[tokio::test]
async fn test_oversized_message_rejected() {
    let network = MockIrohNetwork::new();

    let ep1 = network.create_endpoint();
    let ep2 = network.create_endpoint();

    let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
    let (mut send, _recv) = conn1.open_bi().await.unwrap();

    // Try to send oversized data (11 MB, exceeds 10 MB limit)
    let oversized_data = vec![0u8; 11 * 1024 * 1024];
    let result = send.write_all(&oversized_data).await;

    assert!(result.is_err(), "oversized message should be rejected");
}

/// Test ALPN-based routing with mock network.
#[tokio::test]
async fn test_alpn_routing() {
    let network = MockIrohNetwork::new();

    // Server only accepts RAFT_ALPN
    let server = network.create_endpoint_with_key_and_alpns(iroh::SecretKey::from([1u8; 32]), vec![RAFT_ALPN.to_vec()]);

    let client = network.create_endpoint();

    // Connect with RAFT_ALPN should succeed
    let result = client.connect(server.id(), RAFT_ALPN).await;
    assert!(result.is_ok());

    // Connect with CLIENT_ALPN should fail (not supported)
    let result = client.connect(server.id(), CLIENT_ALPN).await;
    assert!(result.is_err());
}

/// Test multiple concurrent streams on a single connection.
#[tokio::test]
async fn test_concurrent_streams() {
    let network = MockIrohNetwork::new();

    let ep1 = network.create_endpoint();
    let ep2 = network.create_endpoint();

    let conn1 = ep1.connect(ep2.id(), RAFT_ALPN).await.unwrap();
    let incoming = ep2.accept().await.unwrap();
    let conn2 = incoming.accept();

    // Open multiple streams concurrently
    let (mut send1, _recv1) = conn1.open_bi().await.unwrap();
    let (mut send2, _recv2) = conn1.open_bi().await.unwrap();
    let (mut send3, _recv3) = conn1.open_bi().await.unwrap();

    // Accept all streams
    let (_, mut recv_a) = conn2.accept_bi().await.unwrap();
    let (_, mut recv_b) = conn2.accept_bi().await.unwrap();
    let (_, mut recv_c) = conn2.accept_bi().await.unwrap();

    // Send different data on each stream
    send1.write_all(b"stream1").await.unwrap();
    send1.finish().unwrap();
    send2.write_all(b"stream2").await.unwrap();
    send2.finish().unwrap();
    send3.write_all(b"stream3").await.unwrap();
    send3.finish().unwrap();

    // Receive and verify
    let data_a = recv_a.read_to_end(1024).await.unwrap();
    let data_b = recv_b.read_to_end(1024).await.unwrap();
    let data_c = recv_c.read_to_end(1024).await.unwrap();

    assert_eq!(data_a, b"stream1");
    assert_eq!(data_b, b"stream2");
    assert_eq!(data_c, b"stream3");
}

/// Test network partition simulation.
#[tokio::test]
async fn test_network_partition_behavior() {
    let network = MockIrohNetwork::new();

    let ep1 = network.create_endpoint();
    let ep2 = network.create_endpoint();

    // Normal connection works
    let conn = ep1.connect(ep2.id(), RAFT_ALPN).await;
    assert!(conn.is_ok());

    // Accept the connection
    let _ = ep2.accept().await.unwrap();

    // Enable network partition
    network.set_network_partition(true);

    // New connection should fail
    let result = ep1.connect(ep2.id(), RAFT_ALPN).await;
    assert!(result.is_err());

    // Disable partition
    network.set_network_partition(false);

    // New connection should succeed again
    let conn = ep1.connect(ep2.id(), RAFT_ALPN).await;
    assert!(conn.is_ok());
}

/// Test selective connection refusal.
#[tokio::test]
async fn test_selective_connection_refusal() {
    let network = MockIrohNetwork::new();

    let ep1 = network.create_endpoint();
    let ep2 = network.create_endpoint();
    let ep3 = network.create_endpoint();

    // Refuse connections to ep2 only
    network.refuse_connections_to(ep2.id());

    // Connection to ep2 should fail
    let result = ep1.connect(ep2.id(), RAFT_ALPN).await;
    assert!(result.is_err());

    // Connection to ep3 should succeed
    let result = ep1.connect(ep3.id(), RAFT_ALPN).await;
    assert!(result.is_ok());

    // Allow connections to ep2 again
    network.allow_connections_to(ep2.id());

    // Now connection to ep2 should succeed
    let result = ep1.connect(ep2.id(), RAFT_ALPN).await;
    assert!(result.is_ok());
}
