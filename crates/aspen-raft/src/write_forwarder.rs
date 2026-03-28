//! Write forwarding for follower nodes.
//!
//! When a follower node receives a write request, the local Raft instance
//! returns `ForwardToLeader`. Instead of surfacing this as `NotLeader` to
//! internal callers (workers, coordination primitives), the `WriteForwarder`
//! transparently forwards the write to the current leader via iroh QUIC.

use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use async_trait::async_trait;
use iroh::EndpointAddr;

use crate::types::NodeId;

/// Forwards requests from a follower to the current Raft leader.
///
/// Implementations connect to the leader via the cluster's network transport
/// and execute the request on the leader's behalf. The forwarding is transparent
/// to the caller — they see the same result as if they were on the leader.
///
/// Covers both writes (which require leader for Raft consensus) and reads
/// (which require leader for linearizable ReadIndex confirmation).
#[async_trait]
pub trait WriteForwarder: Send + Sync {
    /// Forward a write request to the specified leader node.
    async fn forward_write(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: WriteRequest,
    ) -> Result<WriteResult, KeyValueStoreError>;

    /// Forward a read request to the leader for linearizable consistency.
    ///
    /// The leader performs ReadIndex confirmation and returns the value.
    async fn forward_read(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: ReadRequest,
    ) -> Result<ReadResult, KeyValueStoreError>;

    /// Forward a scan request to the leader for linearizable consistency.
    async fn forward_scan(
        &self,
        leader_id: NodeId,
        leader_addr: EndpointAddr,
        request: ScanRequest,
    ) -> Result<ScanResult, KeyValueStoreError>;
}
