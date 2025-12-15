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
    pub request: VoteRequest<AppTypeConfig>,
}

/// AppendEntries request wrapper for IRPC.
///
/// Sent by leader for log replication and heartbeats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendEntriesRequest {
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
    pub vote: VoteOf<AppTypeConfig>,
    pub snapshot_meta: openraft::SnapshotMeta<AppTypeConfig>,
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
    Vote(VoteResponse<AppTypeConfig>),
    AppendEntries(AppendEntriesResponse<AppTypeConfig>),
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
