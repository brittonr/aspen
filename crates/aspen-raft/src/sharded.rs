//! Sharded Raft RPC types for horizontal scaling.
//!
//! This module defines types for routing Raft RPCs to specific shards
//! in a horizontally sharded Aspen deployment.

use serde::Deserialize;
use serde::Serialize;

use super::rpc::RaftRpcProtocol;
use super::rpc::RaftRpcResponse;
use super::rpc::TimestampInfo;
// Re-export pure encoding functions for backward compatibility
pub use crate::verified::SHARD_PREFIX_SIZE;
pub use crate::verified::decode_shard_prefix;
pub use crate::verified::encode_shard_prefix;
pub use crate::verified::try_decode_shard_prefix;

/// Sharded RPC request wrapper with shard ID.
///
/// Wraps a standard `RaftRpcProtocol` with a shard ID for routing in
/// sharded deployments. The shard ID identifies which Raft cluster
/// should handle this RPC.
///
/// # Wire Format
///
/// The wire format prefixes the shard ID as 4 bytes big-endian:
/// ```text
/// +----------------+------------------------+
/// | shard_id (4B)  | RaftRpcProtocol (var)  |
/// +----------------+------------------------+
/// ```
///
/// This allows the receiver to read just the shard ID, route to the
/// correct handler, then deserialize the rest.
///
/// Note: This type does not implement Clone because RaftRpcProtocol
/// contains channel types that cannot be cloned. The request is
/// serialized and sent over the wire, not cloned in memory.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShardedRaftRpcRequest {
    /// The shard ID this RPC is targeting.
    pub shard_id: u32,
    /// The inner RPC request.
    pub request: RaftRpcProtocol,
}

/// Sharded RPC response wrapper with shard ID.
///
/// Wraps a standard `RaftRpcResponse` with a shard ID for verification.
/// The shard ID confirms which Raft cluster processed the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardedRaftRpcResponse {
    /// The shard ID this response is from.
    pub shard_id: u32,
    /// The inner RPC response.
    pub response: RaftRpcResponse,
    /// Optional server timestamps for clock drift detection.
    pub timestamps: Option<TimestampInfo>,
}

impl ShardedRaftRpcRequest {
    /// Create a new sharded RPC request.
    pub fn new(shard_id: u32, request: RaftRpcProtocol) -> Self {
        Self { shard_id, request }
    }
}

impl ShardedRaftRpcResponse {
    /// Create a new sharded RPC response.
    pub fn new(shard_id: u32, response: RaftRpcResponse) -> Self {
        Self {
            shard_id,
            response,
            timestamps: None,
        }
    }

    /// Create a new sharded RPC response with timestamps.
    pub fn with_timestamps(shard_id: u32, response: RaftRpcResponse, timestamps: TimestampInfo) -> Self {
        Self {
            shard_id,
            response,
            timestamps: Some(timestamps),
        }
    }
}

#[cfg(test)]
mod tests {
    use openraft::Vote;
    use openraft::raft::AppendEntriesResponse;
    use openraft::raft::VoteRequest;
    use openraft::raft::VoteResponse;

    use super::*;
    use crate::rpc::RaftVoteRequest;
    use crate::types::NodeId;

    // Note: Shard prefix encoding/decoding tests are in verified/encoding.rs

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

    #[test]
    fn test_sharded_request_shard_zero() {
        let vote_request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(1, NodeId::from(1)),
                last_log_id: None,
            },
        };
        let request = ShardedRaftRpcRequest::new(0, RaftRpcProtocol::Vote(vote_request));

        assert_eq!(request.shard_id, 0);
    }

    #[test]
    fn test_sharded_request_shard_id_access() {
        let vote_request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(2, NodeId::from(2)),
                last_log_id: None,
            },
        };
        let request = ShardedRaftRpcRequest::new(10, RaftRpcProtocol::Vote(vote_request));

        // ShardedRaftRpcRequest doesn't implement Clone (contains channel types),
        // but we can access its fields
        assert_eq!(request.shard_id, 10);
        assert!(matches!(request.request, RaftRpcProtocol::Vote(_)));
    }

    #[test]
    fn test_sharded_request_debug() {
        let vote_request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(1, NodeId::from(1)),
                last_log_id: None,
            },
        };
        let request = ShardedRaftRpcRequest::new(7, RaftRpcProtocol::Vote(vote_request));
        let debug_str = format!("{:?}", request);

        assert!(debug_str.contains("ShardedRaftRpcRequest"));
        assert!(debug_str.contains("shard_id"));
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
        let ts = response.timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 1000);
        assert_eq!(ts.server_send_ms, 1005);
    }

    #[test]
    fn test_sharded_response_clone() {
        let response = ShardedRaftRpcResponse::new(
            5,
            RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(3, NodeId::from(3)),
                vote_granted: false,
                last_log_id: None,
            }),
        );
        let cloned = response.clone();

        assert_eq!(cloned.shard_id, 5);
        assert!(cloned.timestamps.is_none());
    }

    #[test]
    fn test_sharded_response_debug() {
        let response = ShardedRaftRpcResponse::new(12, RaftRpcResponse::AppendEntries(AppendEntriesResponse::Conflict));
        let debug_str = format!("{:?}", response);

        assert!(debug_str.contains("ShardedRaftRpcResponse"));
        assert!(debug_str.contains("12"));
    }

    #[test]
    fn test_sharded_response_serde_roundtrip() {
        let original = ShardedRaftRpcResponse::with_timestamps(
            42,
            RaftRpcResponse::Vote(VoteResponse {
                vote: Vote::new(5, NodeId::from(5)),
                vote_granted: true,
                last_log_id: None,
            }),
            TimestampInfo {
                server_recv_ms: 9999,
                server_send_ms: 10001,
            },
        );

        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: ShardedRaftRpcResponse = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.shard_id, 42);
        assert!(deserialized.timestamps.is_some());
        let ts = deserialized.timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 9999);
        assert_eq!(ts.server_send_ms, 10001);
    }

    #[test]
    fn test_sharded_request_serde_roundtrip() {
        let vote_request = RaftVoteRequest {
            request: VoteRequest {
                vote: Vote::new(7, NodeId::from(7)),
                last_log_id: None,
            },
        };
        let original = ShardedRaftRpcRequest::new(99, RaftRpcProtocol::Vote(vote_request));

        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: ShardedRaftRpcRequest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.shard_id, 99);
        assert!(matches!(deserialized.request, RaftRpcProtocol::Vote(_)));
    }
}
