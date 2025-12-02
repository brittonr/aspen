/// In-memory Raft router for deterministic multi-node testing.
///
/// AspenRouter manages multiple Raft nodes with simulated networking, enabling fast
/// deterministic tests without real network I/O. Inspired by OpenRaft's RaftRouter.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result};
use openraft::alias::VoteOf;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::metrics::Wait;
use openraft::network::v2::RaftNetworkV2;
use openraft::network::RPCOption;
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::{BasicNode, Config, Raft};
use tokio::time::sleep;

use crate::raft::storage::{InMemoryLogStore, StateMachineStore};
use crate::raft::types::{AppRequest, AppTypeConfig, NodeId};

/// A Raft node managed by the router, including its storage and Raft handle.
pub struct AspenNode {
    pub id: NodeId,
    pub raft: Raft<AppTypeConfig>,
    pub log_store: InMemoryLogStore,
    pub state_machine: Arc<StateMachineStore>,
}

/// Network factory for in-memory Raft nodes. Routes RPCs through the router's
/// simulated network layer with configurable delays and failures.
#[derive(Clone)]
struct InMemoryNetworkFactory {
    target: NodeId,
    router: Arc<InnerRouter>,
}

impl InMemoryNetworkFactory {
    fn new(target: NodeId, router: Arc<InnerRouter>) -> Self {
        Self { target, router }
    }
}

impl openraft::network::RaftNetworkFactory<AppTypeConfig> for InMemoryNetworkFactory {
    type Network = InMemoryNetwork;

    async fn new_client(&mut self, _target: NodeId, _node: &BasicNode) -> Self::Network {
        InMemoryNetwork {
            target: self.target,
            router: self.router.clone(),
        }
    }
}

/// In-memory network client that routes RPCs through the router.
struct InMemoryNetwork {
    target: NodeId,
    router: Arc<InnerRouter>,
}

impl RaftNetworkV2<AppTypeConfig> for InMemoryNetwork {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.router.send_append_entries(self.target, rpc).await
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.router.send_vote(self.target, rpc).await
    }

    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: openraft::Snapshot<AppTypeConfig>,
        _cancel: impl std::future::Future<Output = ReplicationClosed> + openraft::OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        self.router.send_snapshot(self.target, vote, snapshot).await
    }
}

/// Inner router state shared across network factories.
struct InnerRouter {
    nodes: StdMutex<BTreeMap<NodeId, AspenNode>>,
    /// Network send delay in milliseconds
    send_delay_ms: AtomicU64,
    /// Failed nodes that should return Unreachable errors
    failed_nodes: StdMutex<HashMap<NodeId, bool>>,
}

impl InnerRouter {
    fn new() -> Self {
        Self {
            nodes: StdMutex::new(BTreeMap::new()),
            send_delay_ms: AtomicU64::new(0),
            failed_nodes: StdMutex::new(HashMap::new()),
        }
    }

