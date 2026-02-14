//! ArcRaftNode wrapper for CoordinationBackend trait implementation.

use std::ops::Deref;
use std::sync::Arc;

use aspen_traits::ClusterController;
use aspen_traits::CoordinationBackend;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;

use super::RaftNode;

/// Wrapper for `Arc<RaftNode>` that implements `CoordinationBackend`.
///
/// This newtype is required because Rust's orphan rules prevent implementing
/// a foreign trait (`CoordinationBackend`) directly on `Arc<T>`. Use this wrapper
/// when you need to pass a `RaftNode` to coordination primitives.
///
/// ## Example
///
/// ```ignore
/// let node = Arc::new(RaftNode::new(...));
/// let backend = ArcRaftNode::from(node);
/// let lock = DistributedLock::new(backend, "my_lock", ...);
/// ```
#[derive(Clone)]
pub struct ArcRaftNode(pub Arc<RaftNode>);

impl Deref for ArcRaftNode {
    type Target = RaftNode;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Arc<RaftNode>> for ArcRaftNode {
    fn from(node: Arc<RaftNode>) -> Self {
        Self(node)
    }
}

impl ArcRaftNode {
    /// Get the inner `Arc<RaftNode>`.
    pub fn inner(&self) -> &Arc<RaftNode> {
        &self.0
    }

    /// Consume the wrapper and return the inner `Arc<RaftNode>`.
    pub fn into_inner(self) -> Arc<RaftNode> {
        self.0
    }
}

#[async_trait]
impl CoordinationBackend for ArcRaftNode {
    async fn now_unix_ms(&self) -> u64 {
        aspen_core::utils::current_time_ms()
    }

    async fn node_id(&self) -> u64 {
        self.0.node_id().0
    }

    async fn is_leader(&self) -> bool {
        let metrics = self.0.raft().metrics().borrow().clone();
        metrics.current_leader == Some(self.0.node_id())
    }

    fn kv_store(&self) -> Arc<dyn KeyValueStore> {
        self.0.clone() as Arc<dyn KeyValueStore>
    }

    fn cluster_controller(&self) -> Arc<dyn ClusterController> {
        self.0.clone() as Arc<dyn ClusterController>
    }
}
