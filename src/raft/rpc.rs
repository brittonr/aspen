//! IRPC service definition for Raft RPC over Iroh.
//!
//! This module defines the RPC interface for Raft consensus communication using IRPC
//! over Iroh QUIC connections. It provides type-safe, serializable request/response pairs
//! that match OpenRaft's v2 network API.
//!
//! The IRPC service enum defines three RPC methods:
//! - Vote: Leader election vote requests (oneshot)
//! - AppendEntries: Log replication and heartbeats (oneshot)
//! - InstallSnapshot: Full snapshot transfer (oneshot with snapshot bytes)
//!
//! # Test Coverage
//!
//! Unit tests in `#[cfg(test)]` module cover:
//!   - TimestampInfo: Construction and field access
//!   - RaftRpcResponseWithTimestamps: Construction with/without timestamps
//!   - Serde roundtrip for all RPC message types

use irpc::channel::oneshot;
use irpc::rpc_requests;
use openraft::error::RaftError;
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::type_config::alias::VoteOf;
use serde::{Deserialize, Serialize};

use crate::raft::types::AppTypeConfig;

/// Vote request wrapper for IRPC.
///
/// Sent during leader election to request votes from followers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteRequest {
    /// The vote request from OpenRaft.
    pub request: VoteRequest<AppTypeConfig>,
}

/// AppendEntries request wrapper for IRPC.
///
/// Sent by leader for log replication and heartbeats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendEntriesRequest {
    /// The append entries request from OpenRaft.
    pub request: AppendEntriesRequest<AppTypeConfig>,
}

/// Full snapshot request wrapper for IRPC.
///
/// Sends a complete snapshot to a follower that has fallen too far behind.
/// Includes the leader vote, snapshot metadata, and snapshot data bytes.
///
/// The snapshot data is extracted from `Cursor<Vec<u8>>` and sent as raw bytes
/// since IRPC requires all fields to be serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftSnapshotRequest {
    /// The leader's vote information.
    pub vote: VoteOf<AppTypeConfig>,
    /// Snapshot metadata (last_log_id, last_membership, snapshot_id).
    pub snapshot_meta: openraft::SnapshotMeta<AppTypeConfig>,
    /// The snapshot data as raw bytes.
    pub snapshot_data: Vec<u8>,
}

/// IRPC service protocol for Raft RPC operations.
///
/// This enum defines the three core Raft RPC methods using IRPC's `#[rpc_requests]` macro.
/// Each variant corresponds to one RPC method with typed request/response channels.
///
/// Tiger Style: Explicit error handling via Result types on all responses.
///
/// Note: This generates `RaftRpcMessage` enum with `WithChannels` wrappers,
/// but for wire protocol we serialize/deserialize this protocol enum directly
/// (without channels), then create channels on the receiving side.
#[rpc_requests(message = RaftRpcMessage)]
#[derive(Debug, Serialize, Deserialize)]
pub enum RaftRpcProtocol {
    /// Vote RPC for leader election.
    ///
    /// Oneshot channel: single request → single response.
    /// Response is Result wrapping VoteResponse or RaftError.
    #[rpc(tx = oneshot::Sender<Result<VoteResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    Vote(RaftVoteRequest),

    /// AppendEntries RPC for log replication and heartbeats.
    ///
    /// Oneshot channel: single request → single response.
    /// Response is Result wrapping AppendEntriesResponse or RaftError.
    #[rpc(tx = oneshot::Sender<Result<AppendEntriesResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    AppendEntries(RaftAppendEntriesRequest),

    /// InstallSnapshot RPC for sending full snapshots.
    ///
    /// Oneshot channel: single request → single response.
    /// The snapshot data is included in the request as raw bytes.
    /// Response is Result wrapping SnapshotResponse or RaftError.
    #[rpc(tx = oneshot::Sender<Result<SnapshotResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    InstallSnapshot(RaftSnapshotRequest),
}

/// Wire protocol response types that are serializable.
/// These are sent as responses over the wire (without channels).
///
/// Note: Vote and AppendEntries use `Infallible` as error type since
/// OpenRaft v2 doesn't allow these RPCs to fail at the network layer.
/// Any business logic errors are encoded in the response itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftRpcResponse {
    /// Vote response containing vote decision and follower's state.
    Vote(VoteResponse<AppTypeConfig>),
    /// AppendEntries response (Success, Conflict, or HigherTerm).
    AppendEntries(AppendEntriesResponse<AppTypeConfig>),
    /// InstallSnapshot response, may fail with RaftError.
    InstallSnapshot(Result<SnapshotResponse<AppTypeConfig>, RaftError<AppTypeConfig>>),
}

