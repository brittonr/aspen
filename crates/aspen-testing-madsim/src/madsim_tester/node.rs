//! Test node types and helper functions.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use aspen_core::simulation::SimulationArtifactBuilder;
#[cfg(feature = "sql")]
use aspen_raft::node::RaftNode;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use aspen_raft::types::RaftMemberInfo;
use openraft::Raft;

/// Create a test `RaftMemberInfo` with a deterministic Iroh address derived from the node ID.
///
/// This is used in tests where we don't have real Iroh endpoints.
/// The address is deterministically generated from the node ID to ensure consistency.
pub fn create_test_raft_member_info(node_id: impl Into<NodeId>) -> RaftMemberInfo {
    use iroh::EndpointAddr;
    use iroh::EndpointId;
    use iroh::SecretKey;

    let node_id = node_id.into();
    // Generate a deterministic secret key from the node ID
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&node_id.0.to_le_bytes());
    let secret_key = SecretKey::from(seed);
    let endpoint_id: EndpointId = secret_key.public();

    // Create an EndpointAddr with just the ID (no relay URLs or direct addresses for tests)
    let endpoint_addr = EndpointAddr::new(endpoint_id);

    RaftMemberInfo::new(endpoint_addr)
}

/// Helper to create a fresh artifact builder for std::mem::replace
pub(crate) fn empty_artifact_builder() -> SimulationArtifactBuilder {
    SimulationArtifactBuilder::new("_placeholder_", 0)
}

/// Storage paths for a Redb node (for crash recovery testing).
#[derive(Clone, Debug)]
pub(crate) struct RedbStoragePath {
    /// Path to the shared Redb database file.
    pub(crate) db_path: std::path::PathBuf,
}

/// Node handle for tracking individual node state.
pub(crate) enum TestNode {
    /// In-memory node (for testing).
    InMemory {
        raft: Raft<AppTypeConfig>,
        state_machine: Arc<InMemoryStateMachine>,
        connected: AtomicBool,
    },
    /// Redb node (for SQL testing and crash recovery with single-fsync storage).
    Redb {
        raft: Raft<AppTypeConfig>,
        /// RaftNode wrapper for SqlQueryExecutor access (only used with sql feature).
        #[cfg(feature = "sql")]
        raft_node: Arc<RaftNode>,
        /// Shared Redb storage (implements both log and state machine).
        storage: Arc<SharedRedbStorage>,
        connected: AtomicBool,
        /// Storage path (for crash recovery testing).
        storage_path: RedbStoragePath,
    },
}

impl TestNode {
    pub(crate) fn raft(&self) -> &Raft<AppTypeConfig> {
        match self {
            TestNode::InMemory { raft, .. } => raft,
            TestNode::Redb { raft, .. } => raft,
        }
    }

    pub(crate) fn connected(&self) -> &AtomicBool {
        match self {
            TestNode::InMemory { connected, .. } => connected,
            TestNode::Redb { connected, .. } => connected,
        }
    }

    /// Get Redb storage path (for crash recovery testing).
    pub(crate) fn redb_storage_path(&self) -> Option<&RedbStoragePath> {
        match self {
            TestNode::InMemory { .. } => None,
            TestNode::Redb { storage_path, .. } => Some(storage_path),
        }
    }

    /// Get the RaftNode wrapper for SQL execution (only available for Redb nodes).
    #[cfg(feature = "sql")]
    pub(crate) fn raft_node(&self) -> Option<&Arc<RaftNode>> {
        match self {
            TestNode::InMemory { .. } => None,
            TestNode::Redb { raft_node, .. } => Some(raft_node),
        }
    }
}
