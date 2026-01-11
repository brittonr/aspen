//! Multi-node Pijul test harness.
//!
//! Provides infrastructure for testing Pijul P2P synchronization across
//! multiple Aspen nodes with real Iroh networking.
//!
//! This module is placed in tests/support/ to avoid circular dependencies.
//! The PijulMultiNodeTester needs access to aspen::node::Node which is defined
//! in the main aspen crate, so it can't be in aspen-testing (which is a
//! dev-dependency of aspen).
//!
//! # Example
//!
//! ```ignore
//! let mut tester = PijulMultiNodeTester::new(2).await?;
//! tester.init_cluster().await?;
//! tester.start_pijul_sync().await?;
//!
//! let repo_id = tester.create_repo(0, "test-repo").await?;
//! let hash = tester.record_change(0, &repo_id, "main", &[("hello.txt", "Hello!\n")], "Initial").await?;
//!
//! let synced = tester.wait_for_change(1, &repo_id, &hash, Duration::from_secs(30)).await?;
//! assert!(synced);
//!
//! tester.shutdown().await?;
//! ```

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen::api::AddLearnerRequest;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::ReadConsistency;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use aspen_blob::IrohBlobStore;
use aspen_forge::identity::RepoId;
use aspen_pijul::ChangeDirectory;
use aspen_pijul::ChangeHash;
use aspen_pijul::ChangeRecorder;
use aspen_pijul::PijulAnnouncement;
use aspen_pijul::PijulRepoIdentity;
use aspen_pijul::PijulStore;
use aspen_pijul::PijulSyncCallback;
use aspen_pijul::PijulSyncHandler;
use aspen_pijul::PijulSyncHandlerHandle;
use aspen_pijul::PijulSyncService;
use aspen_pijul::PristineManager;
use iroh::PublicKey;
use parking_lot::RwLock;
use tokio::time::sleep;
use tracing::info;

/// Deferred handler wrapper to break circular dependency.
///
/// The handler needs sync_service to send responses, but the service needs
/// the handler as a callback. This wrapper allows creating the service first,
/// then setting the real handler afterward.
struct DeferredSyncHandler {
    inner: RwLock<Option<Arc<PijulSyncHandler<IrohBlobStore, dyn KeyValueStore>>>>,
}

impl DeferredSyncHandler {
    fn new() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }

    fn set_handler(&self, handler: Arc<PijulSyncHandler<IrohBlobStore, dyn KeyValueStore>>) {
        *self.inner.write() = Some(handler);
    }
}

impl PijulSyncCallback for DeferredSyncHandler {
    fn on_announcement(&self, announcement: &PijulAnnouncement, signer: &PublicKey) {
        if let Some(ref handler) = *self.inner.read() {
            handler.on_announcement(announcement, signer);
        }
    }
}

/// Time to wait for gossip discovery between nodes.
const GOSSIP_DISCOVERY_WAIT: Duration = Duration::from_secs(5);
/// Time to wait for leader election.
const LEADER_ELECTION_WAIT: Duration = Duration::from_millis(1000);
/// Polling interval for wait operations.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Multi-node Pijul test harness.
///
/// Manages multiple Aspen nodes with Pijul sync enabled, providing helpers
/// for common test scenarios like creating repos, recording changes, and
/// verifying sync.
pub struct PijulMultiNodeTester {
    /// All nodes in the test cluster.
    nodes: Vec<PijulTestNode>,
    /// Base directory for all node data.
    base_dir: PathBuf,
    /// Test cluster cookie for gossip isolation.
    cookie: String,
}

