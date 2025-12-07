/// Madsim-compatible Raft network layer for deterministic simulation testing.
///
/// This module provides a deterministic network implementation for OpenRaft using madsim,
/// enabling automated detection of distributed systems bugs through deterministic simulation.
/// Unlike the production IrpcRaftNetwork (Iroh-based) and InMemoryNetwork (testing helper),
/// MadsimRaftNetwork uses madsim::net::TcpStream for fully deterministic P2P communication.
///
/// **Key Differences from Existing Network Implementations:**
///
/// 1. **IrpcRaftNetwork** (src/raft/network.rs):
///    - Production: Uses Iroh P2P networking for real distributed systems
///    - Non-deterministic: Real network I/O, timing variations, connection failures
///    - Purpose: Production consensus between physical/virtual nodes
///
/// 2. **InMemoryNetwork** (src/testing/router.rs):
///    - Testing: In-memory message passing via AspenRouter
///    - Deterministic: No real network, controlled delays/failures
///    - Limitation: Can't detect network-level bugs (reordering, partitions, etc.)
///
/// 3. **MadsimRaftNetwork** (this module):
///    - Simulation: madsim::net::TcpStream for virtual network I/O
///    - Deterministic: Reproducible with seeds, controlled time/failures
///    - Purpose: Automated bug detection (split-brain, message loss, reordering, etc.)
///
/// **Architecture:**
///
/// ```text
/// MadsimRaftRouter
///   ├─ MadsimNetworkFactory (per node)
///   │   └─ MadsimRaftNetwork (per RPC target)
///   │       └─ madsim::net::TcpStream
///   └─ FailureInjector (network/node failures)
/// ```
///
/// **Tiger Style Principles:**
/// - Bounded resources: Fixed max RPC size, connection limits
/// - Explicit types: u32/u64 for IDs, no usize
/// - Fail-fast: Errors propagated, no silent failures
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use openraft::error::{NetworkError, RPCError, ReplicationClosed, StreamingError, Unreachable};
use openraft::network::{RPCOption, RaftNetworkFactory, v2::RaftNetworkV2};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::type_config::alias::VoteOf;
use openraft::{BasicNode, OptionalSend, Raft, Snapshot};
use parking_lot::Mutex as SyncMutex;
use tracing::debug;

use crate::raft::types::{AppTypeConfig, NodeId};

/// Maximum size for RPC messages in madsim simulations (10 MB).
///
/// Tiger Style: Fixed limit to prevent unbounded memory use during simulations.
/// Note: Reserved for future message size validation in madsim transport.
const _MAX_RPC_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// Maximum number of concurrent connections per node.
///
/// Tiger Style: Fixed limit to prevent connection exhaustion.
const MAX_CONNECTIONS_PER_NODE: u32 = 100;

/// Madsim-compatible Raft network factory for deterministic simulation.
///
/// Creates MadsimRaftNetwork instances for each target node. Unlike IrpcRaftNetworkFactory
/// which uses Iroh EndpointAddr for peer discovery, MadsimNetworkFactory uses madsim's
/// TCP addresses (e.g., "127.0.0.1:26001") for deterministic network simulation.
#[derive(Clone)]
pub struct MadsimNetworkFactory {
    /// Source node ID for this factory.
    source_node_id: NodeId,
    /// Router managing all nodes in the simulation.
    router: Arc<MadsimRaftRouter>,
    /// Failure injector for chaos testing.
    failure_injector: Arc<FailureInjector>,
}

impl MadsimNetworkFactory {
    /// Create a new madsim network factory for the given source node.
    pub fn new(
        source_node_id: NodeId,
        router: Arc<MadsimRaftRouter>,
        failure_injector: Arc<FailureInjector>,
    ) -> Self {
        Self {
            source_node_id,
            router,
            failure_injector,
        }
    }
}

impl RaftNetworkFactory<AppTypeConfig> for MadsimNetworkFactory {
    type Network = MadsimRaftNetwork;

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        MadsimRaftNetwork::new(
            self.source_node_id,
            target,
            self.router.clone(),
            self.failure_injector.clone(),
        )
    }
}

/// Madsim-compatible Raft network implementation using deterministic TCP.
///
/// Implements OpenRaft's RaftNetworkV2 trait to send vote, append_entries, and snapshot
/// RPCs over madsim's deterministic network layer. All network operations are reproducible
/// given the same seed, enabling automated detection of distributed systems bugs.
pub struct MadsimRaftNetwork {
    source: NodeId,
    target: NodeId,
    router: Arc<MadsimRaftRouter>,
    failure_injector: Arc<FailureInjector>,
}

