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

mod errors;
mod messages;
mod sharding;

pub use errors::*;
pub use messages::*;
pub use sharding::*;

#[cfg(test)]
mod tests {
    use openraft::Vote;
    use openraft::error::RaftError;
    use openraft::raft::AppendEntriesRequest;
    use openraft::raft::AppendEntriesResponse;
    use openraft::raft::SnapshotResponse;
    use openraft::raft::VoteRequest;
    use openraft::raft::VoteResponse;

    use super::*;
    use crate::types::AppTypeConfig;

    type NodeId = aspen_raft_types::NodeId;

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
        let deserialized: RaftRpcResponseWithTimestamps = serde_json::from_str(&json).expect("deserialize");

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
        use openraft::Membership;
        use openraft::SnapshotMeta;
        use openraft::StoredMembership;

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
        use openraft::Membership;
        use openraft::SnapshotMeta;
        use openraft::StoredMembership;

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
        use openraft::Membership;
        use openraft::SnapshotMeta;
        use openraft::StoredMembership;

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

    // =========================================================================
    // RaftFatalErrorKind Tests
    // =========================================================================

    #[test]
    fn test_fatal_error_kind_panicked() {
        let kind = RaftFatalErrorKind::Panicked;
        assert_eq!(kind, RaftFatalErrorKind::Panicked);
        assert_eq!(format!("{}", kind), "RaftCore panicked");
    }

    #[test]
    fn test_fatal_error_kind_stopped() {
        let kind = RaftFatalErrorKind::Stopped;
        assert_eq!(kind, RaftFatalErrorKind::Stopped);
        assert_eq!(format!("{}", kind), "RaftCore stopped");
    }

    #[test]
    fn test_fatal_error_kind_storage_error() {
        let kind = RaftFatalErrorKind::StorageError;
        assert_eq!(kind, RaftFatalErrorKind::StorageError);
        assert_eq!(format!("{}", kind), "storage error");
    }

    #[test]
    fn test_fatal_error_kind_from_fatal_panicked() {
        let fatal = openraft::error::Fatal::<AppTypeConfig>::Panicked;
        let kind = RaftFatalErrorKind::from_fatal(&fatal);
        assert_eq!(kind, RaftFatalErrorKind::Panicked);
    }

    #[test]
    fn test_fatal_error_kind_from_fatal_stopped() {
        let fatal = openraft::error::Fatal::<AppTypeConfig>::Stopped;
        let kind = RaftFatalErrorKind::from_fatal(&fatal);
        assert_eq!(kind, RaftFatalErrorKind::Stopped);
    }

    #[test]
    fn test_fatal_error_kind_clone() {
        let kind = RaftFatalErrorKind::Panicked;
        let cloned = kind;
        assert_eq!(cloned, RaftFatalErrorKind::Panicked);
    }

    #[test]
    fn test_fatal_error_kind_copy() {
        let kind = RaftFatalErrorKind::Stopped;
        let copied: RaftFatalErrorKind = kind;
        assert_eq!(copied, RaftFatalErrorKind::Stopped);
        // Original still usable (Copy trait)
        assert_eq!(kind, RaftFatalErrorKind::Stopped);
    }

    #[test]
    fn test_fatal_error_kind_debug() {
        let kind = RaftFatalErrorKind::StorageError;
        let debug_str = format!("{:?}", kind);
        assert!(debug_str.contains("StorageError"));
    }

    #[test]
    fn test_fatal_error_kind_serde_roundtrip() {
        for kind in [
            RaftFatalErrorKind::Panicked,
            RaftFatalErrorKind::Stopped,
            RaftFatalErrorKind::StorageError,
        ] {
            let json = serde_json::to_string(&kind).expect("serialize");
            let deserialized: RaftFatalErrorKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(deserialized, kind);
        }
    }

