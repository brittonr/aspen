/// In-memory Raft router for deterministic multi-node testing.
///
/// AspenRouter manages multiple Raft nodes with simulated networking, enabling fast
/// deterministic tests without real network I/O. Inspired by OpenRaft's RaftRouter.
///
/// # Test Coverage
///
/// The router has comprehensive unit tests covering:
/// - Router creation and node management
/// - Raft handle access and node removal
/// - Network delay injection (specific pairs and global)
/// - Message drop rate simulation (probabilistic packet loss)
/// - Node failure and recovery
/// - Leader detection and election
/// - Cluster initialization and learner addition
/// - Read/write operations and data replication
/// - RPC routing (append_entries, vote) with failure scenarios
/// - Helper function determinism
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use aspen_raft::storage::InMemoryLogStore;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::types::AppRequest;
use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use aspen_raft::types::RaftMemberInfo;
use openraft::Config;
use openraft::Raft;
use openraft::alias::VoteOf;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::metrics::Wait;
use openraft::network::RPCOption;
use openraft::network::v2::RaftNetworkV2;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use rand::Rng;
use tokio::time::sleep;

/// A Raft node managed by the router, including its storage and Raft handle.
/// Note: This is a test-specific wrapper around a Raft node, distinct from
/// `crate::raft::types::RaftMemberInfo` which represents node metadata in Raft membership.
pub struct TestNode {
    /// The node's unique identifier.
    pub id: NodeId,
    /// The Raft instance handle for this node.
    pub raft: Raft<AppTypeConfig>,
    /// The in-memory log store for this node.
    pub log_store: InMemoryLogStore,
    /// The in-memory state machine for this node.
    pub state_machine: Arc<InMemoryStateMachine>,
}

/// Network factory for in-memory Raft nodes. Routes RPCs through the router's
/// simulated network layer with configurable delays and failures.
#[derive(Clone)]
struct InMemoryNetworkFactory {
    source: NodeId,
    router: Arc<InnerRouter>,
}

impl InMemoryNetworkFactory {
    fn new(source: NodeId, router: Arc<InnerRouter>) -> Self {
        Self { source, router }
    }
}

impl openraft::network::RaftNetworkFactory<AppTypeConfig> for InMemoryNetworkFactory {
    type Network = InMemoryNetwork;

    async fn new_client(&mut self, target: NodeId, _node: &RaftMemberInfo) -> Self::Network {
        InMemoryNetwork {
            source: self.source,
            target,
            router: self.router.clone(),
        }
    }
}

/// In-memory network client that routes RPCs through the router.
struct InMemoryNetwork {
    source: NodeId,
    target: NodeId,
    router: Arc<InnerRouter>,
}

impl RaftNetworkV2<AppTypeConfig> for InMemoryNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.router.send_append_entries(self.source, self.target, rpc).await
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.router.send_vote(self.source, self.target, rpc).await
    }

    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: openraft::Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = ReplicationClosed> + openraft::OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        self.router.send_snapshot(self.source, self.target, vote, snapshot).await
    }
}

/// Inner router state shared across network factories.
struct InnerRouter {
    nodes: StdMutex<BTreeMap<NodeId, TestNode>>,
    /// Per-pair network delays in milliseconds: (source, target) -> delay_ms
    /// Enables simulating asymmetric network latencies between specific node pairs
    delays: StdMutex<HashMap<(NodeId, NodeId), u64>>,
    /// Per-pair message drop rates: (source, target) -> drop_rate (0-100)
    /// Enables probabilistic message dropping to simulate packet loss
    drop_rates: StdMutex<HashMap<(NodeId, NodeId), u32>>,
    /// Failed nodes that should return Unreachable errors
    failed_nodes: StdMutex<HashMap<NodeId, bool>>,
}

impl InnerRouter {
    fn new() -> Self {
        Self {
            nodes: StdMutex::new(BTreeMap::new()),
            delays: StdMutex::new(HashMap::new()),
            drop_rates: StdMutex::new(HashMap::new()),
            failed_nodes: StdMutex::new(HashMap::new()),
        }
    }

    /// Simulate network delay if configured for this source-target pair.
    async fn apply_network_delay(&self, source: NodeId, target: NodeId) {
        let delay_ms: Option<u64> = {
            let delays = self.delays.lock().unwrap();
            delays.get(&(source, target)).copied()
        }; // Lock is dropped here

        if let Some(ms) = delay_ms
            && ms > 0
        {
            sleep(Duration::from_millis(ms)).await;
        }
    }

    /// Check if a message should be dropped based on configured drop rate.
    /// Returns true if the message should be dropped (simulating packet loss).
    fn should_drop_message(&self, source: NodeId, target: NodeId) -> bool {
        let drop_rate: Option<u32> = {
            let drop_rates = self.drop_rates.lock().unwrap();
            drop_rates.get(&(source, target)).copied()
        }; // Lock is dropped here

        if let Some(rate) = drop_rate
            && rate > 0
            && rate <= 100
        {
            let random_value = rand::rng().random_range(0..100);
            return random_value < rate;
        }
        false
    }

    /// Check if a node is marked as failed.
    fn is_node_failed(&self, node_id: NodeId) -> bool {
        let failed = self.failed_nodes.lock().unwrap();
        failed.get(&node_id).copied().unwrap_or(false)
    }

