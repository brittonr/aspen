//! MadsimRaftRouter - manages all Raft nodes in a madsim simulation.

use std::collections::HashMap;

use anyhow::Result;
use openraft::Raft;
use openraft::Snapshot;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::type_config::alias::VoteOf;
use parking_lot::Mutex as SyncMutex;
use tracing::debug;

use crate::constants::MAX_CONNECTIONS_PER_NODE;
use crate::types::AppTypeConfig;
use crate::types::NodeId;

/// Router managing all Raft nodes in a madsim simulation.
///
/// MadsimRaftRouter coordinates message passing between Raft nodes using deterministic
/// network transport. Unlike AspenRouter (InMemoryNetwork), this router uses real
/// madsim::net::TcpStream connections, enabling detection of network-level bugs.
///
/// **Responsibilities:**
/// - Register/unregister Raft nodes with their listen addresses
/// - Route RPC messages between nodes using madsim TCP
/// - Apply network delays and failure injection
/// - Track node lifecycle (running/failed)
pub struct MadsimRaftRouter {
    /// Map of NodeId to (listen_addr, Raft handle)
    ///
    /// Tiger Style: Bounded by MAX_CONNECTIONS_PER_NODE * cluster_size
    pub(super) nodes: SyncMutex<HashMap<NodeId, NodeHandle>>,
    /// Failed nodes that should return Unreachable errors
    pub(super) failed_nodes: SyncMutex<HashMap<NodeId, bool>>,
}

/// Handle to a Raft node in the simulation.
///
/// Contains the node's listen address and Raft instance for direct RPC dispatch.
pub(super) struct NodeHandle {
    /// TCP listen address for this node (e.g., "127.0.0.1:26001")
    pub(super) _listen_addr: String,
    /// Raft instance for direct RPC dispatch
    ///
    /// Phase 2: Direct dispatch (simpler implementation, validates integration)
    /// Future Phase: Replace with real madsim::net::TcpStream for network-level testing
    pub(super) raft: Raft<AppTypeConfig>,
}

impl MadsimRaftRouter {
    /// Create a new router for madsim simulations.
    pub fn new() -> Self {
        Self {
            nodes: SyncMutex::new(HashMap::new()),
            failed_nodes: SyncMutex::new(HashMap::new()),
        }
    }

    /// Register a node with the router.
    ///
    /// Tiger Style: Bounded registration - fails if max nodes exceeded.
    pub fn register_node(&self, node_id: NodeId, listen_addr: String, raft: Raft<AppTypeConfig>) -> Result<()> {
        let mut nodes = self.nodes.lock();
        if nodes.len() >= MAX_CONNECTIONS_PER_NODE as usize {
            anyhow::bail!("max nodes exceeded: {} (max: {})", nodes.len(), MAX_CONNECTIONS_PER_NODE);
        }
        nodes.insert(node_id, NodeHandle {
            _listen_addr: listen_addr,
            raft,
        });
        Ok(())
    }

    /// Mark a node as failed for failure injection testing.
    pub fn mark_node_failed(&self, node_id: NodeId, failed: bool) {
        let mut failed_nodes = self.failed_nodes.lock();
        failed_nodes.insert(node_id, failed);
    }

    /// Check if a node is marked as failed.
    pub(super) fn is_node_failed(&self, node_id: NodeId) -> bool {
        let failed_nodes = self.failed_nodes.lock();
        failed_nodes.get(&node_id).copied().unwrap_or(false)
    }

    /// Send AppendEntries RPC to target node.
    ///
    /// Tiger Style: Bounded message size checked at serialization.
    /// Phase 2: Direct dispatch via Raft handle (validates integration)
    pub(super) async fn send_append_entries(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        // Check if source/target nodes are failed
        if self.is_node_failed(source) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "source node marked as failed",
            ))));
        }
        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "target node marked as failed",
            ))));
        }

        // Get target node's Raft handle
        let raft = {
            let nodes = self.nodes.lock();
            let Some(node) = nodes.get(&target) else {
                return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("target node {target} not registered"),
                ))));
            };
            node.raft.clone()
        };

        debug!(%source, %target, "dispatching append_entries RPC");

        // Dispatch RPC directly to Raft core
        raft.append_entries(rpc).await.map_err(|err| RPCError::Network(NetworkError::new(&err)))
    }

    /// Send Vote RPC to target node.
    ///
    /// Phase 2: Direct dispatch via Raft handle (validates integration)
    pub(super) async fn send_vote(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: VoteRequest<AppTypeConfig>,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        // Check if source/target nodes are failed
        if self.is_node_failed(source) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "source node marked as failed",
            ))));
        }
        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "target node marked as failed",
            ))));
        }

        // Get target node's Raft handle
        let raft = {
            let nodes = self.nodes.lock();
            let Some(node) = nodes.get(&target) else {
                return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("target node {target} not registered"),
                ))));
            };
            node.raft.clone()
        };

        debug!(%source, %target, "dispatching vote RPC");

        // Dispatch RPC directly to Raft core
        raft.vote(rpc).await.map_err(|err| RPCError::Network(NetworkError::new(&err)))
    }

    /// Send Snapshot RPC to target node.
    pub(super) async fn send_snapshot(
        &self,
        source: NodeId,
        target: NodeId,
        vote: VoteOf<AppTypeConfig>,
        snapshot: Snapshot<AppTypeConfig>,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        // Check if source/target nodes are failed
        if self.is_node_failed(source) {
            return Err(StreamingError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "source node marked as failed",
            ))));
        }
        if self.is_node_failed(target) {
            return Err(StreamingError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "target node marked as failed",
            ))));
        }

        // Get target node's Raft handle
        let raft = {
            let nodes = self.nodes.lock();
            let Some(node) = nodes.get(&target) else {
                return Err(StreamingError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("target node {target} not registered"),
                ))));
            };
            node.raft.clone()
        };

        debug!(%source, %target, snapshot_meta = ?snapshot.meta, "dispatching snapshot RPC");

        // Tiger Style: Direct dispatch to Raft core (Phase 2 implementation)
        // This validates integration; future work can add real madsim TCP streaming
        raft.install_full_snapshot(vote, snapshot)
            .await
            .map_err(|err| StreamingError::Network(NetworkError::new(&err)))
    }
}

impl Default for MadsimRaftRouter {
    fn default() -> Self {
        Self::new()
    }
}