/// A single node in the test cluster with Pijul components.
pub struct PijulTestNode {
    /// The Aspen node.
    node: Node,
    /// Pijul store for this node.
    store: Arc<PijulStore<IrohBlobStore, dyn KeyValueStore>>,
    /// Blob store (shared with PijulStore).
    blobs: Arc<IrohBlobStore>,
    /// Pristine manager for local pristine databases.
    pristine_mgr: PristineManager,
    /// Sync service (if started).
    sync_service: Option<Arc<PijulSyncService>>,
    /// Sync handler for processing announcements (if started).
    sync_handler: Option<Arc<PijulSyncHandler<IrohBlobStore, dyn KeyValueStore>>>,
    /// Sync handler worker handle (if started).
    handler_handle: Option<PijulSyncHandlerHandle>,
    /// Node index (0-based).
    index: usize,
}

impl PijulMultiNodeTester {
    /// Create a new multi-node test environment.
    ///
    /// Creates `node_count` Aspen nodes with gossip enabled but Pijul sync
    /// not yet started. Call `init_cluster()` and `start_pijul_sync()` to
    /// complete setup.
    ///
    /// # Arguments
    ///
    /// - `node_count`: Number of nodes to create
    /// - `base_dir`: Base directory for node data (caller should manage cleanup)
    pub async fn new(node_count: usize, base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir)?;
        let cookie = format!("pijul-test-{}", rand::random::<u32>());

        info!(node_count, cookie = %cookie, "creating Pijul multi-node tester");

        let mut nodes = Vec::with_capacity(node_count);
        for idx in 0..node_count {
            let node_id = (idx + 1) as u64;
            let secret_key = generate_secret_key(node_id);
            let data_dir = base_dir.join(format!("node-{}", node_id));

            let mut node = NodeBuilder::new(NodeId(node_id), &data_dir)
                .with_storage(StorageBackend::InMemory)
                .with_cookie(&cookie)
                .with_gossip(true)
                .with_mdns(false) // Disable mDNS for CI compatibility
                .with_iroh_secret_key(&secret_key)
                .with_heartbeat_interval_ms(500)
                .with_election_timeout_ms(1500, 3000)
                .start()
                .await
                .with_context(|| format!("failed to start node {}", node_id))?;

            // Create blob store with P2P support BEFORE spawning router
            // so we can register the blobs protocol handler
            let blob_dir = data_dir.join("blobs");
            std::fs::create_dir_all(&blob_dir)?;
            let endpoint = node.handle().network.iroh_manager.endpoint().clone();
            let blobs = Arc::new(
                IrohBlobStore::new(&blob_dir, endpoint)
                    .await
                    .with_context(|| format!("failed to create blob store for node {}", node_id))?,
            );

            // Spawn router with blobs protocol for P2P blob transfers
            node.spawn_router_with_blobs(blobs.protocol_handler());

            info!(
                node_id = node_id,
                endpoint = %node.endpoint_addr().id.fmt_short(),
                "created test node with blob support"
            );

            // Create Pijul components for this node
            let kv: Arc<dyn KeyValueStore> = node.raft_node().clone();
            let store = Arc::new(PijulStore::new(blobs.clone(), kv, &data_dir));
            let pristine_mgr = PristineManager::new(&data_dir);

            nodes.push(PijulTestNode {
                node,
                store,
                blobs,
                pristine_mgr,
                sync_service: None,
                sync_handler: None,
                handler_handle: None,
                index: idx,
            });
        }

