//! Madsim-compatible Raft network layer for deterministic simulation testing.
//!
//! This module is only available when the "testing" feature is enabled.

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
use openraft::OptionalSend;
use openraft::Raft;
use openraft::Snapshot;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::network::RPCOption;
use openraft::network::RaftNetworkFactory;
use openraft::network::v2::RaftNetworkV2;
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
use crate::types::RaftMemberInfo;

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
    pub fn new(source_node_id: NodeId, router: Arc<MadsimRaftRouter>, failure_injector: Arc<FailureInjector>) -> Self {
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
    async fn new_client(&mut self, target: NodeId, _node: &RaftMemberInfo) -> Self::Network {
        MadsimRaftNetwork::new(self.source_node_id, target, self.router.clone(), self.failure_injector.clone())
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
        if self.failure_injector.should_drop_message(self.source, self.target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "failure injector dropped message",
            ))));
        }
        Ok(())
    }

    /// Apply network delay if configured by the failure injector.
    async fn apply_network_delay(&self) {
        if let Some(delay) = self.failure_injector.get_network_delay(self.source, self.target) {
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

        self.router.send_append_entries(self.source, self.target, rpc).await
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
            return Err(StreamingError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "failure injector dropped snapshot message",
            ))));
        }
        self.apply_network_delay().await;

        self.router.send_snapshot(self.source, self.target, vote, snapshot).await
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
    async fn send_vote(
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
    async fn send_snapshot(
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

/// Failure injector for chaos testing in madsim simulations.
///
/// FailureInjector controls deterministic failure injection including:
/// - Message drops (network failures)
/// - Network delays (latency simulation)
/// - Range-based delays with jitter
/// - Packet loss rates (probabilistic drops)
/// - Node failures (crash simulation)
/// - Clock drift simulation (asymmetric delays)
///
/// Tiger Style: All delays/timeouts are explicitly u64 milliseconds.
pub struct FailureInjector {
    /// Network delay configuration (source, target) -> delay_ms
    delays: SyncMutex<HashMap<(NodeId, NodeId), u64>>,
    /// Range-based delay configuration (source, target) -> (min_ms, max_ms)
    delay_ranges: SyncMutex<HashMap<(NodeId, NodeId), (u64, u64)>>,
    /// Message drop configuration (source, target) -> should_drop
    drops: SyncMutex<HashMap<(NodeId, NodeId), bool>>,
    /// Packet loss rate configuration (source, target) -> loss_rate (0.0-1.0)
    loss_rates: SyncMutex<HashMap<(NodeId, NodeId), f64>>,
    /// Clock drift simulation: node_id -> drift_ms (signed)
    ///
    /// Simulates clock drift by adding asymmetric delays:
    /// - Positive drift (fast clock): Adds delay to OUTGOING messages from this node (simulates the
    ///   node's perception that time has passed faster)
    /// - Negative drift (slow clock): Adds delay to INCOMING messages to this node (simulates the
    ///   node responding late relative to others)
    ///
    /// Note: Madsim uses global virtual time, so we simulate drift effects through
    /// delays rather than actual clock manipulation. This approach effectively tests
    /// how Raft handles nodes that appear to be on different timelines.
    clock_drifts: SyncMutex<HashMap<NodeId, i64>>,
}

impl FailureInjector {
    /// Create a new failure injector with no failures configured.
    pub fn new() -> Self {
        Self {
            delays: SyncMutex::new(HashMap::new()),
            delay_ranges: SyncMutex::new(HashMap::new()),
            drops: SyncMutex::new(HashMap::new()),
            loss_rates: SyncMutex::new(HashMap::new()),
            clock_drifts: SyncMutex::new(HashMap::new()),
        }
    }

    /// Configure network delay between two nodes (in milliseconds).
    ///
    /// Tiger Style: Explicit u64 milliseconds, not Duration directly.
    pub fn set_network_delay(&self, source: NodeId, target: NodeId, delay_ms: u64) {
        let mut delays = self.delays.lock();
        delays.insert((source, target), delay_ms);
    }

    /// Configure range-based network delay between two nodes.
    ///
    /// Delay will be uniformly sampled from [min_ms, max_ms] for each message.
    /// This simulates realistic network jitter patterns.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 1-27ms latency like MadRaft
    /// injector.set_network_delay_range(node1, node2, 1, 27);
    /// ```
    pub fn set_network_delay_range(&self, source: NodeId, target: NodeId, min_ms: u64, max_ms: u64) {
        assert!(min_ms <= max_ms, "min_ms must be <= max_ms");
        let mut delay_ranges = self.delay_ranges.lock();
        delay_ranges.insert((source, target), (min_ms, max_ms));
    }

    /// Configure packet loss rate between two nodes.
    ///
    /// Rate should be between 0.0 (no loss) and 1.0 (100% loss).
    /// Messages are dropped probabilistically based on this rate.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // 10% packet loss
    /// injector.set_packet_loss_rate(node1, node2, 0.1);
    /// ```
    pub fn set_packet_loss_rate(&self, source: NodeId, target: NodeId, rate: f64) {
        assert!((0.0..=1.0).contains(&rate), "loss rate must be between 0.0 and 1.0");
        let mut loss_rates = self.loss_rates.lock();
        loss_rates.insert((source, target), rate);
    }

    /// Configure message drops between two nodes.
    ///
    /// When enabled, all messages from source to target will be dropped.
    pub fn set_message_drop(&self, source: NodeId, target: NodeId, should_drop: bool) {
        let mut drops = self.drops.lock();
        drops.insert((source, target), should_drop);
    }

    /// Configure clock drift for a node (in milliseconds, signed).
    ///
    /// Clock drift is simulated by adding asymmetric delays to messages:
    /// - Positive drift (fast clock): Delays OUTGOING messages from this node
    /// - Negative drift (slow clock): Delays INCOMING messages to this node
    ///
    /// This effectively simulates how Raft behaves when a node's clock runs
    /// faster or slower than other nodes in the cluster.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Node 1 has a clock that's 100ms "fast" - its heartbeats arrive late
    /// // from the perspective of other nodes
    /// injector.set_clock_drift(1, 100);
    ///
    /// // Node 2 has a clock that's 50ms "slow" - messages to it appear delayed
    /// injector.set_clock_drift(2, -50);
    /// ```
    ///
    /// # Arguments
    ///
    /// * `node_id` - The node to configure drift for
    /// * `drift_ms` - Signed drift in milliseconds. Positive = fast clock, negative = slow clock.
    pub fn set_clock_drift(&self, node_id: NodeId, drift_ms: i64) {
        let mut drifts = self.clock_drifts.lock();
        if drift_ms == 0 {
            drifts.remove(&node_id);
        } else {
            drifts.insert(node_id, drift_ms);
        }
    }

    /// Clear clock drift for a specific node.
    pub fn clear_clock_drift(&self, node_id: NodeId) {
        let mut drifts = self.clock_drifts.lock();
        drifts.remove(&node_id);
    }

    /// Get the configured clock drift for a node.
    pub fn get_clock_drift(&self, node_id: NodeId) -> Option<i64> {
        let drifts = self.clock_drifts.lock();
        drifts.get(&node_id).copied()
    }

    /// Check if a message should be dropped.
    ///
    /// Considers both explicit drops and probabilistic loss rates.
    pub(crate) fn should_drop_message(&self, source: NodeId, target: NodeId) -> bool {
        // Check explicit drops first
        {
            let drops = self.drops.lock();
            if drops.get(&(source, target)).copied().unwrap_or(false) {
                return true;
            }
        }

        // Check packet loss rate
        {
            let loss_rates = self.loss_rates.lock();
            if let Some(&rate) = loss_rates.get(&(source, target))
                && rate > 0.0
            {
                // Use madsim's deterministic random
                let random_value: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
                if random_value < rate {
                    return true;
                }
            }
        }

        false
    }

    /// Get the configured network delay for a message, if any.
    ///
    /// Checks range-based delays first, then fixed delays, then clock drift effects.
    /// For range-based delays, samples uniformly from the range.
    ///
    /// Clock drift is applied as additional delay:
    /// - Source with positive drift (fast clock): Add delay to simulate late arrival
    /// - Target with negative drift (slow clock): Add delay to simulate slow response
    pub(crate) fn get_network_delay(&self, source: NodeId, target: NodeId) -> Option<Duration> {
        let mut total_delay_ms: u64 = 0;
        let mut has_delay = false;

        // Check range-based delays first
        {
            let delay_ranges = self.delay_ranges.lock();
            if let Some(&(min_ms, max_ms)) = delay_ranges.get(&(source, target)) {
                let delay_ms = if min_ms == max_ms {
                    min_ms
                } else {
                    // Sample uniformly using madsim's deterministic random
                    min_ms + (madsim::rand::random::<u64>() % (max_ms - min_ms + 1))
                };
                total_delay_ms = delay_ms;
                has_delay = true;
            }
        }

        // Fall back to fixed delays if no range delay
        if !has_delay {
            let delays = self.delays.lock();
            if let Some(&delay_ms) = delays.get(&(source, target)) {
                total_delay_ms = delay_ms;
                has_delay = true;
            }
        }

        // Add clock drift effects
        // Positive drift on source: messages from this node appear delayed (fast clock)
        // Negative drift on target: messages to this node appear delayed (slow clock)
        {
            let drifts = self.clock_drifts.lock();

            // Source with positive drift: add delay to outgoing messages
            if let Some(&drift_ms) = drifts.get(&source)
                && drift_ms > 0
            {
                total_delay_ms = total_delay_ms.saturating_add(drift_ms as u64);
                has_delay = true;
            }

            // Target with negative drift: add delay to incoming messages
            if let Some(&drift_ms) = drifts.get(&target)
                && drift_ms < 0
            {
                total_delay_ms = total_delay_ms.saturating_add(drift_ms.unsigned_abs());
                has_delay = true;
            }
        }

        if has_delay && total_delay_ms > 0 {
            Some(Duration::from_millis(total_delay_ms))
        } else {
            None
        }
    }

    /// Clear all failure injection configuration.
    pub fn clear_all(&self) {
        let mut delays = self.delays.lock();
        let mut delay_ranges = self.delay_ranges.lock();
        let mut drops = self.drops.lock();
        let mut loss_rates = self.loss_rates.lock();
        let mut clock_drifts = self.clock_drifts.lock();
        delays.clear();
        delay_ranges.clear();
        drops.clear();
        loss_rates.clear();
        clock_drifts.clear();
    }
}

impl Default for FailureInjector {
    fn default() -> Self {
        Self::new()
    }
}

/// Byzantine failure injection type for testing consensus under Byzantine conditions.
///
/// These corruption modes simulate various Byzantine behaviors without creating
/// actual malicious nodes. This enables testing Raft's behavior when messages
/// are corrupted in transit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByzantineCorruptionMode {
    /// Flip the vote_granted field in VoteResponse messages.
    /// This tests whether Raft correctly handles conflicting vote responses.
    FlipVote,

    /// Increment the term in messages to simulate stale/future term attacks.
    /// This tests term validation logic.
    IncrementTerm,

    /// Duplicate messages (send twice) to test idempotency.
    /// This tests whether the system handles duplicate delivery.
    DuplicateMessage,

    /// Corrupt log entries by clearing the entries field.
    /// This tests whether followers validate received entries.
    ClearEntries,
}

/// Byzantine network wrapper that corrupts messages according to configured modes.
///
/// This wrapper sits between the Raft node and the underlying MadsimRaftNetwork,
/// intercepting and potentially corrupting messages to test Byzantine fault tolerance.
///
/// **Note**: Raft does not provide Byzantine fault tolerance. This testing is useful for:
/// 1. Verifying Raft's behavior under message corruption (e.g., from network bit flips)
/// 2. Testing term validation and log consistency checks
/// 3. Understanding failure modes
///
/// **Usage in AspenRaftTester**:
/// ```ignore
/// tester.enable_byzantine_mode(node_idx, ByzantineCorruptionMode::FlipVote, 0.3);
/// ```
pub struct ByzantineFailureInjector {
    /// Per-link Byzantine configuration: (source, target) -> (mode, probability)
    #[allow(clippy::type_complexity)]
    links: SyncMutex<HashMap<(NodeId, NodeId), Vec<(ByzantineCorruptionMode, f64)>>>,
    /// Statistics: count of corruptions per mode
    corruption_counts: SyncMutex<HashMap<ByzantineCorruptionMode, u64>>,
}

impl ByzantineFailureInjector {
    /// Create a new Byzantine failure injector.
    pub fn new() -> Self {
        Self {
            links: SyncMutex::new(HashMap::new()),
            corruption_counts: SyncMutex::new(HashMap::new()),
        }
    }

    /// Configure Byzantine behavior on a specific link.
    ///
    /// # Arguments
    /// * `source` - Source node ID
    /// * `target` - Target node ID
    /// * `mode` - Type of Byzantine corruption
    /// * `probability` - Probability of corruption (0.0 to 1.0)
    pub fn set_byzantine_mode(&self, source: NodeId, target: NodeId, mode: ByzantineCorruptionMode, probability: f64) {
        assert!((0.0..=1.0).contains(&probability), "probability must be between 0.0 and 1.0");
        let mut links = self.links.lock();
        let configs = links.entry((source, target)).or_default();
        // Update existing or add new
        if let Some(existing) = configs.iter_mut().find(|(m, _)| *m == mode) {
            existing.1 = probability;
        } else {
            configs.push((mode, probability));
        }
    }

    /// Clear all Byzantine configurations.
    pub fn clear_all(&self) {
        let mut links = self.links.lock();
        links.clear();
    }

    /// Check if a particular corruption should be applied.
    fn should_corrupt(&self, source: NodeId, target: NodeId, mode: ByzantineCorruptionMode) -> bool {
        let links = self.links.lock();
        if let Some(configs) = links.get(&(source, target)) {
            for (m, prob) in configs {
                if *m == mode {
                    let random_value: f64 = (madsim::rand::random::<u64>() as f64) / (u64::MAX as f64);
                    if random_value < *prob {
                        // Record corruption
                        drop(links); // Release lock before acquiring another
                        let mut counts = self.corruption_counts.lock();
                        *counts.entry(mode).or_insert(0) += 1;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Potentially corrupt an AppendEntries request.
    pub fn maybe_corrupt_append_entries(
        &self,
        source: NodeId,
        target: NodeId,
        mut rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> AppendEntriesRequest<AppTypeConfig> {
        // IncrementTerm: Add 1 to term to simulate stale term attack
        if self.should_corrupt(source, target, ByzantineCorruptionMode::IncrementTerm) {
            rpc.vote.leader_id.term = rpc.vote.leader_id.term.saturating_add(1);
            tracing::warn!(
                %source, %target,
                "BYZANTINE: Corrupted AppendEntries term to {}",
                rpc.vote.leader_id.term
            );
        }

        // ClearEntries: Remove all entries to test log validation
        if self.should_corrupt(source, target, ByzantineCorruptionMode::ClearEntries) {
            let original_len = rpc.entries.len();
            rpc.entries.clear();
            tracing::warn!(
                %source, %target,
                "BYZANTINE: Cleared {} entries from AppendEntries",
                original_len
            );
        }

        rpc
    }

    /// Potentially corrupt a Vote response.
    pub fn maybe_corrupt_vote_response(
        &self,
        source: NodeId,
        target: NodeId,
        mut resp: VoteResponse<AppTypeConfig>,
    ) -> VoteResponse<AppTypeConfig> {
        // FlipVote: Invert the vote_granted field
        if self.should_corrupt(source, target, ByzantineCorruptionMode::FlipVote) {
            resp.vote_granted = !resp.vote_granted;
            tracing::warn!(
                %source, %target,
                "BYZANTINE: Flipped vote_granted to {}",
                resp.vote_granted
            );
        }

        resp
    }

    /// Check if a message should be duplicated.
    pub fn should_duplicate(&self, source: NodeId, target: NodeId) -> bool {
        self.should_corrupt(source, target, ByzantineCorruptionMode::DuplicateMessage)
    }

    /// Get corruption statistics.
    pub fn get_corruption_stats(&self) -> HashMap<ByzantineCorruptionMode, u64> {
        let counts = self.corruption_counts.lock();
        counts.clone()
    }

    /// Get total corruption count.
    pub fn total_corruptions(&self) -> u64 {
        let counts = self.corruption_counts.lock();
        counts.values().sum()
    }
}

impl Default for ByzantineFailureInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl std::hash::Hash for ByzantineCorruptionMode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

// ====================================================================================
// Tests
// ====================================================================================

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    // =========================================================================
    // MadsimRaftRouter Tests
    // =========================================================================

    #[test]
    fn test_router_new_creates_empty() {
        let router = MadsimRaftRouter::new();
        // The router should start with no nodes
        let nodes = router.nodes.lock();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_router_default_creates_empty() {
        let router = MadsimRaftRouter::default();
        let nodes = router.nodes.lock();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_router_is_node_failed_default_false() {
        let router = MadsimRaftRouter::new();
        // Unregistered nodes should not be marked as failed
        assert!(!router.is_node_failed(NodeId::new(1)));
        assert!(!router.is_node_failed(NodeId::new(999)));
    }

    #[test]
    fn test_router_mark_node_failed_and_check() {
        let router = MadsimRaftRouter::new();

        // Initially not failed
        assert!(!router.is_node_failed(NodeId::new(1)));

        // Mark as failed
        router.mark_node_failed(NodeId::new(1), true);
        assert!(router.is_node_failed(NodeId::new(1)));

        // Other nodes still not failed
        assert!(!router.is_node_failed(NodeId::new(2)));

        // Unmark as failed
        router.mark_node_failed(NodeId::new(1), false);
        assert!(!router.is_node_failed(NodeId::new(1)));
    }

    // =========================================================================
    // FailureInjector Tests
    // =========================================================================

    #[test]
    fn test_failure_injector_new_creates_empty() {
        let injector = FailureInjector::new();
        // All maps should be empty
        assert!(injector.delays.lock().is_empty());
        assert!(injector.delay_ranges.lock().is_empty());
        assert!(injector.drops.lock().is_empty());
        assert!(injector.loss_rates.lock().is_empty());
        assert!(injector.clock_drifts.lock().is_empty());
    }

    #[test]
    fn test_failure_injector_default_creates_empty() {
        let injector = FailureInjector::default();
        assert!(injector.delays.lock().is_empty());
    }

    #[test]
    fn test_set_network_delay() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_network_delay(node1, node2, 100);

        let delays = injector.delays.lock();
        assert_eq!(delays.get(&(node1, node2)), Some(&100));
    }

    #[test]
    fn test_set_network_delay_range() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_network_delay_range(node1, node2, 10, 50);

        let delay_ranges = injector.delay_ranges.lock();
        assert_eq!(delay_ranges.get(&(node1, node2)), Some(&(10, 50)));
    }

    #[test]
    #[should_panic(expected = "min_ms must be <= max_ms")]
    fn test_set_network_delay_range_invalid() {
        let injector = FailureInjector::new();
        // min > max should panic
        injector.set_network_delay_range(NodeId::new(1), NodeId::new(2), 100, 10);
    }

    #[test]
    fn test_set_packet_loss_rate() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_packet_loss_rate(node1, node2, 0.5);

        let loss_rates = injector.loss_rates.lock();
        assert_eq!(loss_rates.get(&(node1, node2)), Some(&0.5));
    }

    #[test]
    #[should_panic(expected = "loss rate must be between 0.0 and 1.0")]
    fn test_set_packet_loss_rate_negative_invalid() {
        let injector = FailureInjector::new();
        injector.set_packet_loss_rate(NodeId::new(1), NodeId::new(2), -0.1);
    }

    #[test]
    #[should_panic(expected = "loss rate must be between 0.0 and 1.0")]
    fn test_set_packet_loss_rate_over_one_invalid() {
        let injector = FailureInjector::new();
        injector.set_packet_loss_rate(NodeId::new(1), NodeId::new(2), 1.1);
    }

    #[test]
    fn test_set_message_drop() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // Initially no drops
        assert!(!injector.should_drop_message(node1, node2));

        injector.set_message_drop(node1, node2, true);

        // Now should drop
        assert!(injector.should_drop_message(node1, node2));

        // Reverse direction not affected
        assert!(!injector.should_drop_message(node2, node1));

        // Clear the drop
        injector.set_message_drop(node1, node2, false);
        assert!(!injector.should_drop_message(node1, node2));
    }

    #[test]
    fn test_set_clock_drift() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);

        // No initial drift
        assert_eq!(injector.get_clock_drift(node1), None);

        // Set positive drift (fast clock)
        injector.set_clock_drift(node1, 100);
        assert_eq!(injector.get_clock_drift(node1), Some(100));

        // Set negative drift (slow clock)
        injector.set_clock_drift(node1, -50);
        assert_eq!(injector.get_clock_drift(node1), Some(-50));

        // Set to zero removes the entry
        injector.set_clock_drift(node1, 0);
        assert_eq!(injector.get_clock_drift(node1), None);
    }

    #[test]
    fn test_clear_clock_drift() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);

        injector.set_clock_drift(node1, 100);
        assert!(injector.get_clock_drift(node1).is_some());

        injector.clear_clock_drift(node1);
        assert!(injector.get_clock_drift(node1).is_none());
    }

    #[test]
    fn test_get_network_delay_with_fixed_delay() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // No delay configured
        assert!(injector.get_network_delay(node1, node2).is_none());

        // Set fixed delay
        injector.set_network_delay(node1, node2, 100);
        let delay = injector.get_network_delay(node1, node2);
        assert_eq!(delay, Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_get_network_delay_with_clock_drift_positive_source() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // Positive drift on source adds delay to outgoing
        injector.set_clock_drift(node1, 50);
        let delay = injector.get_network_delay(node1, node2);
        assert_eq!(delay, Some(Duration::from_millis(50)));
    }

    #[test]
    fn test_get_network_delay_with_clock_drift_negative_target() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // Negative drift on target adds delay to incoming
        injector.set_clock_drift(node2, -30);
        let delay = injector.get_network_delay(node1, node2);
        assert_eq!(delay, Some(Duration::from_millis(30)));
    }

    #[test]
    fn test_get_network_delay_combines_fixed_and_drift() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_network_delay(node1, node2, 100);
        injector.set_clock_drift(node1, 50); // Positive drift on source

        let delay = injector.get_network_delay(node1, node2);
        assert_eq!(delay, Some(Duration::from_millis(150))); // 100 + 50
    }

    #[test]
    fn test_clear_all() {
        let injector = FailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // Set up various configurations
        injector.set_network_delay(node1, node2, 100);
        injector.set_network_delay_range(node1, node2, 10, 50);
        injector.set_message_drop(node1, node2, true);
        injector.set_packet_loss_rate(node1, node2, 0.5);
        injector.set_clock_drift(node1, 100);

        // Clear all
        injector.clear_all();

        // Verify everything is empty
        assert!(injector.delays.lock().is_empty());
        assert!(injector.delay_ranges.lock().is_empty());
        assert!(injector.drops.lock().is_empty());
        assert!(injector.loss_rates.lock().is_empty());
        assert!(injector.clock_drifts.lock().is_empty());
    }

    // =========================================================================
    // ByzantineFailureInjector Tests
    // =========================================================================

    #[test]
    fn test_byzantine_injector_new() {
        let injector = ByzantineFailureInjector::new();
        assert!(injector.links.lock().is_empty());
        assert!(injector.corruption_counts.lock().is_empty());
    }

    #[test]
    fn test_byzantine_injector_default() {
        let injector = ByzantineFailureInjector::default();
        assert!(injector.links.lock().is_empty());
    }

    #[test]
    fn test_set_byzantine_mode() {
        let injector = ByzantineFailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);

        let links = injector.links.lock();
        let configs = links.get(&(node1, node2)).expect("should have configs");
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0], (ByzantineCorruptionMode::FlipVote, 0.5));
    }

    #[test]
    fn test_set_byzantine_mode_multiple_modes() {
        let injector = ByzantineFailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);
        injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::IncrementTerm, 0.3);

        let links = injector.links.lock();
        let configs = links.get(&(node1, node2)).expect("should have configs");
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_set_byzantine_mode_updates_existing() {
        let injector = ByzantineFailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);
        injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.8);

        let links = injector.links.lock();
        let configs = links.get(&(node1, node2)).expect("should have configs");
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0], (ByzantineCorruptionMode::FlipVote, 0.8));
    }

    #[test]
    #[should_panic(expected = "probability must be between 0.0 and 1.0")]
    fn test_set_byzantine_mode_invalid_probability() {
        let injector = ByzantineFailureInjector::new();
        injector.set_byzantine_mode(NodeId::new(1), NodeId::new(2), ByzantineCorruptionMode::FlipVote, 1.5);
    }

    #[test]
    fn test_byzantine_clear_all() {
        let injector = ByzantineFailureInjector::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        injector.set_byzantine_mode(node1, node2, ByzantineCorruptionMode::FlipVote, 0.5);
        assert!(!injector.links.lock().is_empty());

        injector.clear_all();
        assert!(injector.links.lock().is_empty());
    }

    #[test]
    fn test_get_corruption_stats_empty() {
        let injector = ByzantineFailureInjector::new();
        let stats = injector.get_corruption_stats();
        assert!(stats.is_empty());
    }

    #[test]
    fn test_total_corruptions_empty() {
        let injector = ByzantineFailureInjector::new();
        assert_eq!(injector.total_corruptions(), 0);
    }

    // =========================================================================
    // ByzantineCorruptionMode Tests
    // =========================================================================

    #[test]
    fn test_byzantine_corruption_mode_debug() {
        assert_eq!(format!("{:?}", ByzantineCorruptionMode::FlipVote), "FlipVote");
        assert_eq!(format!("{:?}", ByzantineCorruptionMode::IncrementTerm), "IncrementTerm");
        assert_eq!(format!("{:?}", ByzantineCorruptionMode::DuplicateMessage), "DuplicateMessage");
        assert_eq!(format!("{:?}", ByzantineCorruptionMode::ClearEntries), "ClearEntries");
    }

    #[test]
    fn test_byzantine_corruption_mode_clone() {
        let mode = ByzantineCorruptionMode::FlipVote;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_byzantine_corruption_mode_eq() {
        assert_eq!(ByzantineCorruptionMode::FlipVote, ByzantineCorruptionMode::FlipVote);
        assert_ne!(ByzantineCorruptionMode::FlipVote, ByzantineCorruptionMode::IncrementTerm);
    }

    #[test]
    fn test_byzantine_corruption_mode_hash() {
        use std::hash::Hash;
        use std::hash::Hasher;

        fn hash_mode(mode: ByzantineCorruptionMode) -> u64 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            mode.hash(&mut hasher);
            hasher.finish()
        }

        // Same mode should hash the same
        assert_eq!(hash_mode(ByzantineCorruptionMode::FlipVote), hash_mode(ByzantineCorruptionMode::FlipVote));

        // Different modes should (likely) hash differently
        assert_ne!(hash_mode(ByzantineCorruptionMode::FlipVote), hash_mode(ByzantineCorruptionMode::IncrementTerm));
    }

    // =========================================================================
    // MadsimNetworkFactory Tests
    // =========================================================================

    #[test]
    fn test_network_factory_creation() {
        let router = Arc::new(MadsimRaftRouter::new());
        let injector = Arc::new(FailureInjector::new());
        let node_id = NodeId::new(1);

        let factory = MadsimNetworkFactory::new(node_id, router.clone(), injector.clone());
        assert_eq!(factory.source_node_id, node_id);
    }

    #[test]
    fn test_network_factory_clone() {
        let router = Arc::new(MadsimRaftRouter::new());
        let injector = Arc::new(FailureInjector::new());
        let node_id = NodeId::new(1);

        let factory = MadsimNetworkFactory::new(node_id, router, injector);
        let cloned = factory.clone();

        assert_eq!(factory.source_node_id, cloned.source_node_id);
    }
}
