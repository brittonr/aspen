//! Tests for Raft RPC server message handling.
//!
//! This module tests the RPC request/response serialization and handling logic
//! used by the RaftRpcServer without requiring full Iroh endpoints or a Raft core.
//!
//! # Coverage Goals
//!
//! - RPC message serialization/deserialization (postcard format)
//! - Response timestamp wrapping (clock drift detection)
//! - Size limit enforcement (MAX_RPC_MESSAGE_SIZE)
//! - Mock stream RPC flow testing
//!
//! # Tiger Style
//!
//! - Explicit bounds verification matching constants.rs
//! - Deterministic tests with no real I/O
//! - Comprehensive error path coverage

mod support;

use aspen::raft::constants::MAX_RPC_MESSAGE_SIZE;
use aspen::raft::rpc::{
    RaftAppendEntriesRequest, RaftRpcProtocol, RaftRpcResponse, RaftRpcResponseWithTimestamps,
    RaftSnapshotRequest, RaftVoteRequest, TimestampInfo,
};
use aspen::raft::types::{AppRequest, AppTypeConfig, NodeId};
use openraft::error::RaftError;
use openraft::raft::{AppendEntriesResponse, SnapshotResponse, VoteResponse};
use openraft::testing::log_id;
use openraft::{Membership, SnapshotMeta, Vote};
use support::mock_iroh::{MockIrohNetwork, RAFT_ALPN};

// ============================================================================
// Phase 1: RPC Request Serialization Tests
// ============================================================================

/// Test that Vote request serializes and deserializes correctly.
#[tokio::test]
async fn test_vote_request_serialization() {
    let vote_request = RaftVoteRequest {
        request: openraft::raft::VoteRequest {
            vote: Vote::new(5, NodeId::from(1)),
            last_log_id: Some(log_id::<AppTypeConfig>(3, NodeId::from(1), 42)),
        },
    };

    let rpc = RaftRpcProtocol::Vote(vote_request);

    // Serialize
    let bytes = postcard::to_stdvec(&rpc).expect("failed to serialize vote request");

    // Verify size is within limits
    assert!(
        bytes.len() <= MAX_RPC_MESSAGE_SIZE as usize,
        "serialized vote request {} bytes exceeds MAX_RPC_MESSAGE_SIZE {} bytes",
        bytes.len(),
        MAX_RPC_MESSAGE_SIZE
    );

    // Deserialize
    let deserialized: RaftRpcProtocol =
        postcard::from_bytes(&bytes).expect("failed to deserialize vote request");

    match deserialized {
        RaftRpcProtocol::Vote(req) => {
            assert_eq!(req.request.vote.leader_id().node_id, NodeId::from(1));
            assert_eq!(req.request.vote.leader_id().term, 5);
            assert!(req.request.last_log_id.is_some());
        }
        _ => panic!("expected Vote variant, got {:?}", deserialized),
    }
}

/// Test Vote request with no last_log_id (fresh node).
#[tokio::test]
async fn test_vote_request_empty_log() {
    let vote_request = RaftVoteRequest {
        request: openraft::raft::VoteRequest {
            vote: Vote::new(1, NodeId::from(42)),
            last_log_id: None,
        },
    };

    let rpc = RaftRpcProtocol::Vote(vote_request);
    let bytes = postcard::to_stdvec(&rpc).expect("serialize");
    let deserialized: RaftRpcProtocol = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcProtocol::Vote(req) => {
            assert_eq!(req.request.vote.leader_id().node_id, NodeId::from(42));
            assert!(req.request.last_log_id.is_none());
        }
        _ => panic!("expected Vote variant"),
    }
}

