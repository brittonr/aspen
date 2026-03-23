//! Write forwarding for follower nodes.
//!
//! When a follower node receives a write request, the local Raft instance
//! returns `ForwardToLeader`. Instead of surfacing this as `NotLeader` to
//! internal callers (workers, coordination primitives), the `WriteForwarder`
//! transparently forwards the write to the current leader via iroh QUIC.

use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use async_trait::async_trait;
use iroh::EndpointAddr;

use crate::types::NodeId;

/// Forwards write requests from a follower to the current Raft leader.
///
/// Implementations connect to the leader via the cluster's network transport
/// and execute the write on the leader's behalf. The forwarding is transparent
/// to the caller — they see the same `WriteResult` as if they were on the leader.
#[async_trait]
pub trait WriteForwarder: Send + Sync {
    /// Forward a write request to the specified leader node.
    ///
    /// `leader_addr` is the iroh endpoint address from the Raft membership.
    /// Returns the leader's write result on success. If the target node is also
    /// not the leader (stale hint), returns `NotLeader` — the caller's retry
    /// loop handles re-discovery.
    async fn forward_write(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: WriteRequest,
    ) -> Result<WriteResult, KeyValueStoreError>;
}