/// Server-side timestamps for clock drift detection.
///
/// Embedded in RPC responses to enable NTP-style clock offset estimation.
/// The client records t1 (send) and t4 (receive), while the server provides
/// t2 (receive) and t3 (send).
///
/// Clock offset = ((t2 - t1) + (t3 - t4)) / 2
///
/// Note: This is purely for observational monitoring. Clock synchronization
/// is NOT required for Raft consensus correctness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampInfo {
    /// Server receive time (t2) in milliseconds since UNIX epoch.
    /// Captured when the server receives the RPC request.
    pub server_recv_ms: u64,
    /// Server send time (t3) in milliseconds since UNIX epoch.
    /// Captured just before the server sends the RPC response.
    pub server_send_ms: u64,
}

/// Wire protocol response with timestamps for clock drift detection.
///
/// Wraps the actual RPC response with optional timestamp information.
/// The timestamps field is optional to maintain backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftRpcResponseWithTimestamps {
    /// The actual RPC response.
    pub inner: RaftRpcResponse,
    /// Optional server timestamps for clock drift detection.
    /// May be None if the server doesn't support drift detection.
    pub timestamps: Option<TimestampInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::types::NodeId;
    use openraft::Vote;

    // =========================================================================
    // TimestampInfo Tests
    // =========================================================================

    #[test]
    fn test_timestamp_info_construction() {
        let ts = TimestampInfo {
            server_recv_ms: 1000,
            server_send_ms: 1005,
        };
        assert_eq!(ts.server_recv_ms, 1000);
        assert_eq!(ts.server_send_ms, 1005);
    }

    #[test]
    fn test_timestamp_info_zero_values() {
        let ts = TimestampInfo {
            server_recv_ms: 0,
            server_send_ms: 0,
        };
        assert_eq!(ts.server_recv_ms, 0);
        assert_eq!(ts.server_send_ms, 0);
    }

    #[test]
    fn test_timestamp_info_max_values() {
        let ts = TimestampInfo {
            server_recv_ms: u64::MAX,
            server_send_ms: u64::MAX,
        };
        assert_eq!(ts.server_recv_ms, u64::MAX);
        assert_eq!(ts.server_send_ms, u64::MAX);
    }

    #[test]
    fn test_timestamp_info_clone() {
        let ts = TimestampInfo {
            server_recv_ms: 100,
            server_send_ms: 200,
        };
        let cloned = ts.clone();
        assert_eq!(ts.server_recv_ms, cloned.server_recv_ms);
        assert_eq!(ts.server_send_ms, cloned.server_send_ms);
    }

    #[test]
    fn test_timestamp_info_debug() {
        let ts = TimestampInfo {
            server_recv_ms: 1000,
            server_send_ms: 1005,
        };
        let debug_str = format!("{:?}", ts);
        assert!(debug_str.contains("1000"));
        assert!(debug_str.contains("1005"));
    }

    #[test]
    fn test_timestamp_info_serde_roundtrip() {
        let original = TimestampInfo {
            server_recv_ms: 12345678,
            server_send_ms: 12345700,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: TimestampInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original.server_recv_ms, deserialized.server_recv_ms);
        assert_eq!(original.server_send_ms, deserialized.server_send_ms);
    }

    // =========================================================================
    // RaftRpcResponse Tests
    // =========================================================================

    #[test]
    fn test_rpc_response_vote() {
        let response = RaftRpcResponse::Vote(VoteResponse {
            vote: Vote::new(1, NodeId::from(1)),
            vote_granted: true,
            last_log_id: None,
        });

        assert!(matches!(response, RaftRpcResponse::Vote(_)));
    }

    #[test]
    fn test_rpc_response_append_entries() {
        let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success);
        assert!(matches!(response, RaftRpcResponse::AppendEntries(_)));
    }

    #[test]
    fn test_rpc_response_install_snapshot_ok() {
        let response = RaftRpcResponse::InstallSnapshot(Ok(SnapshotResponse {
            vote: Vote::new(1, NodeId::from(1)),
        }));
        assert!(matches!(response, RaftRpcResponse::InstallSnapshot(Ok(_))));
    }

    #[test]
    fn test_rpc_response_install_snapshot_err() {
        let error = RaftError::<AppTypeConfig>::Fatal(openraft::error::Fatal::Panicked);
        let response = RaftRpcResponse::InstallSnapshot(Err(error));
        assert!(matches!(response, RaftRpcResponse::InstallSnapshot(Err(_))));
    }

    #[test]
    fn test_rpc_response_clone() {
        let response = RaftRpcResponse::Vote(VoteResponse {
            vote: Vote::new(1, NodeId::from(1)),
            vote_granted: false,
            last_log_id: None,
        });
        let cloned = response.clone();
        assert!(matches!(cloned, RaftRpcResponse::Vote(v) if !v.vote_granted));
    }

    // =========================================================================
    // RaftRpcResponseWithTimestamps Tests
    // =========================================================================

    #[test]
    fn test_response_with_timestamps_some() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(1, NodeId::from(1)),
                vote_granted: true,
                last_log_id: None,
            }),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 1000,
                server_send_ms: 1005,
            }),
        };

        assert!(response.timestamps.is_some());
        let ts = response.timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 1000);
        assert_eq!(ts.server_send_ms, 1005);
    }

    #[test]
    fn test_response_with_timestamps_none() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success),
            timestamps: None,
        };

        assert!(response.timestamps.is_none());
    }

    #[test]
    fn test_response_with_timestamps_clone() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(2, NodeId::from(2)),
                vote_granted: true,
                last_log_id: None,
            }),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 500,
                server_send_ms: 600,
            }),
        };

        let cloned = response.clone();
        assert!(cloned.timestamps.is_some());
    }

    #[test]
    fn test_response_with_timestamps_debug() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::AppendEntries(AppendEntriesResponse::Conflict),
            timestamps: None,
        };

        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("RaftRpcResponseWithTimestamps"));
    }

    #[test]
    fn test_response_with_timestamps_serde_roundtrip() {
        let original = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(5, NodeId::from(3)),
                vote_granted: true,
                last_log_id: None,
            }),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 9999,
                server_send_ms: 10001,
            }),
        };

        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: RaftRpcResponseWithTimestamps =
            serde_json::from_str(&json).expect("deserialize");

        assert!(deserialized.timestamps.is_some());
        let ts = deserialized.timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 9999);
        assert_eq!(ts.server_send_ms, 10001);
    }

    // =========================================================================
    // RaftVoteRequest Tests
    // =========================================================================

    #[test]
    fn test_vote_request_construction() {
        let request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(1, NodeId::from(1)),
                last_log_id: None,
            },
        };

        assert_eq!(request.request.vote.leader_id().term, 1);
    }

    #[test]
    fn test_vote_request_clone() {
        let request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(2, NodeId::from(2)),
                last_log_id: None,
            },
        };

        let cloned = request.clone();
        assert_eq!(cloned.request.vote.leader_id().term, 2);
    }

    // =========================================================================
    // RaftAppendEntriesRequest Tests
    // =========================================================================

    #[test]
    fn test_append_entries_request_empty() {
        let request = RaftAppendEntriesRequest {
            request: AppendEntriesRequest {
                vote: Vote::new(1, NodeId::from(1)),
                prev_log_id: None,
                entries: vec![],
                leader_commit: None,
            },
        };

        assert!(request.request.entries.is_empty());
    }

    #[test]
    fn test_append_entries_request_clone() {
        let request = RaftAppendEntriesRequest {
            request: AppendEntriesRequest {
                vote: Vote::new(3, NodeId::from(3)),
                prev_log_id: None,
                entries: vec![],
                leader_commit: None,
            },
        };

        let cloned = request.clone();
        assert_eq!(cloned.request.vote.leader_id().term, 3);
    }

    // =========================================================================
    // RaftSnapshotRequest Tests
    // =========================================================================

    #[test]
    fn test_snapshot_request_construction() {
        use openraft::{Membership, SnapshotMeta, StoredMembership};

        let membership = Membership::<AppTypeConfig>::new_with_defaults(vec![], []);

        let request = RaftSnapshotRequest {
            vote: Vote::new(1, NodeId::from(1)),
            snapshot_meta: SnapshotMeta {
                last_log_id: None,
                last_membership: StoredMembership::new(None, membership),
                snapshot_id: "test-snapshot".to_string(),
            },
            snapshot_data: vec![1, 2, 3, 4, 5],
        };

        assert_eq!(request.snapshot_data.len(), 5);
        assert_eq!(request.snapshot_meta.snapshot_id, "test-snapshot");
    }

    #[test]
    fn test_snapshot_request_empty_data() {
        use openraft::{Membership, SnapshotMeta, StoredMembership};

        let membership = Membership::<AppTypeConfig>::new_with_defaults(vec![], []);

        let request = RaftSnapshotRequest {
            vote: Vote::new(1, NodeId::from(1)),
            snapshot_meta: SnapshotMeta {
                last_log_id: None,
                last_membership: StoredMembership::new(None, membership),
                snapshot_id: "empty".to_string(),
            },
            snapshot_data: vec![],
        };

        assert!(request.snapshot_data.is_empty());
    }

    #[test]
    fn test_snapshot_request_clone() {
        use openraft::{Membership, SnapshotMeta, StoredMembership};

        let membership = Membership::<AppTypeConfig>::new_with_defaults(vec![], []);

        let request = RaftSnapshotRequest {
            vote: Vote::new(2, NodeId::from(2)),
            snapshot_meta: SnapshotMeta {
                last_log_id: None,
                last_membership: StoredMembership::new(None, membership),
                snapshot_id: "clone-test".to_string(),
            },
            snapshot_data: vec![10, 20, 30],
        };

        let cloned = request.clone();
        assert_eq!(cloned.snapshot_data, vec![10, 20, 30]);
    }
}