/// Test AppendEntries request serialization with log entries.
#[tokio::test]
async fn test_append_entries_request_serialization() {
    use openraft::Entry;
    use openraft::EntryPayload;

    let entry = Entry::<AppTypeConfig> {
        log_id: log_id::<AppTypeConfig>(2, NodeId::from(1), 10),
        payload: EntryPayload::Normal(AppRequest::Set {
            key: "test-key".to_string(),
            value: "test-value".to_string(),
        }),
    };

    let append_request = RaftAppendEntriesRequest {
        request: openraft::raft::AppendEntriesRequest {
            vote: Vote::new(2, NodeId::from(1)),
            prev_log_id: Some(log_id::<AppTypeConfig>(2, NodeId::from(1), 9)),
            entries: vec![entry],
            leader_commit: Some(log_id::<AppTypeConfig>(2, NodeId::from(1), 8)),
        },
    };

    let rpc = RaftRpcProtocol::AppendEntries(append_request);
    let bytes = postcard::to_stdvec(&rpc).expect("serialize");

    assert!(
        bytes.len() <= MAX_RPC_MESSAGE_SIZE as usize,
        "append entries request exceeds size limit"
    );

    let deserialized: RaftRpcProtocol = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcProtocol::AppendEntries(req) => {
            assert_eq!(req.request.vote.leader_id().term, 2);
            assert_eq!(req.request.entries.len(), 1);
            assert!(req.request.prev_log_id.is_some());
        }
        _ => panic!("expected AppendEntries variant"),
    }
}

/// Test AppendEntries heartbeat (empty entries).
#[tokio::test]
async fn test_append_entries_heartbeat_serialization() {
    let heartbeat = RaftAppendEntriesRequest {
        request: openraft::raft::AppendEntriesRequest {
            vote: Vote::new(3, NodeId::from(1)),
            prev_log_id: Some(log_id::<AppTypeConfig>(3, NodeId::from(1), 100)),
            entries: vec![],
            leader_commit: Some(log_id::<AppTypeConfig>(3, NodeId::from(1), 99)),
        },
    };

    let rpc = RaftRpcProtocol::AppendEntries(heartbeat);
    let bytes = postcard::to_stdvec(&rpc).expect("serialize");

    // Heartbeats should be small
    assert!(
        bytes.len() < 200,
        "heartbeat should be small: {} bytes",
        bytes.len()
    );

    let deserialized: RaftRpcProtocol = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcProtocol::AppendEntries(req) => {
            assert!(
                req.request.entries.is_empty(),
                "heartbeat should have no entries"
            );
        }
        _ => panic!("expected AppendEntries variant"),
    }
}

/// Test InstallSnapshot request serialization.
#[tokio::test]
async fn test_install_snapshot_request_serialization() {
    // Create a realistic snapshot with some data
    let snapshot_data = b"snapshot-content-key1=value1,key2=value2".to_vec();

    // Create membership for snapshot meta using new_with_defaults
    let membership = Membership::<AppTypeConfig>::new_with_defaults(
        vec![btreeset! {NodeId::from(1), NodeId::from(2)}],
        [],
    );

    let snapshot_request = RaftSnapshotRequest {
        vote: Vote::new(5, NodeId::from(1)),
        snapshot_meta: SnapshotMeta {
            last_log_id: Some(log_id::<AppTypeConfig>(5, NodeId::from(1), 500)),
            last_membership: openraft::StoredMembership::new(
                Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 1)),
                membership,
            ),
            snapshot_id: "test-snapshot-001".to_string(),
        },
        snapshot_data,
    };

    let rpc = RaftRpcProtocol::InstallSnapshot(snapshot_request.clone());
    let bytes = postcard::to_stdvec(&rpc).expect("serialize");

    assert!(
        bytes.len() <= MAX_RPC_MESSAGE_SIZE as usize,
        "snapshot request exceeds size limit"
    );

    let deserialized: RaftRpcProtocol = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcProtocol::InstallSnapshot(req) => {
            assert_eq!(req.vote.leader_id().term, 5);
            assert_eq!(req.snapshot_meta.snapshot_id, "test-snapshot-001");
            assert_eq!(
                req.snapshot_data,
                b"snapshot-content-key1=value1,key2=value2".to_vec()
            );
        }
        _ => panic!("expected InstallSnapshot variant"),
    }
}