    async fn send_append_entries(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.apply_network_delay(source, target).await;

        // Check if message should be dropped (simulating packet loss)
        if self.should_drop_message(source, target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "message dropped (simulated packet loss)",
            ))));
        }

        // Check if SOURCE node is failed (can't send if you're dead)
        if self.is_node_failed(source) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "source node marked as failed",
            ))));
        }

        // Check if TARGET node is failed (can't reach if they're dead)
        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "target node marked as failed",
            ))));
        }

        let raft = {
            let nodes = self.nodes.lock().unwrap();
            let node = nodes.get(&target).ok_or_else(|| {
                RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("target node {} not found", target),
                )))
            })?;
            node.raft.clone()
        };

        raft.append_entries(rpc).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))
    }

    async fn send_vote(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: VoteRequest<AppTypeConfig>,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.apply_network_delay(source, target).await;

        // Check if message should be dropped (simulating packet loss)
        if self.should_drop_message(source, target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "message dropped (simulated packet loss)",
            ))));
        }

        // Check if SOURCE node is failed (can't send if you're dead)
        if self.is_node_failed(source) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "source node marked as failed",
            ))));
        }

        // Check if TARGET node is failed (can't reach if they're dead)
        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "target node marked as failed",
            ))));
        }

        let raft = {
            let nodes = self.nodes.lock().unwrap();
            let node = nodes.get(&target).ok_or_else(|| {
                RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("target node {} not found", target),
                )))
            })?;
            node.raft.clone()
        };

        raft.vote(rpc).await.map_err(|e| RPCError::Network(NetworkError::new(&e)))
    }

    async fn send_snapshot(
        &self,
        source: NodeId,
        target: NodeId,
        vote: VoteOf<AppTypeConfig>,
        snapshot: openraft::Snapshot<AppTypeConfig>,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        self.apply_network_delay(source, target).await;

        // Check if message should be dropped (simulating packet loss)
        if self.should_drop_message(source, target) {
            return Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "message dropped (simulated packet loss)",
            ))));
        }

        // Check if SOURCE node is failed (can't send if you're dead)
        if self.is_node_failed(source) {
            return Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "source node marked as failed",
            ))));
        }

        // Check if TARGET node is failed (can't reach if they're dead)
        if self.is_node_failed(target) {
            return Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "target node marked as failed",
            ))));
        }

        let raft = {
            let nodes = self.nodes.lock().unwrap();
            let node = nodes.get(&target).ok_or_else(|| {
                StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("target node {} not found", target),
                )))
            })?;
            node.raft.clone()
        };

        raft.install_full_snapshot(vote, snapshot)
            .await
            .map_err(|e| StreamingError::Network(NetworkError::new(&e)))
    }
}

/// Router managing multiple in-memory Raft nodes for deterministic testing.
///
/// ## Network Simulation Features
///
/// - Configurable send delays via `set_network_delay()`
/// - Node failure simulation via `fail_node()` / `recover_node()`
/// - All RPCs routed through in-memory channels (no real network I/O)
///
/// ## Wait Helpers
///
/// Use `wait()` to get OpenRaft's `Wait` API for metrics-based assertions:
/// - `wait(&id, timeout).applied_index(Some(5), "msg")` - Wait for log application
/// - `wait(&id, timeout).current_leader(Some(0), "msg")` - Wait for leader election
/// - `wait(&id, timeout).state(ServerState::Leader, "msg")` - Wait for state change
///
/// ## Multi-Node Cluster Pattern
///
/// When creating multi-node clusters, follow OpenRaft's pattern:
/// 1. Create leader node first
/// 2. Initialize the leader
/// 3. Wait for leader to be ready (ServerState::Leader)
/// 4. Create other nodes
/// 5. Add them as learners via `add_learner()`
///
/// This ensures clean state transitions and avoids race conditions during replication setup.
pub struct AspenRouter {
    config: Arc<Config>,
    inner: Arc<InnerRouter>,
}

