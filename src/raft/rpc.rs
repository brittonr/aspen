//! IRPC service definition for Raft RPC over Iroh.
//!
//! This module defines the RPC interface for Raft consensus communication using IRPC
//! over Iroh QUIC connections. It provides type-safe, serializable request/response pairs
//! that match OpenRaft's v2 network API.
//!
//! For now, this module provides type definitions but does not include the actual IRPC
//! service implementation. The HTTP-based transport remains in use until the Iroh
//! endpoint management and IRPC client/server are fully wired.

use openraft::raft::{AppendEntriesRequest, VoteRequest};
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

// TODO: Complete IRPC service implementation
// The full IRPC service with #[rpc_requests] macro and client/server code
// will be added in subsequent commits as part of Phase 3 implementation.