/// Test snapshot with large data near (but under) the limit.
#[tokio::test]
async fn test_large_snapshot_serialization() {
    // Create 1MB of snapshot data
    let snapshot_data = vec![0xABu8; 1024 * 1024];

    let membership =
        Membership::<AppTypeConfig>::new_with_defaults(vec![btreeset! {NodeId::from(1)}], []);

    let snapshot_request = RaftSnapshotRequest {
        vote: Vote::new(1, NodeId::from(1)),
        snapshot_meta: SnapshotMeta {
            last_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 1000)),
            last_membership: openraft::StoredMembership::new(
                Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 1)),
                membership,
            ),
            snapshot_id: "large-snapshot".to_string(),
        },
        snapshot_data: snapshot_data.clone(),
    };

    let rpc = RaftRpcProtocol::InstallSnapshot(snapshot_request);
    let bytes = postcard::to_stdvec(&rpc).expect("serialize large snapshot");

    // Should be slightly larger than 1MB due to metadata
    assert!(bytes.len() > 1024 * 1024);
    assert!(
        bytes.len() <= MAX_RPC_MESSAGE_SIZE as usize,
        "1MB snapshot should fit in 10MB limit"
    );

    let deserialized: RaftRpcProtocol = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcProtocol::InstallSnapshot(req) => {
            assert_eq!(req.snapshot_data.len(), 1024 * 1024);
        }
        _ => panic!("expected InstallSnapshot variant"),
    }
}

// ============================================================================
// Phase 1: RPC Response Serialization Tests
// ============================================================================

/// Test VoteResponse serialization.
#[tokio::test]
async fn test_vote_response_serialization() {
    let response = RaftRpcResponse::Vote(VoteResponse {
        vote: Vote::new(5, NodeId::from(2)),
        vote_granted: true,
        last_log_id: Some(log_id::<AppTypeConfig>(4, NodeId::from(2), 100)),
    });

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::Vote(resp) => {
            assert!(resp.vote_granted);
            assert_eq!(resp.vote.leader_id().term, 5);
        }
        _ => panic!("expected Vote response"),
    }
}

/// Test VoteResponse when vote is not granted.
#[tokio::test]
async fn test_vote_response_not_granted() {
    let response = RaftRpcResponse::Vote(VoteResponse {
        vote: Vote::new(10, NodeId::from(3)),
        vote_granted: false,
        last_log_id: None,
    });

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::Vote(resp) => {
            assert!(!resp.vote_granted);
        }
        _ => panic!("expected Vote response"),
    }
}

/// Test AppendEntriesResponse::Success serialization.
#[tokio::test]
async fn test_append_entries_response_success() {
    let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success);

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::AppendEntries(resp) => {
            assert!(resp.is_success());
        }
        _ => panic!("expected AppendEntries response"),
    }
}

/// Test AppendEntriesResponse::Conflict serialization.
#[tokio::test]
async fn test_append_entries_response_conflict() {
    let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Conflict);

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::AppendEntries(resp) => {
            assert!(resp.is_conflict());
        }
        _ => panic!("expected AppendEntries response"),
    }
}

/// Test AppendEntriesResponse::HigherVote serialization.
#[tokio::test]
async fn test_append_entries_response_higher_vote() {
    let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::HigherVote(Vote::new(
        10,
        NodeId::from(5),
    )));

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::AppendEntries(AppendEntriesResponse::HigherVote(vote)) => {
            assert_eq!(vote.leader_id().term, 10);
            assert_eq!(vote.leader_id().node_id, NodeId::from(5));
        }
        _ => panic!("expected AppendEntries::HigherVote response"),
    }
}

/// Test AppendEntriesResponse::PartialSuccess serialization.
#[tokio::test]
async fn test_append_entries_response_partial_success() {
    let response =
        RaftRpcResponse::AppendEntries(AppendEntriesResponse::PartialSuccess(Some(log_id::<
            AppTypeConfig,
        >(
            3,
            NodeId::from(1),
            50,
        ))));

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::AppendEntries(AppendEntriesResponse::PartialSuccess(log_id)) => {
            assert!(log_id.is_some());
        }
        _ => panic!("expected AppendEntries::PartialSuccess response"),
    }
}