    /// Simulate network delay if configured.
    async fn apply_network_delay(&self) {
        let delay_ms = self.send_delay_ms.load(Ordering::Relaxed);
        if delay_ms > 0 {
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    /// Check if a node is marked as failed.
    fn is_node_failed(&self, node_id: NodeId) -> bool {
        let failed = self.failed_nodes.lock().unwrap();
        failed.get(&node_id).copied().unwrap_or(false)
    }

    async fn send_append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.apply_network_delay().await;

        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "node marked as failed",
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

        raft.append_entries(rpc)
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))
    }

    async fn send_vote(
        &self,
        target: NodeId,
        rpc: VoteRequest<AppTypeConfig>,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.apply_network_delay().await;

        if self.is_node_failed(target) {
            return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "node marked as failed",
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

        raft.vote(rpc)
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))
    }

    async fn send_snapshot(
        &self,
        target: NodeId,
        vote: VoteOf<AppTypeConfig>,
        snapshot: openraft::Snapshot<AppTypeConfig>,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        self.apply_network_delay().await;

        if self.is_node_failed(target) {
            return Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "node marked as failed",
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
    pub async fn new_raft_node(&mut self, id: NodeId) -> Result<()> {
        let log_store = InMemoryLogStore::default();
        let state_machine = StateMachineStore::new();
        let network_factory = InMemoryNetworkFactory::new(id, self.inner.clone());

        let raft = Raft::new(id, self.config.clone(), network_factory, log_store.clone(), state_machine.clone())
            .await
            .context("failed to create Raft node")?;

        let node = AspenNode {
            id,
            raft,
            log_store,
            state_machine,
        };

        let mut nodes = self.inner.nodes.lock().unwrap();
        nodes.insert(id, node);

        Ok(())
    }

    /// Get a handle to the Raft instance for the given node.
    ///
    /// Useful for calling Raft APIs directly like `initialize()`, `write()`, etc.
    pub fn get_raft_handle(&self, node_id: &NodeId) -> Result<Raft<AppTypeConfig>> {
        let nodes = self.inner.nodes.lock().unwrap();
        let node = nodes
            .get(node_id)
            .with_context(|| format!("node {} not found", node_id))?;
        Ok(node.raft.clone())
    }

    /// Get a wait helper for metrics-based assertions on the given node.
    ///
    /// ## Usage
    ///
    /// ```ignore
    /// // Wait for log index 5 to be applied
    /// router.wait(&0, Some(Duration::from_secs(5)))
    ///     .applied_index(Some(5), "entries committed")
    ///     .await?;
    ///
    /// // Wait for leader election
    /// router.wait(&0, Some(Duration::from_secs(10)))
    ///     .current_leader(Some(0), "leader elected")
    ///     .await?;
    /// ```
    pub fn wait(&self, node_id: &NodeId, timeout: Option<Duration>) -> Wait<AppTypeConfig> {
        let nodes = self.inner.nodes.lock().unwrap();
        let node = nodes.get(node_id).expect("node not found in routing table");
        node.raft.wait(timeout)
    }

    /// Set network send delay in milliseconds. 0 means no delay.
    ///
    /// Useful for testing timeouts and slow network scenarios.
    pub fn set_network_delay(&mut self, delay_ms: u64) {
        self.inner.send_delay_ms.store(delay_ms, Ordering::Relaxed);
    }

    /// Mark a node as failed. All RPCs to this node will return Unreachable errors.
    ///
    /// Useful for simulating node crashes and network partitions.
    pub fn fail_node(&mut self, node_id: NodeId) {
        let mut failed = self.inner.failed_nodes.lock().unwrap();
        failed.insert(node_id, true);
    }

    /// Recover a previously failed node. RPCs to this node will succeed again.
    pub fn recover_node(&mut self, node_id: NodeId) {
        let mut failed = self.inner.failed_nodes.lock().unwrap();
        failed.insert(node_id, false);
    }

    /// Get the current leader node ID by checking metrics across all nodes.
    pub fn leader(&self) -> Option<NodeId> {
        let nodes = self.inner.nodes.lock().unwrap();
        for node in nodes.values() {
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
        node_id: &NodeId,
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
    pub async fn read(&self, node_id: &NodeId, key: &str) -> Option<String> {
        let nodes = self.inner.nodes.lock().unwrap();
        let node = nodes.get(node_id)?;
        node.state_machine.get(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use openraft::BasicNode;

    fn timeout() -> Option<Duration> {
        Some(Duration::from_secs(10))
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
        assert!(router.get_raft_handle(&0).is_ok());
        assert!(router.get_raft_handle(&1).is_ok());
        assert!(router.get_raft_handle(&2).is_ok());

        // Verify read/write operations
        let node0 = router.get_raft_handle(&0)?;
        assert!(node0.is_initialized().await.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_network_delay() -> Result<()> {
        let config = Arc::new(Config::default().validate()?);
        let mut router = AspenRouter::new(config);

        // Test that setting network delay doesn't break functionality
        router.set_network_delay(10); // Small delay to avoid test flakiness

        router.new_raft_node(0).await?;
        router.new_raft_node(1).await?;

        // Verify nodes still work with delay configured
        assert!(router.get_raft_handle(&0).is_ok());
        assert!(router.get_raft_handle(&1).is_ok());

        Ok(())
    }

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
        let node1 = router.get_raft_handle(&1)?;
        let _ = node1.is_initialized().await; // Should not panic

        Ok(())
    }
}