impl AspenRouter {
    /// Create a new router with the given Raft configuration.
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            inner: Arc::new(InnerRouter::new()),
        }
    }

    /// Create a new Raft node with the given ID and add it to the router.
    ///
    /// Returns a reference to the created node. The node starts in Learner state
    /// and must be initialized or added to a cluster via `initialize()` or membership changes.
    pub async fn new_raft_node(&mut self, id: impl Into<NodeId>) -> Result<()> {
        let log_store = InMemoryLogStore::default();
        let state_machine = InMemoryStateMachine::new();
        self.new_raft_node_with_storage(id.into(), log_store, state_machine).await
    }

    /// Create a new Raft node with custom storage.
    ///
    /// Useful for testing election logic, log comparison, and recovery scenarios
    /// where you need to pre-populate the log or state machine.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// let mut log_store = InMemoryLogStore::default();
    /// // Pre-populate log with entries
    /// log_store.append([...]).await?;
    ///
    /// let state_machine = InMemoryStateMachine::new();
    /// router.new_raft_node_with_storage(0, log_store, state_machine).await?;
    /// ```
    pub async fn new_raft_node_with_storage(
        &mut self,
        id: impl Into<NodeId>,
        log_store: InMemoryLogStore,
        state_machine: Arc<InMemoryStateMachine>,
    ) -> Result<()> {
        let id = id.into();
        let network_factory = InMemoryNetworkFactory::new(id, self.inner.clone());

        let raft = Raft::new(id, self.config.clone(), network_factory, log_store.clone(), state_machine.clone())
            .await
            .context("failed to create Raft node")?;

        let node = TestNode {
            id,
            raft,
            log_store,
            state_machine,
        };

        let mut nodes = self.inner.nodes.lock().unwrap();
        nodes.insert(id, node);

        Ok(())
    }

    /// Create a new storage pair (log store + state machine) for testing.
    ///
    /// Useful when you need to pre-populate storage before creating a node.
    pub fn new_store(&self) -> (InMemoryLogStore, Arc<InMemoryStateMachine>) {
        (InMemoryLogStore::default(), InMemoryStateMachine::new())
    }

    /// Get a handle to the Raft instance for the given node.
    ///
    /// Useful for calling Raft APIs directly like `initialize()`, `write()`, etc.
    pub fn get_raft_handle(&self, node_id: impl Into<NodeId>) -> Result<Raft<AppTypeConfig>> {
        let node_id = node_id.into();
        let nodes = self.inner.nodes.lock().unwrap();
        let node = nodes.get(&node_id).with_context(|| format!("node {} not found", node_id))?;
        Ok(node.raft.clone())
    }

    /// Get a wait helper for metrics-based assertions on the given node.
    ///
    /// ## Usage
    ///
    /// ```ignore
    /// // Wait for log index 5 to be applied
    /// router.wait(0, Some(Duration::from_secs(5)))
    ///     .applied_index(Some(5), "entries committed")
    ///     .await?;
    ///
    /// // Wait for leader election
    /// router.wait(0, Some(Duration::from_secs(10)))
    ///     .current_leader(Some(0), "leader elected")
    ///     .await?;
    /// ```
    pub fn wait(&self, node_id: impl Into<NodeId>, timeout: Option<Duration>) -> Wait<AppTypeConfig> {
        let node_id = node_id.into();
        let nodes = self.inner.nodes.lock().unwrap();
        let node = nodes.get(&node_id).expect("node not found in routing table");
        node.raft.wait(timeout)
    }

    /// Set network send delay in milliseconds for a specific source-target node pair.
    ///
    /// Enables asymmetric network latency testing. Delay of 0 means no delay.
    ///
    /// # Arguments
    /// * `source` - The node ID sending the RPC
    /// * `target` - The node ID receiving the RPC
    /// * `delay_ms` - Latency in milliseconds for this specific path
    ///
    /// # Example
    /// ```ignore
    /// // Node 1 → Node 2: 200ms latency
    /// router.set_network_delay(1, 2, 200);
    /// // Node 2 → Node 1: 50ms latency (asymmetric)
    /// router.set_network_delay(2, 1, 50);
    /// ```
    pub fn set_network_delay(&mut self, source: impl Into<NodeId>, target: impl Into<NodeId>, delay_ms: u64) {
        let mut delays = self.inner.delays.lock().unwrap();
        delays.insert((source.into(), target.into()), delay_ms);
    }

    /// Set global network delay for all node pairs. Convenience method.
    ///
    /// Applies the same latency to all source-target pairs in the cluster.
    /// Useful for testing uniform network degradation.
    ///
    /// # Arguments
    /// * `delay_ms` - Latency in milliseconds applied to all RPCs
    pub fn set_global_network_delay(&mut self, delay_ms: u64) {
        let nodes: Vec<NodeId> = {
            let nodes = self.inner.nodes.lock().unwrap();
            nodes.keys().copied().collect()
        };

        let mut delays = self.inner.delays.lock().unwrap();
        for &source in &nodes {
            for &target in &nodes {
                if source != target {
                    delays.insert((source, target), delay_ms);
                }
            }
        }
    }

    /// Clear all network delays.
    pub fn clear_network_delays(&mut self) {
        let mut delays = self.inner.delays.lock().unwrap();
        delays.clear();
    }

    /// Set probabilistic message drop rate for a specific source-target pair.
    ///
    /// Simulates packet loss by randomly dropping messages between two nodes.
    /// Real networks typically have 0.1-5% packet loss; tests often use 10-20%
    /// to stress Raft's retry mechanisms.
    ///
    /// # Arguments
    /// * `source` - Source node ID
    /// * `target` - Target node ID
    /// * `drop_rate` - Drop probability as percentage (0-100)
    ///   - 0 = no drops
    ///   - 10 = 10% of messages dropped
    ///   - 100 = all messages dropped
    ///
    /// # Example
    /// ```ignore
    /// // Simulate 20% packet loss from node 1 to node 2
    /// router.set_message_drop_rate(1, 2, 20);
    /// ```
    pub fn set_message_drop_rate(&mut self, source: impl Into<NodeId>, target: impl Into<NodeId>, drop_rate: u32) {
        let clamped_rate = drop_rate.min(100);
        let mut drop_rates = self.inner.drop_rates.lock().unwrap();
        drop_rates.insert((source.into(), target.into()), clamped_rate);
    }

    /// Set global message drop rate for all node pairs. Convenience method.
    ///
    /// Applies the same packet loss probability to all source-target pairs.
    /// Useful for testing uniform network unreliability.
    ///
    /// # Arguments
    /// * `drop_rate` - Drop probability as percentage (0-100)
    pub fn set_global_message_drop_rate(&mut self, drop_rate: u32) {
        let clamped_rate = drop_rate.min(100);
        let nodes: Vec<NodeId> = {
            let nodes = self.inner.nodes.lock().unwrap();
            nodes.keys().copied().collect()
        };

        let mut drop_rates = self.inner.drop_rates.lock().unwrap();
        for &source in &nodes {
            for &target in &nodes {
                if source != target {
                    drop_rates.insert((source, target), clamped_rate);
                }
            }
        }
    }

    /// Clear all message drop rates.
    pub fn clear_message_drop_rates(&mut self) {
        let mut drop_rates = self.inner.drop_rates.lock().unwrap();
        drop_rates.clear();
    }

    /// Mark a node as failed. All RPCs to this node will return Unreachable errors.
    ///
    /// Useful for simulating node crashes and network partitions.
    pub fn fail_node(&mut self, node_id: impl Into<NodeId>) {
        let mut failed = self.inner.failed_nodes.lock().unwrap();
        failed.insert(node_id.into(), true);
    }

    /// Recover a previously failed node. RPCs to this node will succeed again.
    pub fn recover_node(&mut self, node_id: impl Into<NodeId>) {
        let mut failed = self.inner.failed_nodes.lock().unwrap();
        failed.insert(node_id.into(), false);
    }

    /// Get the current leader node ID by checking metrics across all nodes.
    /// Skips nodes that are marked as failed. Checks server state to find actual leader.
    pub fn leader(&self) -> Option<NodeId> {
        let nodes = self.inner.nodes.lock().unwrap();
        let failed = self.inner.failed_nodes.lock().unwrap();

        // First, try to find a node that thinks it's the leader (ServerState::Leader)
        for node in nodes.values() {
            // Skip nodes marked as failed - they may have stale state
            if failed.get(&node.id).copied().unwrap_or(false) {
                continue;
            }

            let metrics = node.raft.metrics().borrow().clone();
            if metrics.state == openraft::ServerState::Leader {
                return Some(node.id);
            }
        }

        // Fallback: check if any node reports itself as current_leader
        // (in case metrics.state hasn't updated yet)
        for node in nodes.values() {
            if failed.get(&node.id).copied().unwrap_or(false) {
                continue;
            }

            let metrics = node.raft.metrics().borrow().clone();
            if metrics.current_leader == Some(node.id) {
                return Some(node.id);
            }
        }

        None
    }

    /// Write a value to the cluster via the given node.
    ///
    /// Returns an error if the node is not the leader. Use `leader()` to find
    /// the current leader first, or use `write_to_leader()` which handles routing.
    pub async fn write(
        &self,
        node_id: impl Into<NodeId>,
        key: String,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let raft = self.get_raft_handle(node_id)?;
        let request = AppRequest::Set { key, value };
        raft.client_write(request).await?;
        Ok(())
    }

    /// Read a value from the state machine of the given node.
    ///
    /// Note: This reads directly from the state machine without going through Raft,
    /// so it may return stale data if the node is behind. For linearizable reads,
    /// use the Raft read API.
    pub async fn read(&self, node_id: impl Into<NodeId>, key: &str) -> Option<String> {
        let node_id = node_id.into();
        let state_machine = {
            let nodes = self.inner.nodes.lock().unwrap();
            let node = nodes.get(&node_id)?;
            node.state_machine.clone() // Clone the Arc before awaiting
        }; // Lock released here
        state_machine.get(key).await
    }

    /// Initialize a single-node cluster with the given node as the leader.
    pub async fn initialize(&self, node_id: impl Into<NodeId>) -> Result<()> {
        use std::collections::BTreeMap;
        let node_id = node_id.into();

        let members: BTreeMap<NodeId, RaftMemberInfo> = {
            let nodes = self.inner.nodes.lock().unwrap();
            nodes.keys().map(|id| (*id, create_test_raft_member_info(*id))).collect()
        };

        let raft = self.get_raft_handle(node_id)?;
        raft.initialize(members).await?;
        Ok(())
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(&self, leader: impl Into<NodeId>, target: impl Into<NodeId>) -> Result<()> {
        let target = target.into();
        let raft = self.get_raft_handle(leader)?;
        raft.add_learner(target, create_test_raft_member_info(target), true)
            .await
            .map_err(|e| e.into_api_error().unwrap())?;
        Ok(())
    }

    /// Execute a callback with read-only access to the internal Raft state.
    pub async fn external_request<F>(&self, target: impl Into<NodeId>, req: F) -> Result<()>
    where F: FnOnce(&openraft::RaftState<AppTypeConfig>) + Send + 'static {
        let raft = self.get_raft_handle(target)?;
        raft.external_request(req).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(())
    }

    /// Create a new cluster with the given voters and learners.
    pub async fn new_cluster(
        &mut self,
        voter_ids: std::collections::BTreeSet<NodeId>,
        learners: std::collections::BTreeSet<NodeId>,
    ) -> Result<u64> {
        use openraft::ServerState;

        let leader_id: NodeId = 0.into();
        assert!(voter_ids.contains(&leader_id), "voter_ids must contain node 0");

        self.new_raft_node(leader_id).await?;
        self.wait(leader_id, Some(Duration::from_secs(10))).applied_index(None, "empty").await?;

        self.initialize(leader_id).await?;
        let mut log_index = 1_u64;

        self.wait(leader_id, Some(Duration::from_secs(10))).applied_index(Some(log_index), "init").await?;

        for id in voter_ids.iter() {
            if *id == leader_id {
                continue;
            }
            self.new_raft_node(*id).await?;
            self.add_learner(leader_id, *id).await?;
            log_index += 1;

            self.wait(*id, Some(Duration::from_secs(10))).state(ServerState::Learner, "empty node").await?;
        }

        for id in voter_ids.iter() {
            self.wait(*id, Some(Duration::from_secs(10)))
                .applied_index(Some(log_index), &format!("learners of {:?}", voter_ids))
                .await?;
        }

        if voter_ids.len() > 1 {
            let raft = self.get_raft_handle(leader_id)?;
            raft.change_membership(voter_ids.clone(), false).await?;
            log_index += 2;

            for id in voter_ids.iter() {
                self.wait(*id, Some(Duration::from_secs(10)))
                    .applied_index(Some(log_index), &format!("cluster of {:?}", voter_ids))
                    .await?;
            }
        }

        for id in learners.iter() {
            self.new_raft_node(*id).await?;
            self.add_learner(leader_id, *id).await?;
            log_index += 1;
        }

        for id in learners.iter() {
            self.wait(*id, Some(Duration::from_secs(10)))
                .applied_index(Some(log_index), &format!("learners of {:?}", learners))
                .await?;
        }

        Ok(log_index)
    }

    /// Remove a node from the router and return its Raft handle and storage.
    ///
    /// Useful for testing append_entries and other Raft APIs directly without
    /// going through the network layer.
    pub fn remove_node(
        &mut self,
        node_id: impl Into<NodeId>,
    ) -> Option<(Raft<AppTypeConfig>, InMemoryLogStore, Arc<InMemoryStateMachine>)> {
        let node_id = node_id.into();
        let mut nodes = self.inner.nodes.lock().unwrap();
        let node = nodes.remove(&node_id)?;
        Some((node.raft, node.log_store, node.state_machine))
    }
}