/// Test InstallSnapshotResponse serialization (success case).
#[tokio::test]
async fn test_install_snapshot_response_success() {
    let response = RaftRpcResponse::InstallSnapshot(Ok(SnapshotResponse {
        vote: Vote::new(5, NodeId::from(1)),
    }));

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::InstallSnapshot(Ok(resp)) => {
            assert_eq!(resp.vote.leader_id().term, 5);
        }
        _ => panic!("expected successful InstallSnapshot response"),
    }
}

/// Test InstallSnapshotResponse serialization (error case).
#[tokio::test]
async fn test_install_snapshot_response_error() {
    // Create a fatal error using the Fatal variant directly
    let error = RaftError::<AppTypeConfig>::Fatal(openraft::error::Fatal::Panicked);
    let response = RaftRpcResponse::InstallSnapshot(Err(error));

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

    match deserialized {
        RaftRpcResponse::InstallSnapshot(Err(_)) => {
            // Successfully roundtripped the error
        }
        _ => panic!("expected error InstallSnapshot response"),
    }
}

// ============================================================================
// Phase 1: Timestamp Wrapper Tests
// ============================================================================

/// Test RaftRpcResponseWithTimestamps serialization.
#[tokio::test]
async fn test_response_with_timestamps() {
    let inner = RaftRpcResponse::Vote(VoteResponse {
        vote: Vote::new(1, NodeId::from(1)),
        vote_granted: true,
        last_log_id: None,
    });

    let response = RaftRpcResponseWithTimestamps {
        inner,
        timestamps: Some(TimestampInfo {
            server_recv_ms: 1702000000000, // Example: Dec 2023 timestamp
            server_send_ms: 1702000000005, // 5ms later
        }),
    };

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponseWithTimestamps =
        postcard::from_bytes(&bytes).expect("deserialize");

    assert!(deserialized.timestamps.is_some());
    let ts = deserialized.timestamps.unwrap();
    assert_eq!(ts.server_recv_ms, 1702000000000);
    assert_eq!(ts.server_send_ms, 1702000000005);

    match deserialized.inner {
        RaftRpcResponse::Vote(resp) => assert!(resp.vote_granted),
        _ => panic!("expected Vote response"),
    }
}

/// Test response with None timestamps (backwards compatibility).
#[tokio::test]
async fn test_response_without_timestamps() {
    let inner = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success);

    let response = RaftRpcResponseWithTimestamps {
        inner,
        timestamps: None,
    };

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponseWithTimestamps =
        postcard::from_bytes(&bytes).expect("deserialize");

    assert!(deserialized.timestamps.is_none());
}

/// Test timestamp wraparound handling (large values).
#[tokio::test]
async fn test_timestamp_large_values() {
    let response = RaftRpcResponseWithTimestamps {
        inner: RaftRpcResponse::Vote(VoteResponse {
            vote: Vote::new(1, NodeId::from(1)),
            vote_granted: true,
            last_log_id: None,
        }),
        timestamps: Some(TimestampInfo {
            server_recv_ms: u64::MAX - 1000,
            server_send_ms: u64::MAX,
        }),
    };

    let bytes = postcard::to_stdvec(&response).expect("serialize");
    let deserialized: RaftRpcResponseWithTimestamps =
        postcard::from_bytes(&bytes).expect("deserialize");

    let ts = deserialized.timestamps.unwrap();
    assert_eq!(ts.server_recv_ms, u64::MAX - 1000);
    assert_eq!(ts.server_send_ms, u64::MAX);
}

// ============================================================================
// Phase 1: Size Limit and Error Tests
// ============================================================================

/// Test that malformed bytes fail to deserialize with clear error.
#[tokio::test]
async fn test_malformed_message_rejected() {
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB];
    let result: Result<RaftRpcProtocol, _> = postcard::from_bytes(&garbage);

    assert!(result.is_err(), "garbage bytes should fail to deserialize");
}