    #[test]
    fn test_rpc_response_fatal_error() {
        let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked);
        assert!(matches!(response, RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked)));
    }

    #[test]
    fn test_rpc_response_fatal_error_clone() {
        let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::Stopped);
        let cloned = response.clone();
        assert!(matches!(cloned, RaftRpcResponse::FatalError(RaftFatalErrorKind::Stopped)));
    }

    #[test]
    fn test_rpc_response_fatal_error_serde_roundtrip() {
        let original = RaftRpcResponse::FatalError(RaftFatalErrorKind::StorageError);

        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: RaftRpcResponse = serde_json::from_str(&json).expect("deserialize");

        assert!(matches!(deserialized, RaftRpcResponse::FatalError(RaftFatalErrorKind::StorageError)));
    }

    #[test]
    fn test_rpc_response_with_timestamps_fatal_error() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 1000,
                server_send_ms: 1005,
            }),
        };

        assert!(matches!(response.inner, RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked)));
        assert!(response.timestamps.is_some());
    }

    #[test]
    fn test_rpc_response_with_timestamps_fatal_error_serde_roundtrip() {
        let original = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::FatalError(RaftFatalErrorKind::Stopped),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 5000,
                server_send_ms: 5010,
            }),
        };

        // Use postcard for wire format testing (what we actually use)
        let bytes = postcard::to_stdvec(&original).expect("serialize");
        let deserialized: RaftRpcResponseWithTimestamps = postcard::from_bytes(&bytes).expect("deserialize");

        assert!(matches!(deserialized.inner, RaftRpcResponse::FatalError(RaftFatalErrorKind::Stopped)));
        assert!(deserialized.timestamps.is_some());
        let ts = deserialized.timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 5000);
        assert_eq!(ts.server_send_ms, 5010);
    }

    // =========================================================================
    // Shard Prefix Encoding/Decoding Tests
    // =========================================================================

    #[test]
    fn test_shard_prefix_size() {
        assert_eq!(SHARD_PREFIX_SIZE, 4);
    }

    #[test]
    fn test_encode_shard_prefix_small() {
        let prefix = encode_shard_prefix(42);
        assert_eq!(prefix, [0, 0, 0, 42]);
    }

    #[test]
    fn test_decode_shard_prefix_small() {
        let shard_id = decode_shard_prefix(&[0, 0, 0, 42]);
        assert_eq!(shard_id, 42);
    }

    #[test]
    fn test_shard_prefix_roundtrip() {
        for shard_id in [0, 1, 42, 255, 256, 1000, 65535, 0x12345678, u32::MAX] {
            let encoded = encode_shard_prefix(shard_id);
            let decoded = decode_shard_prefix(&encoded);
            assert_eq!(decoded, shard_id);
        }
    }

    #[test]
    fn test_try_decode_shard_prefix_success() {
        let data = vec![0, 0, 0, 42, 1, 2, 3, 4];
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_try_decode_shard_prefix_too_short() {
        let data = vec![0, 0, 0];
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, None);
    }

    // =========================================================================
    // ShardedRaftRpcRequest Tests
    // =========================================================================

    #[test]
    fn test_sharded_request_new() {
        let vote_request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(1, NodeId::from(1)),
                last_log_id: None,
            },
        };
        let request = ShardedRaftRpcRequest::new(5, RaftRpcProtocol::Vote(vote_request));

        assert_eq!(request.shard_id, 5);
        assert!(matches!(request.request, RaftRpcProtocol::Vote(_)));
    }

    // =========================================================================
    // ShardedRaftRpcResponse Tests
    // =========================================================================

    #[test]
    fn test_sharded_response_new() {
        let response = ShardedRaftRpcResponse::new(
            3,
            RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(1, NodeId::from(1)),
                vote_granted: true,
                last_log_id: None,
            }),
        );

        assert_eq!(response.shard_id, 3);
        assert!(response.timestamps.is_none());
        assert!(matches!(response.response, RaftRpcResponse::Vote(_)));
    }

    #[test]
    fn test_sharded_response_with_timestamps() {
        let timestamps = TimestampInfo {
            server_recv_ms: 1000,
            server_send_ms: 1005,
        };
        let response = ShardedRaftRpcResponse::with_timestamps(
            8,
            RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success),
            timestamps,
        );

        assert_eq!(response.shard_id, 8);
        assert!(response.timestamps.is_some());
    }
}