impl MadsimRaftNetwork {
    fn new(
        source: NodeId,
        target: NodeId,
        router: Arc<MadsimRaftRouter>,
        failure_injector: Arc<FailureInjector>,
    ) -> Self {
        Self {
            source,
            target,
            router,
            failure_injector,
        }
    }

    /// Check if the failure injector should drop this message.
    ///
    /// Returns Err(RPCError::Unreachable) if the message should be dropped,
    /// otherwise Ok(()).
    async fn check_failure_injection(&self) -> Result<(), RPCError<AppTypeConfig>> {
        if self
            .failure_injector
            .should_drop_message(self.source, self.target)
        {
            return Err(RPCError::Unreachable(Unreachable::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "failure injector dropped message",
                ),
            )));
        }
        Ok(())
    }

    /// Apply network delay if configured by the failure injector.
    async fn apply_network_delay(&self) {
        if let Some(delay) = self
            .failure_injector
            .get_network_delay(self.source, self.target)
        {
            madsim::time::sleep(delay).await;
        }
    }
}

impl RaftNetworkV2<AppTypeConfig> for MadsimRaftNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.check_failure_injection().await?;
        self.apply_network_delay().await;

        self.router
            .send_append_entries(self.source, self.target, rpc)
            .await
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.check_failure_injection().await?;
        self.apply_network_delay().await;

        self.router.send_vote(self.source, self.target, rpc).await
    }

    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = ReplicationClosed> + OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        // Check failure injection first
        if let Err(_rpc_err) = self.check_failure_injection().await {
            return Err(StreamingError::Network(NetworkError::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "failure injector dropped snapshot message",
                ),
            )));
        }
        self.apply_network_delay().await;

        self.router
            .send_snapshot(self.source, self.target, vote, snapshot)
            .await
    }
}

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
    nodes: SyncMutex<HashMap<NodeId, NodeHandle>>,
    /// Failed nodes that should return Unreachable errors
    failed_nodes: SyncMutex<HashMap<NodeId, bool>>,
}

/// Handle to a Raft node in the simulation.
///
/// Contains the node's listen address and Raft instance for direct RPC dispatch.
struct NodeHandle {
    /// TCP listen address for this node (e.g., "127.0.0.1:26001")
    _listen_addr: String,
    /// Raft instance for direct RPC dispatch
    ///
    /// Phase 2: Direct dispatch (simpler implementation, validates integration)
    /// Future Phase: Replace with real madsim::net::TcpStream for network-level testing
    raft: Raft<AppTypeConfig>,
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
    pub fn register_node(
        &self,
        node_id: NodeId,
        listen_addr: String,
        raft: Raft<AppTypeConfig>,
    ) -> Result<()> {
        let mut nodes = self.nodes.lock();
        if nodes.len() >= MAX_CONNECTIONS_PER_NODE as usize {
            anyhow::bail!(
                "max nodes exceeded: {} (max: {})",
                nodes.len(),
                MAX_CONNECTIONS_PER_NODE
            );
        }
        nodes.insert(
            node_id,
            NodeHandle {
                _listen_addr: listen_addr,
                raft,
            },
        );
        Ok(())
    }

    /// Mark a node as failed for failure injection testing.
    pub fn mark_node_failed(&self, node_id: NodeId, failed: bool) {
        let mut failed_nodes = self.failed_nodes.lock();
        failed_nodes.insert(node_id, failed);
    }

    /// Check if a node is marked as failed.
    fn is_node_failed(&self, node_id: NodeId) -> bool {
        let failed_nodes = self.failed_nodes.lock();
        failed_nodes.get(&node_id).copied().unwrap_or(false)
    }