/// Test that truncated messages fail to deserialize.
#[tokio::test]
async fn test_truncated_message_rejected() {
    let vote_request = RaftVoteRequest {
        request: openraft::raft::VoteRequest {
            vote: Vote::new(1, NodeId::from(1)),
            last_log_id: None,
        },
    };

    let rpc = RaftRpcProtocol::Vote(vote_request);
    let bytes = postcard::to_stdvec(&rpc).expect("serialize");

    // Truncate the message
    let truncated = &bytes[..bytes.len() / 2];
    let result: Result<RaftRpcProtocol, _> = postcard::from_bytes(truncated);

    assert!(
        result.is_err(),
        "truncated message should fail to deserialize"
    );
}

/// Test empty message rejection.
#[tokio::test]
async fn test_empty_message_rejected() {
    let empty: &[u8] = &[];
    let result: Result<RaftRpcProtocol, _> = postcard::from_bytes(empty);

    assert!(result.is_err(), "empty message should fail to deserialize");
}

/// Verify MAX_RPC_MESSAGE_SIZE constant matches expectations.
#[test]
fn test_max_rpc_message_size_constant() {
    // 10 MB limit
    assert_eq!(MAX_RPC_MESSAGE_SIZE, 10 * 1024 * 1024);
}

// ============================================================================
// Phase 2: Mock Stream RPC Flow Tests
// ============================================================================

/// Test full Vote RPC flow using mock Iroh network.
#[tokio::test]
async fn test_mock_vote_rpc_flow() {
    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Client connects to server
    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    // Client sends Vote request
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

    let vote_request = RaftVoteRequest {
        request: openraft::raft::VoteRequest {
            vote: Vote::new(3, NodeId::from(1)),
            last_log_id: Some(log_id::<AppTypeConfig>(2, NodeId::from(1), 50)),
        },
    };

    let request = RaftRpcProtocol::Vote(vote_request);
    let request_bytes = postcard::to_stdvec(&request).unwrap();
    send.write_all(&request_bytes).await.unwrap();
    send.finish().unwrap();

    // Server receives and processes request
    let (mut server_send, mut server_recv) = server_conn.accept_bi().await.unwrap();
    let received_bytes = server_recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .unwrap();

    let received: RaftRpcProtocol = postcard::from_bytes(&received_bytes).unwrap();
    assert!(matches!(received, RaftRpcProtocol::Vote(_)));

    // Server sends response
    let response = RaftRpcResponseWithTimestamps {
        inner: RaftRpcResponse::Vote(VoteResponse {
            vote: Vote::new(3, NodeId::from(1)),
            vote_granted: true,
            last_log_id: Some(log_id::<AppTypeConfig>(2, NodeId::from(2), 45)),
        }),
        timestamps: Some(TimestampInfo {
            server_recv_ms: 1000,
            server_send_ms: 1001,
        }),
    };

    let response_bytes = postcard::to_stdvec(&response).unwrap();
    server_send.write_all(&response_bytes).await.unwrap();
    server_send.finish().unwrap();

    // Client receives response
    let response_data = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .unwrap();
    let response: RaftRpcResponseWithTimestamps = postcard::from_bytes(&response_data).unwrap();

    match response.inner {
        RaftRpcResponse::Vote(resp) => {
            assert!(resp.vote_granted);
        }
        _ => panic!("expected Vote response"),
    }
    assert!(response.timestamps.is_some());
}

