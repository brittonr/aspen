/// Inner router state shared across network factories.
///
/// Manages simulated network conditions (delays, packet loss, node failures)
/// and routes RPCs between in-memory Raft nodes.
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use openraft::alias::VoteOf;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use rand::Rng;
use tokio::time::sleep;

use super::TestNode;

pub(crate) struct InnerRouter {
    pub(super) nodes: StdMutex<BTreeMap<NodeId, TestNode>>,
    /// Per-pair network delays in milliseconds: (source, target) -> delay_ms
    /// Enables simulating asymmetric network latencies between specific node pairs
    pub(super) delays: StdMutex<HashMap<(NodeId, NodeId), u64>>,
    /// Per-pair message drop rates: (source, target) -> drop_rate (0-100)
    /// Enables probabilistic message dropping to simulate packet loss
    pub(super) drop_rates: StdMutex<HashMap<(NodeId, NodeId), u32>>,
    /// Failed nodes that should return Unreachable errors
    pub(super) failed_nodes: StdMutex<HashMap<NodeId, bool>>,
}

impl InnerRouter {
    pub(crate) fn new() -> Self {
        Self {
            nodes: StdMutex::new(BTreeMap::new()),
            delays: StdMutex::new(HashMap::new()),
            drop_rates: StdMutex::new(HashMap::new()),
            failed_nodes: StdMutex::new(HashMap::new()),
        }
    }

    /// Simulate network delay if configured for this source-target pair.
    pub(crate) async fn apply_network_delay(&self, source: NodeId, target: NodeId) {
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
    pub(crate) fn should_drop_message(&self, source: NodeId, target: NodeId) -> bool {
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
    pub(crate) fn is_node_failed(&self, node_id: NodeId) -> bool {
        let failed = self.failed_nodes.lock().unwrap();
        failed.get(&node_id).copied().unwrap_or(false)
    }

    /// Check pre-send conditions: packet loss, source failure, target failure.
    /// Returns an error if the message should not be delivered.
    fn check_pre_send<E: FromUnreachable>(&self, source: NodeId, target: NodeId) -> Result<(), E> {
        if self.should_drop_message(source, target) {
            return Err(E::from_unreachable(std::io::ErrorKind::TimedOut, "message dropped (simulated packet loss)"));
        }

        if self.is_node_failed(source) {
            return Err(E::from_unreachable(std::io::ErrorKind::ConnectionAborted, "source node marked as failed"));
        }

        if self.is_node_failed(target) {
            return Err(E::from_unreachable(std::io::ErrorKind::ConnectionRefused, "target node marked as failed"));
        }

        Ok(())
    }

    pub(crate) async fn send_append_entries(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: AppendEntriesRequest<AppTypeConfig>,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.apply_network_delay(source, target).await;
        self.check_pre_send::<RPCError<AppTypeConfig>>(source, target)?;

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

    pub(crate) async fn send_vote(
        &self,
        source: NodeId,
        target: NodeId,
        rpc: VoteRequest<AppTypeConfig>,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        self.apply_network_delay(source, target).await;
        self.check_pre_send::<RPCError<AppTypeConfig>>(source, target)?;

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

    pub(crate) async fn send_snapshot(
        &self,
        source: NodeId,
        target: NodeId,
        vote: VoteOf<AppTypeConfig>,
        snapshot: openraft::Snapshot<AppTypeConfig>,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        self.apply_network_delay(source, target).await;
        self.check_pre_send::<StreamingError<AppTypeConfig>>(source, target)?;

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

/// Trait for constructing unreachable errors from different openraft error types.
trait FromUnreachable {
    fn from_unreachable(kind: std::io::ErrorKind, msg: &str) -> Self;
}

impl FromUnreachable for RPCError<AppTypeConfig> {
    fn from_unreachable(kind: std::io::ErrorKind, msg: &str) -> Self {
        RPCError::Unreachable(Unreachable::new(&std::io::Error::new(kind, msg)))
    }
}

impl FromUnreachable for StreamingError<AppTypeConfig> {
    fn from_unreachable(kind: std::io::ErrorKind, msg: &str) -> Self {
        StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(kind, msg)))
    }
}
