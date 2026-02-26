//! Sharded RPC request/response wrappers and shard prefix encoding.
//!
//! Wraps RPC messages with shard IDs for routing in sharded deployments.

use serde::Deserialize;
use serde::Serialize;

use super::messages::RaftRpcProtocol;
use super::messages::RaftRpcResponse;
use super::messages::TimestampInfo;
// Re-export encoding functions from verified module
pub use crate::verified::SHARD_PREFIX_SIZE;
pub use crate::verified::decode_shard_prefix;
pub use crate::verified::encode_shard_prefix;
pub use crate::verified::try_decode_shard_prefix;

/// Sharded RPC request wrapper.
///
/// Wraps an RPC request with a shard ID for routing in sharded deployments.
/// The shard ID determines which Raft core should handle the request.
///
/// Note: This type does not implement Clone because RaftRpcProtocol
/// contains channel types that cannot be cloned. The request is
/// serialized and sent over the wire, not cloned in memory.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShardedRaftRpcRequest {
    /// The shard ID for routing.
    pub shard_id: u32,
    /// The inner RPC request.
    pub request: RaftRpcProtocol,
}

impl ShardedRaftRpcRequest {
    /// Create a new sharded request.
    pub fn new(shard_id: u32, request: RaftRpcProtocol) -> Self {
        Self { shard_id, request }
    }
}

/// Sharded RPC response wrapper.
///
/// Wraps an RPC response with a shard ID for correlation in sharded deployments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardedRaftRpcResponse {
    /// The shard ID for correlation.
    pub shard_id: u32,
    /// The inner RPC response.
    pub response: RaftRpcResponse,
    /// Optional server timestamps for clock drift detection.
    pub timestamps: Option<TimestampInfo>,
}

impl ShardedRaftRpcResponse {
    /// Create a new sharded response without timestamps.
    pub fn new(shard_id: u32, response: RaftRpcResponse) -> Self {
        Self {
            shard_id,
            response,
            timestamps: None,
        }
    }

    /// Create a new sharded response with timestamps.
    pub fn with_timestamps(shard_id: u32, response: RaftRpcResponse, timestamps: TimestampInfo) -> Self {
        Self {
            shard_id,
            response,
            timestamps: Some(timestamps),
        }
    }
}