/// Test full AppendEntries RPC flow using mock Iroh network.
#[tokio::test]
async fn test_mock_append_entries_rpc_flow() {
    use openraft::Entry;
    use openraft::EntryPayload;

    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    // Client sends AppendEntries request
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

    let entry = Entry::<AppTypeConfig> {
        log_id: log_id::<AppTypeConfig>(1, NodeId::from(1), 5),
        payload: EntryPayload::Normal(AppRequest::Set {
            key: "foo".to_string(),
            value: "bar".to_string(),
        }),
    };

    let append_request = RaftAppendEntriesRequest {
        request: openraft::raft::AppendEntriesRequest {
            vote: Vote::new(1, NodeId::from(1)),
            prev_log_id: Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 4)),
            entries: vec![entry],
            leader_commit: Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 4)),
        },
    };

    let request = RaftRpcProtocol::AppendEntries(append_request);
    let request_bytes = postcard::to_stdvec(&request).unwrap();
    send.write_all(&request_bytes).await.unwrap();
    send.finish().unwrap();

    // Server receives
    let (mut server_send, mut server_recv) = server_conn.accept_bi().await.unwrap();
    let received_bytes = server_recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .unwrap();

    let received: RaftRpcProtocol = postcard::from_bytes(&received_bytes).unwrap();
    match received {
        RaftRpcProtocol::AppendEntries(req) => {
            assert_eq!(req.request.entries.len(), 1);
        }
        _ => panic!("expected AppendEntries"),
    }

    // Server sends Success response
    let response = RaftRpcResponseWithTimestamps {
        inner: RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success),
        timestamps: Some(TimestampInfo {
            server_recv_ms: 2000,
            server_send_ms: 2002,
        }),
    };

    let response_bytes = postcard::to_stdvec(&response).unwrap();
    server_send.write_all(&response_bytes).await.unwrap();
    server_send.finish().unwrap();

    // Client receives
    let response_data = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .unwrap();
    let response: RaftRpcResponseWithTimestamps = postcard::from_bytes(&response_data).unwrap();

    match response.inner {
        RaftRpcResponse::AppendEntries(resp) => {
            assert!(resp.is_success());
        }
        _ => panic!("expected AppendEntries response"),
    }
}

/// Test InstallSnapshot RPC flow with mock network.
#[tokio::test]
async fn test_mock_install_snapshot_rpc_flow() {
    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

    // Create snapshot request with realistic data
    let snapshot_data = b"key1=value1\nkey2=value2\nkey3=value3".to_vec();

    let membership =
        Membership::<AppTypeConfig>::new_with_defaults(vec![btreeset! {NodeId::from(1)}], []);

    let snapshot_request = RaftSnapshotRequest {
        vote: Vote::new(2, NodeId::from(1)),
        snapshot_meta: SnapshotMeta {
            last_log_id: Some(log_id::<AppTypeConfig>(2, NodeId::from(1), 100)),
            last_membership: openraft::StoredMembership::new(
                Some(log_id::<AppTypeConfig>(1, NodeId::from(1), 1)),
                membership,
            ),
            snapshot_id: "snap-001".to_string(),
        },
        snapshot_data: snapshot_data.clone(),
    };

    let request = RaftRpcProtocol::InstallSnapshot(snapshot_request);
    let request_bytes = postcard::to_stdvec(&request).unwrap();
    send.write_all(&request_bytes).await.unwrap();
    send.finish().unwrap();

    // Server receives
    let (mut server_send, mut server_recv) = server_conn.accept_bi().await.unwrap();
    let received_bytes = server_recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .unwrap();

    let received: RaftRpcProtocol = postcard::from_bytes(&received_bytes).unwrap();
    match received {
        RaftRpcProtocol::InstallSnapshot(req) => {
            assert_eq!(req.snapshot_data, snapshot_data);
            assert_eq!(req.snapshot_meta.snapshot_id, "snap-001");
        }
        _ => panic!("expected InstallSnapshot"),
    }

    // Server sends success response
    let response = RaftRpcResponseWithTimestamps {
        inner: RaftRpcResponse::InstallSnapshot(Ok(SnapshotResponse {
            vote: Vote::new(2, NodeId::from(1)),
        })),
        timestamps: Some(TimestampInfo {
            server_recv_ms: 3000,
            server_send_ms: 3050, // Snapshot processing takes longer
        }),
    };

    let response_bytes = postcard::to_stdvec(&response).unwrap();
    server_send.write_all(&response_bytes).await.unwrap();
    server_send.finish().unwrap();

    // Client receives
    let response_data = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .unwrap();
    let response: RaftRpcResponseWithTimestamps = postcard::from_bytes(&response_data).unwrap();

    match response.inner {
        RaftRpcResponse::InstallSnapshot(Ok(_)) => {
            // Success
        }
        _ => panic!("expected successful InstallSnapshot response"),
    }
}