        Ok(Self {
            nodes,
            base_dir,
            cookie,
        })
    }

    /// Initialize the Raft cluster.
    ///
    /// Node 0 becomes the leader, other nodes are added as learners
    /// and then promoted to voters.
    pub async fn init_cluster(&mut self) -> Result<()> {
        info!("initializing Raft cluster");

        // Initialize cluster on node 0
        let node0 = &self.nodes[0].node;
        let init_request = InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, node0.endpoint_addr())],
        };

        node0.raft_node().init(init_request).await.context("failed to init cluster")?;

        // Wait for leader election
        sleep(LEADER_ELECTION_WAIT).await;

        // Verify node 0 is the leader
        let leader = node0.raft_node().get_leader().await?;
        assert_eq!(leader, Some(1), "node 1 should be the leader");
        info!("node 1 is the leader");

        // Wait for gossip discovery
        info!("waiting for gossip discovery");
        sleep(GOSSIP_DISCOVERY_WAIT).await;

        // Add other nodes as learners
        if self.nodes.len() > 1 {
            info!("adding nodes as learners");
            for idx in 1..self.nodes.len() {
                let node_id = (idx + 1) as u64;
                let node = &self.nodes[idx].node;
                let request = AddLearnerRequest {
                    learner: ClusterNode::with_iroh_addr(node_id, node.endpoint_addr()),
                };

                match node0.raft_node().add_learner(request).await {
                    Ok(state) => {
                        info!(node_id = node_id, learners = ?state.learners, "added learner");
                    }
                    Err(e) => {
                        info!(node_id = node_id, error = %e, "failed to add learner (may already be added)");
                    }
                }

                sleep(Duration::from_millis(500)).await;
            }

            // Promote all nodes to voters
            info!("promoting all nodes to voters");
            sleep(Duration::from_secs(2)).await;

            let members: Vec<u64> = (1..=self.nodes.len() as u64).collect();
            let request = ChangeMembershipRequest { members };

            let state = node0.raft_node().change_membership(request).await.context("failed to change membership")?;

            info!(members = ?state.members, "membership changed");
        }

        Ok(())
    }

    /// Start Pijul sync services on all nodes.
    ///
    /// Creates `PijulSyncService` and `PijulSyncHandler` for each node,
    /// enabling P2P change synchronization via gossip.
    pub async fn start_pijul_sync(&mut self) -> Result<()> {
        info!("starting Pijul sync on all nodes");

        for test_node in &mut self.nodes {
            let node = &test_node.node;
            let handle = node.handle();

            // Get the gossip instance from the IrohEndpointManager
            let gossip = handle.network.iroh_manager.gossip().cloned().context("gossip not enabled on node")?;

            // Get the secret key for signing announcements
            let secret_key = handle.network.iroh_manager.secret_key().clone();

            // Subscribe to channel update events from the PijulRefStore
            let channel_events = test_node.store.refs().subscribe();

            // Create a deferred handler wrapper to break the circular dependency:
            // - Handler needs sync_service to send responses
            // - Service needs handler to forward announcements
            let deferred_handler = Arc::new(DeferredSyncHandler::new());

            // Create sync service with the deferred handler
            let sync_service = PijulSyncService::spawn(
                gossip,
                secret_key,
                channel_events,
                Some(deferred_handler.clone() as Arc<dyn PijulSyncCallback>),
            )
            .await
            .context("failed to spawn PijulSyncService")?;

            // Now create the real handler with the sync service
            let (handler, handler_handle) = PijulSyncHandler::spawn(test_node.store.clone(), sync_service.clone());

            // Connect the deferred handler to the real handler
            deferred_handler.set_handler(handler.clone());

            test_node.sync_service = Some(sync_service);
            test_node.sync_handler = Some(handler);
            test_node.handler_handle = Some(handler_handle);

            info!(node_idx = test_node.index, "started Pijul sync service with handler");
        }

        Ok(())
    }

    /// Create a repository on a specific node.
    pub async fn create_repo(&self, node_idx: usize, name: &str) -> Result<RepoId> {
        let test_node = self.nodes.get(node_idx).context("node index out of bounds")?;

        let delegates = vec![test_node.node.handle().network.iroh_manager.node_addr().id];
        let identity = PijulRepoIdentity::new(name, delegates);

        let repo_id = test_node.store.create_repo(identity).await.context("failed to create repo")?;

        info!(node_idx, name, repo_id = %repo_id, "created repository");

        Ok(repo_id)
    }

    /// Subscribe a node to a repository's gossip topic.
    ///
    /// This both subscribes to gossip announcements for the repo AND
    /// watches the repo in the sync handler so announcements are processed.
    ///
    /// The method automatically discovers other nodes in the cluster and uses
    /// them as bootstrap peers for gossip connectivity.
    pub async fn subscribe_repo(&self, node_idx: usize, repo_id: &RepoId) -> Result<()> {
        let test_node = self.nodes.get(node_idx).context("node index out of bounds")?;

        // Collect node IDs of other nodes as bootstrap peers for gossip
        let bootstrap_peers: Vec<PublicKey> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != node_idx)
            .map(|(_, n)| n.node.handle().network.iroh_manager.node_addr().id)
            .collect();

        // Subscribe to gossip topic for this repo with bootstrap peers
        if let Some(sync_service) = &test_node.sync_service {
            sync_service
                .subscribe_repo_with_peers(repo_id, bootstrap_peers)
                .await
                .context("failed to subscribe to repo")?;
        }

        // Watch the repo in the handler so announcements trigger sync
        if let Some(sync_handler) = &test_node.sync_handler {
            sync_handler.watch_repo(*repo_id);
        }

        info!(node_idx, repo_id = %repo_id, "subscribed to repo and watching for sync");

        Ok(())
    }

    /// Record a change on a specific node.
    ///
    /// Creates files in a temporary working directory and records a change.
    /// Note: The ref update is routed through the Raft leader, which may be
    /// a different node. The change blob is stored locally on the specified node.
    pub async fn record_change(
        &self,
        node_idx: usize,
        repo_id: &RepoId,
        channel: &str,
        files: &[(&str, &str)],
        message: &str,
    ) -> Result<ChangeHash> {
        let test_node = self.nodes.get(node_idx).context("node index out of bounds")?;

        // Create a temporary working directory with files
        let work_dir = self.base_dir.join(format!("work-{}-{}-{}", node_idx, repo_id, rand::random::<u32>()));
        std::fs::create_dir_all(&work_dir)?;

        for (path, content) in files {
            let file_path = work_dir.join(path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, content)?;
        }

        // Create change directory
        let change_dir = ChangeDirectory::new(&test_node.node.data_dir(), *repo_id, test_node.store.changes().clone());

        // Get or create pristine
        let pristine = test_node.pristine_mgr.open_or_create(repo_id)?;

        // Record the change
        let recorder = ChangeRecorder::new(pristine, change_dir, work_dir);
        let author = format!("Node {} <node{}@test.local>", node_idx, node_idx);

        let result = recorder.record(channel, message, &author).await.context("failed to record change")?;

        let record_result = result.context("no changes to record")?;
        let hash = record_result.hash;

        // Update channel head in ref store via the leader.
        // Raft writes must go through the leader, so we find the leader node
        // and use its PijulStore.refs() for the write operation.
        let leader_store = self.get_leader_store().await?;
        leader_store
            .refs()
            .set_channel(repo_id, channel, hash)
            .await
            .context("failed to set channel head")?;

        info!(
            node_idx,
            repo_id = %repo_id,
            channel,
            hash = %hash,
            "recorded change"
        );

        Ok(hash)
    }

    /// Get the PijulStore for the current Raft leader.
    ///
    /// This is used to route writes through the leader node since Raft
    /// requires all writes to go through the leader.
    async fn get_leader_store(&self) -> Result<&Arc<PijulStore<IrohBlobStore, dyn KeyValueStore>>> {
        // Find the leader by checking any node
        let leader_id = self
            .nodes
            .first()
            .context("no nodes in cluster")?
            .node
            .raft_node()
            .get_leader()
            .await?
            .context("no leader elected")?;

        // Node IDs are 1-indexed, array is 0-indexed
        let leader_idx = (leader_id - 1) as usize;
        let leader_node = self.nodes.get(leader_idx).with_context(|| format!("leader node {} not found", leader_id))?;

        Ok(&leader_node.store)
    }

    /// Get the channel head on a specific node.
    ///
    /// Uses stale (local) reads so this works on follower nodes without
    /// requiring a round-trip to the Raft leader. This is appropriate for
    /// test verification since we just care that the data was synced.
    pub async fn get_channel_head(
        &self,
        node_idx: usize,
        repo_id: &RepoId,
        channel: &str,
    ) -> Result<Option<ChangeHash>> {
        let test_node = self.nodes.get(node_idx).context("node index out of bounds")?;

        let head = test_node
            .store
            .refs()
            .get_channel_with_consistency(repo_id, channel, ReadConsistency::Stale)
            .await
            .context("failed to get channel head")?;

        Ok(head)
    }

    /// Check if a node has a specific change.
    pub async fn has_change(&self, node_idx: usize, hash: &ChangeHash) -> Result<bool> {
        let test_node = self.nodes.get(node_idx).context("node index out of bounds")?;

        let has = test_node.store.changes().has_change(hash).await.context("failed to check change")?;

        Ok(has)
    }

    /// Wait for a change to propagate to a node.
    ///
    /// Polls until the change is available or timeout expires.
    pub async fn wait_for_change(
        &self,
        node_idx: usize,
        _repo_id: &RepoId,
        hash: &ChangeHash,
        timeout: Duration,
    ) -> Result<bool> {
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            if self.has_change(node_idx, hash).await? {
                return Ok(true);
            }
            sleep(POLL_INTERVAL).await;
        }

        Ok(false)
    }

    /// Wait for channel heads to match across all nodes.
    pub async fn wait_for_channel_sync(&self, repo_id: &RepoId, channel: &str, timeout: Duration) -> Result<bool> {
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            let mut heads = Vec::new();
            for (idx, _) in self.nodes.iter().enumerate() {
                let head = self.get_channel_head(idx, repo_id, channel).await?;
                heads.push(head);
            }

            // Check if all heads are the same (and not None)
            if heads.iter().all(|h| h.is_some()) {
                let first = &heads[0];
                if heads.iter().all(|h| h == first) {
                    return Ok(true);
                }
            }

            sleep(POLL_INTERVAL).await;
        }

        Ok(false)
    }

    /// Get a reference to a test node.
    pub fn node(&self, idx: usize) -> Option<&PijulTestNode> {
        self.nodes.get(idx)
    }

    /// Get the number of nodes in the cluster.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the base directory path.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Shutdown all nodes gracefully.
    pub async fn shutdown(self) -> Result<()> {
        info!("shutting down Pijul multi-node tester");

        for test_node in self.nodes {
            // Abort the handler worker first
            if let Some(handler_handle) = test_node.handler_handle {
                handler_handle.abort();
            }

            // Shutdown sync service
            if let Some(sync_service) = test_node.sync_service {
                if let Err(e) = sync_service.shutdown().await {
                    tracing::warn!(error = %e, "failed to shutdown sync service");
                }
            }

            // Shutdown blob store
            if let Err(e) = test_node.blobs.shutdown().await {
                tracing::warn!(error = %e, "failed to shutdown blob store");
            }

            // Then shutdown the node
            test_node.node.shutdown().await.context("failed to shutdown node")?;
        }

        Ok(())
    }
}

impl PijulTestNode {
    /// Get the Pijul store for this node.
    pub fn store(&self) -> &Arc<PijulStore<IrohBlobStore, dyn KeyValueStore>> {
        &self.store
    }

    /// Get the Aspen node.
    pub fn node(&self) -> &Node {
        &self.node
    }

    /// Get the blob store.
    pub fn blobs(&self) -> &Arc<IrohBlobStore> {
        &self.blobs
    }
}

/// Generate a deterministic secret key for a node (same pattern as multi_node_cluster_test.rs).
fn generate_secret_key(node_id: u64) -> String {
    format!("{:064x}", 1000 + node_id)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn test_pijul_tester_basic() -> Result<()> {
        let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

        let temp_dir = TempDir::new()?;
        let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;
        tester.init_cluster().await?;

        assert_eq!(tester.node_count(), 2);
        assert!(tester.node(0).is_some());
        assert!(tester.node(1).is_some());

        tester.shutdown().await?;
        Ok(())
    }
}