/// Create a test `RaftMemberInfo` with a deterministic Iroh address derived from the node ID.
///
/// This is used in the in-memory test router where we don't have real Iroh endpoints.
/// The address is deterministically generated from the node ID to ensure consistency.
fn create_test_raft_member_info(node_id: NodeId) -> RaftMemberInfo {
    use iroh::EndpointAddr;
    use iroh::EndpointId;
    use iroh::SecretKey;

    // Generate a deterministic secret key from the node ID
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&node_id.0.to_le_bytes());
    let secret_key = SecretKey::from(seed);
    let endpoint_id: EndpointId = secret_key.public();

    // Create an EndpointAddr with just the ID (no relay URLs or direct addresses for tests)
    let endpoint_addr = EndpointAddr::new(endpoint_id);

    RaftMemberInfo::new(endpoint_addr)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::Instant;

    use openraft::ServerState;

    use super::*;

    // =============================================================================
    // Router Creation Tests
    // =============================================================================

    #[tokio::test]
    async fn test_router_new_creates_empty_router() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        // No nodes yet
        assert!(router.get_raft_handle(0).is_err());
        assert!(router.leader().is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_router_basic_workflow() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        // Create nodes
        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;
        router.new_raft_node(2).await?;

        // Verify nodes were created
        assert!(router.get_raft_handle(0).is_ok());
        assert!(router.get_raft_handle(1).is_ok());
        assert!(router.get_raft_handle(2).is_ok());

        // Verify read/write operations
        let node0 = router.get_raft_handle(0)?;
        assert!(node0.is_initialized().await.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_new_raft_node_with_custom_storage() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        let (log_store, state_machine) = router.new_store();
        router.new_raft_node_with_storage(0_u64, log_store, state_machine).await?;

        // Verify node was created
        assert!(router.get_raft_handle(0).is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_new_store_creates_fresh_storage() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        let (log_store1, sm1) = router.new_store();
        let (log_store2, sm2) = router.new_store();

        // Each call returns new instances
        assert!(Arc::ptr_eq(&sm1, &sm1)); // Same instance
        assert!(!Arc::ptr_eq(&sm1, &sm2)); // Different instances

        // Log stores are default (empty)
        let _ = log_store1;
        let _ = log_store2;

        Ok(())
    }

    // =============================================================================
    // Node Handle Tests
    // =============================================================================

    #[tokio::test]
    async fn test_get_raft_handle_returns_valid_handle() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;

        let handle = router.get_raft_handle(0)?;
        // Handle should be functional
        let _ = handle.is_initialized().await;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_raft_handle_missing_node_returns_error() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        let result = router.get_raft_handle(99);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        Ok(())
    }

    #[tokio::test]
    async fn test_wait_returns_wait_handle() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;

        // wait() should return a valid Wait handle
        let wait = router.wait(0, Some(Duration::from_secs(1)));
        // Wait is usable (won't panic)
        let _ = wait;

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_node_returns_components() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;

        // Remove the node
        let removed = router.remove_node(0);
        assert!(removed.is_some());

        let (raft, _log_store, _state_machine) = removed.unwrap();
        // Raft handle is still functional
        let _ = raft.is_initialized().await;

        // Node is no longer in router
        assert!(router.get_raft_handle(0).is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_remove_node_missing_returns_none() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        let removed = router.remove_node(99);
        assert!(removed.is_none());

        Ok(())
    }

    // =============================================================================
    // Network Delay Tests
    // =============================================================================

    #[tokio::test]
    async fn test_network_delay() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Test that setting network delay doesn't break functionality
        router.set_global_network_delay(10); // Small delay to avoid test flakiness

        // Verify nodes still work with delay configured
        assert!(router.get_raft_handle(0).is_ok());
        assert!(router.get_raft_handle(1).is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_set_network_delay_specific_pair() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Set delay for specific direction
        router.set_network_delay(0, 1, 50);

        // Verify internal state by checking delays map
        let delays = router.inner.delays.lock().unwrap();
        assert_eq!(delays.get(&(0.into(), 1.into())), Some(&50));
        assert!(delays.get(&(1.into(), 0.into())).is_none()); // Asymmetric

        Ok(())
    }

    #[tokio::test]
    async fn test_set_global_network_delay_all_pairs() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;
        router.new_raft_node(2).await?;

        router.set_global_network_delay(100);

        let delays = router.inner.delays.lock().unwrap();
        // Should have delays for all pairs (excluding self-to-self)
        assert_eq!(delays.get(&(0.into(), 1.into())), Some(&100));
        assert_eq!(delays.get(&(0.into(), 2.into())), Some(&100));
        assert_eq!(delays.get(&(1.into(), 0.into())), Some(&100));
        assert_eq!(delays.get(&(1.into(), 2.into())), Some(&100));
        assert_eq!(delays.get(&(2.into(), 0.into())), Some(&100));
        assert_eq!(delays.get(&(2.into(), 1.into())), Some(&100));
        // No self-delays
        assert!(delays.get(&(0.into(), 0.into())).is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_clear_network_delays() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_global_network_delay(100);
        router.clear_network_delays();

        let delays = router.inner.delays.lock().unwrap();
        assert!(delays.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_apply_network_delay_actually_delays() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Set a measurable delay
        router.set_network_delay(0, 1, 50);

        let start = Instant::now();
        router.inner.apply_network_delay(0.into(), 1.into()).await;
        let elapsed = start.elapsed();

        // Should have delayed at least 50ms
        assert!(elapsed >= Duration::from_millis(45)); // Allow small variance

        Ok(())
    }

    #[tokio::test]
    async fn test_apply_network_delay_zero_is_noop() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Set zero delay
        router.set_network_delay(0, 1, 0);

        let start = Instant::now();
        router.inner.apply_network_delay(0.into(), 1.into()).await;
        let elapsed = start.elapsed();

        // Should be nearly instant
        assert!(elapsed < Duration::from_millis(10));

        Ok(())
    }

    #[tokio::test]
    async fn test_apply_network_delay_no_config_is_noop() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // No delay configured for this pair
        let start = Instant::now();
        router.inner.apply_network_delay(0.into(), 1.into()).await;
        let elapsed = start.elapsed();

        // Should be nearly instant
        assert!(elapsed < Duration::from_millis(10));

        Ok(())
    }

    // =============================================================================
    // Message Drop Rate Tests
    // =============================================================================

    #[tokio::test]
    async fn test_set_message_drop_rate_specific_pair() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_message_drop_rate(0, 1, 50);

        let drop_rates = router.inner.drop_rates.lock().unwrap();
        assert_eq!(drop_rates.get(&(0.into(), 1.into())), Some(&50));
        assert!(drop_rates.get(&(1.into(), 0.into())).is_none()); // Asymmetric

        Ok(())
    }

    #[tokio::test]
    async fn test_set_message_drop_rate_clamps_to_100() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_message_drop_rate(0, 1, 150); // Over 100

        let drop_rates = router.inner.drop_rates.lock().unwrap();
        assert_eq!(drop_rates.get(&(0.into(), 1.into())), Some(&100));

        Ok(())
    }

    #[tokio::test]
    async fn test_set_global_message_drop_rate_all_pairs() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;
        router.new_raft_node(2).await?;

        router.set_global_message_drop_rate(25);

        let drop_rates = router.inner.drop_rates.lock().unwrap();
        // Should have rates for all pairs (excluding self-to-self)
        assert_eq!(drop_rates.get(&(0.into(), 1.into())), Some(&25));
        assert_eq!(drop_rates.get(&(0.into(), 2.into())), Some(&25));
        assert_eq!(drop_rates.get(&(1.into(), 0.into())), Some(&25));

        Ok(())
    }

    #[tokio::test]
    async fn test_clear_message_drop_rates() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_global_message_drop_rate(50);
        router.clear_message_drop_rates();

        let drop_rates = router.inner.drop_rates.lock().unwrap();
        assert!(drop_rates.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_should_drop_message_zero_rate_never_drops() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_message_drop_rate(0, 1, 0);

        // With 0% rate, should never drop
        for _ in 0..100 {
            assert!(!router.inner.should_drop_message(0.into(), 1.into()));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_should_drop_message_hundred_rate_always_drops() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_message_drop_rate(0, 1, 100);

        // With 100% rate, should always drop
        for _ in 0..100 {
            assert!(router.inner.should_drop_message(0.into(), 1.into()));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_should_drop_message_no_config_never_drops() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // No drop rate configured - should never drop
        for _ in 0..100 {
            assert!(!router.inner.should_drop_message(0.into(), 1.into()));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_should_drop_message_probabilistic() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.set_message_drop_rate(0, 1, 50);

        // With 50% rate, roughly half should drop over many iterations
        let mut drops = 0;
        let iterations = 1000;
        for _ in 0..iterations {
            if router.inner.should_drop_message(0.into(), 1.into()) {
                drops += 1;
            }
        }

        // Should be roughly 50% (allow 20% variance for randomness)
        let drop_rate = (drops as f64) / (iterations as f64);
        assert!(drop_rate > 0.3 && drop_rate < 0.7);

        Ok(())
    }

    // =============================================================================
    // Node Failure Tests
    // =============================================================================

    #[tokio::test]
    async fn test_node_failure() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Mark node 1 as failed
        router.fail_node(1);

        // RPCs to node 1 should fail
        // (This would be tested by observing replication failures in a real scenario)

        // Recover node 1
        router.recover_node(1);

        // Node 1 should work again
        let node1 = router.get_raft_handle(1)?;
        let _ = node1.is_initialized().await; // Should not panic

        Ok(())
    }

    #[tokio::test]
    async fn test_fail_node_marks_as_failed() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;

        assert!(!router.inner.is_node_failed(0.into()));

        router.fail_node(0);

        assert!(router.inner.is_node_failed(0.into()));

        Ok(())
    }

    #[tokio::test]
    async fn test_recover_node_clears_failed_state() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;

        router.fail_node(0);
        assert!(router.inner.is_node_failed(0.into()));

        router.recover_node(0);
        assert!(!router.inner.is_node_failed(0.into()));

        Ok(())
    }

    #[tokio::test]
    async fn test_is_node_failed_unknown_node_returns_false() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        // Unknown node is not failed
        assert!(!router.inner.is_node_failed(99.into()));

        Ok(())
    }

    // =============================================================================
    // Leader Detection Tests
    // =============================================================================

    #[tokio::test]
    async fn test_leader_returns_none_for_empty_cluster() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        assert!(router.leader().is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_leader_returns_none_for_uninitialized_nodes() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Nodes not initialized, no leader
        assert!(router.leader().is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_leader_skips_failed_nodes() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Initialize to get a leader
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        assert_eq!(router.leader(), Some(0.into()));

        // Mark the leader as failed
        router.fail_node(0);

        // Should skip failed node
        assert!(router.leader().is_none());

        Ok(())
    }

    // =============================================================================
    // Cluster Initialization Tests
    // =============================================================================

    #[tokio::test]
    async fn test_initialize_creates_leader() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.initialize(0).await?;

        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        assert_eq!(router.leader(), Some(0.into()));

        Ok(())
    }

    #[tokio::test]
    async fn test_add_learner_adds_node_to_cluster() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        // Create and initialize node 0 first (as single-node cluster)
        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        // Now create node 1 and add it as a learner
        router.new_raft_node(1).await?;
        router.add_learner(0, 1).await?;

        // Node 1 should become a learner
        router.wait(1, Some(Duration::from_secs(5))).state(ServerState::Learner, "learner added").await?;

        Ok(())
    }

    // =============================================================================
    // Read/Write Tests
    // =============================================================================

    #[tokio::test]
    async fn test_write_to_leader_succeeds() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        router
            .write(0, "key1".to_string(), "value1".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_read_returns_written_value() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        router
            .write(0, "key1".to_string(), "value1".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Wait for write to be applied
        router.wait(0, Some(Duration::from_secs(5))).applied_index(Some(2), "write applied").await?;

        let value = router.read(0, "key1").await;
        assert_eq!(value, Some("value1".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_read_missing_key_returns_none() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        let value = router.read(0, "nonexistent").await;
        assert!(value.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_read_from_missing_node_returns_none() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        let value = router.read(99, "key").await;
        assert!(value.is_none());

        Ok(())
    }

    // =============================================================================
    // Multi-Node Cluster Tests
    // =============================================================================

    #[tokio::test]
    async fn test_new_cluster_creates_multi_node_cluster() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        let voters: BTreeSet<NodeId> = [0, 1, 2].into_iter().map(NodeId).collect();
        let learners: BTreeSet<NodeId> = BTreeSet::new();

        let log_index = router.new_cluster(voters.clone(), learners).await?;

        // Cluster should be initialized
        assert!(log_index > 0);

        // Leader should be elected (node 0)
        assert_eq!(router.leader(), Some(0.into()));

        // All voters should be present
        assert!(router.get_raft_handle(0).is_ok());
        assert!(router.get_raft_handle(1).is_ok());
        assert!(router.get_raft_handle(2).is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_new_cluster_with_learners() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        let voters: BTreeSet<NodeId> = [0, 1, 2].into_iter().map(NodeId).collect();
        let learners: BTreeSet<NodeId> = [3].into_iter().map(NodeId).collect();

        router.new_cluster(voters, learners).await?;

        // Learner should be added
        assert!(router.get_raft_handle(3).is_ok());
        router.wait(3, Some(Duration::from_secs(5))).state(ServerState::Learner, "learner added").await?;

        Ok(())
    }

    // =============================================================================
    // External Request Tests
    // =============================================================================

    #[tokio::test]
    async fn test_external_request_accesses_raft_state() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        // Use external_request to inspect state
        router
            .external_request(0, |state| {
                // Should be able to access state
                let _vote = state.vote_ref();
            })
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_external_request_missing_node_returns_error() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let router = AspenRouter::new(config);

        let result = router.external_request(99, |_state| {}).await;
        assert!(result.is_err());

        Ok(())
    }

    // =============================================================================
    // RPC Routing Tests (InnerRouter)
    // =============================================================================

    #[tokio::test]
    async fn test_send_append_entries_target_node_failed() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.fail_node(1);

        // Create a minimal append entries request
        let vote = openraft::Vote::new(1, 0.into());
        let rpc = AppendEntriesRequest {
            vote,
            prev_log_id: None,
            leader_commit: None,
            entries: vec![],
        };

        let result = router.inner.send_append_entries(0.into(), 1.into(), rpc).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(RPCError::Unreachable(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_send_append_entries_source_node_failed() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.fail_node(0); // Source is failed

        let vote = openraft::Vote::new(1, 0.into());
        let rpc = AppendEntriesRequest {
            vote,
            prev_log_id: None,
            leader_commit: None,
            entries: vec![],
        };

        let result = router.inner.send_append_entries(0.into(), 1.into(), rpc).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(RPCError::Unreachable(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_send_append_entries_target_not_found() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;

        let vote = openraft::Vote::new(1, 0.into());
        let rpc = AppendEntriesRequest {
            vote,
            prev_log_id: None,
            leader_commit: None,
            entries: vec![],
        };

        let result = router.inner.send_append_entries(0.into(), 99.into(), rpc).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(RPCError::Unreachable(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_send_vote_target_node_failed() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        router.fail_node(1);

        let vote = openraft::Vote::new(1, 0.into());
        let rpc = VoteRequest {
            vote,
            last_log_id: None,
        };

        let result = router.inner.send_vote(0.into(), 1.into(), rpc).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(RPCError::Unreachable(_))));

        Ok(())
    }

    #[tokio::test]
    async fn test_send_vote_with_message_drop() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // 100% drop rate
        router.set_message_drop_rate(0, 1, 100);

        let vote = openraft::Vote::new(1, 0.into());
        let rpc = VoteRequest {
            vote,
            last_log_id: None,
        };

        let result = router.inner.send_vote(0.into(), 1.into(), rpc).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(RPCError::Unreachable(_))));

        Ok(())
    }

    // =============================================================================
    // Helper Function Tests
    // =============================================================================

    #[test]
    fn test_create_test_raft_member_info_deterministic() {
        let info1 = create_test_raft_member_info(0.into());
        let info2 = create_test_raft_member_info(0.into());
        let info3 = create_test_raft_member_info(1.into());

        // Same node ID produces same info
        assert_eq!(info1.iroh_addr.id, info2.iroh_addr.id);

        // Different node IDs produce different info
        assert_ne!(info1.iroh_addr.id, info3.iroh_addr.id);
    }

    #[test]
    fn test_create_test_raft_member_info_valid() {
        let info = create_test_raft_member_info(42.into());

        // Should have a valid endpoint address
        let _id = info.iroh_addr.id; // Should not panic
    }

    // =============================================================================
    // Integration Tests
    // =============================================================================

    #[tokio::test]
    async fn test_data_replication_to_learner() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        // Create and initialize single-node cluster first
        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        // Now create node 1 and add it as a learner
        router.new_raft_node(1).await?;
        router.add_learner(0, 1).await?;
        router.wait(1, Some(Duration::from_secs(5))).state(ServerState::Learner, "learner added").await?;

        // Write data via leader
        router
            .write(0, "key".to_string(), "value".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Wait for replication to learner
        router.wait(1, Some(Duration::from_secs(5))).applied_index(Some(3), "write replicated").await?;

        // Read from learner (should have the data)
        let value = router.read(1, "key").await;
        assert_eq!(value, Some("value".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_network_delay_configuration() -> Result<()> {
        // Test that network delays are configured correctly - the actual delay
        // is tested in test_apply_network_delay_actually_delays
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Configure delay and verify it's stored
        router.set_network_delay(0, 1, 100);
        router.set_network_delay(1, 0, 50);

        // Verify delays are set asymmetrically
        let delays = router.inner.delays.lock().unwrap();
        assert_eq!(delays.get(&(0.into(), 1.into())), Some(&100));
        assert_eq!(delays.get(&(1.into(), 0.into())), Some(&50));

        Ok(())
    }

    #[tokio::test]
    async fn test_failed_node_blocks_replication() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        // Create and initialize single-node cluster first
        router.new_raft_node(0).await?;
        router.initialize(0).await?;
        router.wait(0, Some(Duration::from_secs(5))).state(ServerState::Leader, "leader elected").await?;

        // Now create node 1 and add it as a learner
        router.new_raft_node(1).await?;
        router.add_learner(0, 1).await?;
        router.wait(1, Some(Duration::from_secs(5))).state(ServerState::Learner, "learner added").await?;

        // Fail the learner
        router.fail_node(1);

        // Write should still succeed (leader doesn't need learner)
        router
            .write(0, "key".to_string(), "value".to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Leader should have the write applied
        router.wait(0, Some(Duration::from_secs(5))).applied_index(Some(3), "write applied").await?;

        Ok(())
    }
}