/// Test concurrent RPC streams on a single connection.
#[tokio::test]
async fn test_concurrent_rpc_streams() {
    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();
    let incoming = server_ep.accept().await.unwrap();
    let server_conn = incoming.accept();

    // Open 3 streams concurrently
    let (mut send1, mut recv1) = client_conn.open_bi().await.unwrap();
    let (mut send2, mut recv2) = client_conn.open_bi().await.unwrap();
    let (mut send3, mut recv3) = client_conn.open_bi().await.unwrap();

    // Send different Vote requests on each stream
    for (i, send) in [&mut send1, &mut send2, &mut send3].iter_mut().enumerate() {
        let vote_request = RaftVoteRequest {
            request: openraft::raft::VoteRequest {
                vote: Vote::new((i + 1) as u64, NodeId::from(1)),
                last_log_id: None,
            },
        };
        let request = RaftRpcProtocol::Vote(vote_request);
        let bytes = postcard::to_stdvec(&request).unwrap();
        send.write_all(&bytes).await.unwrap();
        send.finish().unwrap();
    }

    // Server accepts all 3 streams
    let mut terms_received = vec![];
    for _ in 0..3 {
        let (mut server_send, mut server_recv) = server_conn.accept_bi().await.unwrap();
        let bytes = server_recv
            .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
            .await
            .unwrap();
        let received: RaftRpcProtocol = postcard::from_bytes(&bytes).unwrap();

        match received {
            RaftRpcProtocol::Vote(req) => {
                terms_received.push(req.request.vote.leader_id().term);
            }
            _ => panic!("expected Vote"),
        }

        // Send response
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(1, NodeId::from(1)),
                vote_granted: true,
                last_log_id: None,
            }),
            timestamps: None,
        };
        let response_bytes = postcard::to_stdvec(&response).unwrap();
        server_send.write_all(&response_bytes).await.unwrap();
        server_send.finish().unwrap();
    }

    // Verify all 3 terms were received (order may vary)
    terms_received.sort();
    assert_eq!(terms_received, vec![1, 2, 3]);

    // Client receives all 3 responses
    for recv in [&mut recv1, &mut recv2, &mut recv3] {
        let bytes = recv
            .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
            .await
            .unwrap();
        let response: RaftRpcResponseWithTimestamps = postcard::from_bytes(&bytes).unwrap();
        assert!(matches!(response.inner, RaftRpcResponse::Vote(_)));
    }
}

/// Test RPC with network partition (connection refused).
#[tokio::test]
async fn test_rpc_network_partition() {
    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    // Simulate network partition
    network.set_network_partition(true);

    // Connection should fail
    let result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;
    assert!(result.is_err(), "connection should fail during partition");

    // Disable partition
    network.set_network_partition(false);

    // Now connection should succeed
    let result = client_ep.connect(server_ep.id(), RAFT_ALPN).await;
    assert!(
        result.is_ok(),
        "connection should succeed after partition heals"
    );
}

/// Test handling of connection close during RPC.
#[tokio::test]
async fn test_rpc_connection_closed() {
    let network = MockIrohNetwork::new();

    let client_ep = network.create_endpoint();
    let server_ep = network.create_endpoint();

    let client_conn = client_ep.connect(server_ep.id(), RAFT_ALPN).await.unwrap();

    // Close connection before opening stream
    client_conn.close();

    // Opening stream should fail
    let result = client_conn.open_bi().await;
    assert!(
        result.is_err(),
        "opening stream on closed connection should fail"
    );
}

// Convenience macro for btreeset (used in Membership tests)
macro_rules! btreeset {
    () => { std::collections::BTreeSet::new() };
    ( $($x:expr),* $(,)? ) => {{
        let mut set = std::collections::BTreeSet::new();
        $(set.insert($x);)*
        set
    }};
}
use btreeset;
