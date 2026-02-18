//! RPC protocol types for Raft consensus over Iroh transport.
//!
//! This module defines the RPC protocol for Raft consensus communication
//! using IRPC over Iroh QUIC connections. The AppTypeConfig here is identical
//! to the one in aspen-raft/src/types.rs to avoid circular dependencies.
//!
//! **IMPORTANT**: The AppTypeConfig declaration below MUST be kept in sync
//! with the one in aspen-raft/src/types.rs. Any changes to the type parameters
//! must be applied to both locations.

use aspen_raft_types::AppRequest;
use aspen_raft_types::AppResponse;
use aspen_raft_types::NodeId;
use aspen_raft_types::RaftMemberInfo;
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

// Declare AppTypeConfig locally to avoid circular dependencies.
// aspen-transport cannot depend on aspen-raft (would create a cycle).
// This is structurally identical to aspen-raft/src/types.rs::AppTypeConfig.
//
// SAFETY: The transmutes between this AppTypeConfig and aspen_raft::AppTypeConfig
// are safe because both declarations use identical type parameters from aspen-raft-types.
// This is verified at compile time by the types in the RPC messages.
openraft::declare_raft_types!(
    pub AppTypeConfig: D = AppRequest, R = AppResponse, NodeId = NodeId, Node = RaftMemberInfo,
);

/// Vote request wrapper for IRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteRequest {
    pub request: VoteRequest<AppTypeConfig>,
}

/// AppendEntries request wrapper for IRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendEntriesRequest {
    pub request: AppendEntriesRequest<AppTypeConfig>,
}

/// Full snapshot request wrapper for IRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftSnapshotRequest {
    pub vote: VoteOf<AppTypeConfig>,
    pub snapshot_meta: openraft::SnapshotMeta<AppTypeConfig>,
    pub snapshot_data: Vec<u8>,
}

/// IRPC service protocol for Raft RPC operations.
#[rpc_requests(message = RaftRpcMessage)]
#[derive(Debug, Serialize, Deserialize)]
pub enum RaftRpcProtocol {
    #[rpc(tx = oneshot::Sender<Result<VoteResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    Vote(RaftVoteRequest),

    #[rpc(tx = oneshot::Sender<Result<AppendEntriesResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    AppendEntries(RaftAppendEntriesRequest),

    #[rpc(tx = oneshot::Sender<Result<SnapshotResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    InstallSnapshot(RaftSnapshotRequest),
}

/// Wire protocol response types that are serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftRpcResponse {
    Vote(VoteResponse<AppTypeConfig>),
    AppendEntries(AppendEntriesResponse<AppTypeConfig>),
    InstallSnapshot(Result<SnapshotResponse<AppTypeConfig>, RaftError<AppTypeConfig>>),
    FatalError(RaftFatalErrorKind),
}

/// Classification of fatal Raft errors for wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftFatalErrorKind {
    Panicked,
    Stopped,
    StorageError,
}

impl RaftFatalErrorKind {
    pub fn from_fatal<C: openraft::RaftTypeConfig>(fatal: &openraft::error::Fatal<C>) -> Self {
        match fatal {
            openraft::error::Fatal::Panicked => Self::Panicked,
            openraft::error::Fatal::Stopped => Self::Stopped,
            _ => Self::StorageError,
        }
    }
}

/// Sharding constants and functions for prefixing RPC messages.
pub const SHARD_PREFIX_SIZE: usize = 4;

pub fn encode_shard_prefix(shard_id: u32) -> [u8; SHARD_PREFIX_SIZE] {
    shard_id.to_be_bytes()
}

pub fn try_decode_shard_prefix(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < SHARD_PREFIX_SIZE {
        return None;
    }
    let prefix: [u8; SHARD_PREFIX_SIZE] = bytes[..SHARD_PREFIX_SIZE].try_into().ok()?;
    Some(u32::from_be_bytes(prefix))
}