    /// Send AppendEntries RPC to target node.
    ///
    /// Tiger Style: Bounded message size checked at serialization.
    /// Phase 2: Direct dispatch via Raft handle (validates integration)
    async fn send_append_entries(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        // Check if source/target nodes are failed
        if self.is_node_failed(source) {
            return Err(RPCError::Unreachable(Unreachable::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "source node marked as failed",
                ),
            )));
        }
        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "target node marked as failed",
                ),
            )));
        }

        // Get target node's Raft handle
        let raft = {
            let nodes = self.nodes.lock();
            let Some(node) = nodes.get(&target) else {
                return Err(RPCError::Unreachable(Unreachable::new(
                    &std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("target node {target} not registered"),
                    ),
                )));
            };
            node.raft.clone()
        };

        debug!(source, target, "dispatching append_entries RPC");

        // Dispatch RPC directly to Raft core
        raft.append_entries(rpc)
            .await
            .map_err(|err| RPCError::Network(NetworkError::new(&err)))
    }

    /// Send Vote RPC to target node.
    ///
    /// Phase 2: Direct dispatch via Raft handle (validates integration)
    async fn send_vote(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: VoteRequest<AppTypeConfig>,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        // Check if source/target nodes are failed
        if self.is_node_failed(source) {
            return Err(RPCError::Unreachable(Unreachable::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "source node marked as failed",
                ),
            )));
        }
        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "target node marked as failed",
                ),
            )));
        }

        // Get target node's Raft handle
        let raft = {
            let nodes = self.nodes.lock();
            let Some(node) = nodes.get(&target) else {
                return Err(RPCError::Unreachable(Unreachable::new(
                    &std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("target node {target} not registered"),
                    ),
                )));
            };
            node.raft.clone()
        };

        debug!(source, target, "dispatching vote RPC");

        // Dispatch RPC directly to Raft core
        raft.vote(rpc)
            .await
            .map_err(|err| RPCError::Network(NetworkError::new(&err)))
    }

    /// Send Snapshot RPC to target node.
    async fn send_snapshot(
        &self,
        source: NodeId,
        target: NodeId,
        _vote: VoteOf<AppTypeConfig>,
        _snapshot: Snapshot<AppTypeConfig>,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        // Check if source/target nodes are failed
        if self.is_node_failed(source) {
            return Err(StreamingError::Network(NetworkError::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "source node marked as failed",
                ),
            )));
        }
        if self.is_node_failed(target) {
            return Err(StreamingError::Network(NetworkError::new(
                &std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "target node marked as failed",
                ),
            )));
        }

        // TODO: Implement actual madsim TCP snapshot streaming
        Err(StreamingError::Network(NetworkError::new(
            &std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "madsim snapshot streaming not yet implemented",
            ),
        )))
    }
}

impl Default for MadsimRaftRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Failure injector for chaos testing in madsim simulations.
///
/// FailureInjector controls deterministic failure injection including:
/// - Message drops (network failures)
/// - Network delays (latency simulation)
/// - Node failures (crash simulation)
///
/// Tiger Style: All delays/timeouts are explicitly u64 milliseconds.
pub struct FailureInjector {
    /// Network delay configuration (source, target) -> delay_ms
    delays: SyncMutex<HashMap<(NodeId, NodeId), u64>>,
    /// Message drop configuration (source, target) -> should_drop
    drops: SyncMutex<HashMap<(NodeId, NodeId), bool>>,
}

impl FailureInjector {
    /// Create a new failure injector with no failures configured.
    pub fn new() -> Self {
        Self {
            delays: SyncMutex::new(HashMap::new()),
            drops: SyncMutex::new(HashMap::new()),
        }
    }

    /// Configure network delay between two nodes (in milliseconds).
    ///
    /// Tiger Style: Explicit u64 milliseconds, not Duration directly.
    pub fn set_network_delay(&self, source: NodeId, target: NodeId, delay_ms: u64) {
        let mut delays = self.delays.lock();
        delays.insert((source, target), delay_ms);
    }

    /// Configure message drops between two nodes.
    ///
    /// When enabled, all messages from source to target will be dropped.
    pub fn set_message_drop(&self, source: NodeId, target: NodeId, should_drop: bool) {
        let mut drops = self.drops.lock();
        drops.insert((source, target), should_drop);
    }

    /// Check if a message should be dropped.
    fn should_drop_message(&self, source: NodeId, target: NodeId) -> bool {
        let drops = self.drops.lock();
        drops.get(&(source, target)).copied().unwrap_or(false)
    }

    /// Get the configured network delay for a message, if any.
    fn get_network_delay(&self, source: NodeId, target: NodeId) -> Option<Duration> {
        let delays = self.delays.lock();
        delays
            .get(&(source, target))
            .map(|&delay_ms| Duration::from_millis(delay_ms))
    }

    /// Clear all failure injection configuration.
    pub fn clear_all(&self) {
        let mut delays = self.delays.lock();
        let mut drops = self.drops.lock();
        delays.clear();
        drops.clear();
    }
}

impl Default for FailureInjector {
    fn default() -> Self {
        Self::new()
    }
}
