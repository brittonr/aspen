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
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::type_config::alias::VoteOf;
use serde::Deserialize;
use serde::Serialize;

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
    /// Fatal error response indicating the Raft core is in an unrecoverable state.
    ///
    /// Sent when the server's RaftCore has panicked, stopped, or encountered a storage error.
    /// Clients should mark this node as unreachable and retry with other nodes.
    ///
    /// The error kind distinguishes between:
    /// - `Panicked`: RaftCore task panicked (programming error)
    /// - `Stopped`: RaftCore was explicitly shut down
    /// - `StorageError`: Unrecoverable storage failure
    FatalError(RaftFatalErrorKind),
}

/// Classification of fatal Raft errors for wire protocol.
///
/// This is a simplified representation of `openraft::error::Fatal` that can be
/// serialized and sent over the wire. It allows clients to understand why the
/// server cannot process requests and take appropriate action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftFatalErrorKind {
    /// RaftCore panicked - indicates a programming error.
    /// The node should be restarted, and clients should retry with other nodes.
    Panicked,
    /// RaftCore was explicitly stopped via shutdown.
    /// This is a normal shutdown, clients should retry with other nodes.
    Stopped,
    /// Unrecoverable storage error (log or state machine).
    /// Data may be corrupted, requires operator intervention.
    StorageError,
}

impl RaftFatalErrorKind {
    /// Convert from an openraft Fatal error to the wire protocol representation.
    pub fn from_fatal<C: openraft::RaftTypeConfig>(fatal: &openraft::error::Fatal<C>) -> Self {
        match fatal {
            openraft::error::Fatal::Panicked => Self::Panicked,
            openraft::error::Fatal::Stopped => Self::Stopped,
            openraft::error::Fatal::StorageError(_) => Self::StorageError,
        }
    }
}

impl std::fmt::Display for RaftFatalErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panicked => write!(f, "RaftCore panicked"),
            Self::Stopped => write!(f, "RaftCore stopped"),
            Self::StorageError => write!(f, "storage error"),
        }
    }
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

// =============================================================================
// Sharded RPC Types
// =============================================================================

/// Shard ID prefix size in bytes.
///
/// The shard ID is encoded as a 4-byte big-endian u32 at the start of each
/// sharded RPC message. This allows routing to the correct Raft core based
/// on the shard ID before deserializing the full message.
pub const SHARD_PREFIX_SIZE: usize = 4;

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

/// Encode a shard ID as a 4-byte big-endian prefix.
///
/// This is the wire format used for sharded RPC routing. The receiver
/// can read these 4 bytes to determine which shard should handle the
/// RPC before deserializing the full message.
///
/// # Example
///
/// ```ignore
/// let shard_id = 42u32;
/// let prefix = encode_shard_prefix(shard_id);
/// assert_eq!(prefix, [0, 0, 0, 42]);
/// ```
#[inline]
pub fn encode_shard_prefix(shard_id: u32) -> [u8; SHARD_PREFIX_SIZE] {
    shard_id.to_be_bytes()
}

/// Decode a shard ID from a 4-byte big-endian prefix.
///
/// # Example
///
/// ```ignore
/// let prefix = [0, 0, 0, 42];
/// let shard_id = decode_shard_prefix(&prefix);
/// assert_eq!(shard_id, 42);
/// ```
#[inline]
pub fn decode_shard_prefix(bytes: &[u8; SHARD_PREFIX_SIZE]) -> u32 {
    u32::from_be_bytes(*bytes)
}

/// Try to decode a shard ID from a byte slice.
///
/// Returns `None` if the slice is shorter than `SHARD_PREFIX_SIZE` bytes.
///
/// # Example
///
/// ```ignore
/// let data = vec![0, 0, 0, 42, /* rest of message */];
/// let shard_id = try_decode_shard_prefix(&data);
/// assert_eq!(shard_id, Some(42));
/// ```
#[inline]
pub fn try_decode_shard_prefix(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < SHARD_PREFIX_SIZE {
        return None;
    }
    let prefix: [u8; SHARD_PREFIX_SIZE] = bytes[..SHARD_PREFIX_SIZE].try_into().ok()?;
    Some(decode_shard_prefix(&prefix))
}

#[cfg(test)]
mod tests {
    use openraft::Vote;

    use super::*;
    use crate::raft::types::NodeId;

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
    // Shard Prefix Encoding/Decoding Tests
    // =========================================================================

    #[test]
    fn test_shard_prefix_size() {
        assert_eq!(SHARD_PREFIX_SIZE, 4);
    }

    #[test]
    fn test_encode_shard_prefix_zero() {
        let prefix = encode_shard_prefix(0);
        assert_eq!(prefix, [0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_shard_prefix_small() {
        let prefix = encode_shard_prefix(42);
        assert_eq!(prefix, [0, 0, 0, 42]);
    }

    #[test]
    fn test_encode_shard_prefix_256() {
        let prefix = encode_shard_prefix(256);
        assert_eq!(prefix, [0, 0, 1, 0]);
    }

    #[test]
    fn test_encode_shard_prefix_large() {
        let prefix = encode_shard_prefix(0x12345678);
        assert_eq!(prefix, [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_encode_shard_prefix_max() {
        let prefix = encode_shard_prefix(u32::MAX);
        assert_eq!(prefix, [0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_decode_shard_prefix_zero() {
        let shard_id = decode_shard_prefix(&[0, 0, 0, 0]);
        assert_eq!(shard_id, 0);
    }

    #[test]
    fn test_decode_shard_prefix_small() {
        let shard_id = decode_shard_prefix(&[0, 0, 0, 42]);
        assert_eq!(shard_id, 42);
    }

    #[test]
    fn test_decode_shard_prefix_large() {
        let shard_id = decode_shard_prefix(&[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(shard_id, 0x12345678);
    }

    #[test]
    fn test_decode_shard_prefix_max() {
        let shard_id = decode_shard_prefix(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(shard_id, u32::MAX);
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
        let data = vec![0, 0, 0, 42, 1, 2, 3, 4]; // shard_id followed by other data
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_try_decode_shard_prefix_exact_size() {
        let data = vec![0, 0, 1, 0]; // exactly 4 bytes
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, Some(256));
    }

    #[test]
    fn test_try_decode_shard_prefix_too_short() {
        let data = vec![0, 0, 0]; // only 3 bytes
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_decode_shard_prefix_empty() {
        let data: Vec<u8> = vec![];
        let result = try_decode_shard_prefix(&data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_try_decode_shard_prefix_one_byte() {
        let data = vec![42];
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
}
