//! Integration tests for Forge with real Aspen clusters.
//!
//! These tests validate Forge operations using actual Raft consensus and Iroh P2P
//! networking, unlike the in-memory tests that use DeterministicKeyValueStore.
//!
//! # Test Categories
//!
//! 1. **Repository Operations** - Create repos via Raft consensus
//! 2. **Commit Operations** - Store blobs/trees/commits with real storage
//! 3. **Multi-Node Sync** - Verify data replication across cluster
//! 4. **Issue Lifecycle** - COB operations with consensus
//!
//! # Running These Tests
//!
//! These tests require network access (Iroh P2P binding) and are marked `#[ignore]`
//! since they can't run in the Nix sandbox:
//!
//! ```sh
//! cargo nextest run --ignored forge_real_cluster
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Nodes shut down after tests
//! - Explicit error handling: All errors wrapped with context

#![cfg(feature = "forge")]

mod support;

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
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use aspen_blob::InMemoryBlobStore;
use aspen_forge::ForgeNode;
use aspen_forge::RepoId;
use aspen_forge::TreeEntry;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::info;

/// Maximum time to wait for cluster formation.
const CLUSTER_FORMATION_TIMEOUT: Duration = Duration::from_secs(60);
/// Time to wait for leader election.
const LEADER_ELECTION_WAIT: Duration = Duration::from_millis(2000);
/// Time to wait for Raft replication.
const REPLICATION_WAIT: Duration = Duration::from_millis(500);
/// Test cluster cookie.
const TEST_COOKIE: &str = "forge-real-cluster-test";

/// Test infrastructure for Forge with real Aspen clusters.
struct ForgeClusterTester {
    /// Running nodes.
    nodes: Vec<Node>,
    /// ForgeNode instances (one per cluster node).
    forge_nodes: Vec<ForgeNode<InMemoryBlobStore, dyn KeyValueStore>>,
    /// Shared blob store for all nodes (simulates iroh-blobs replication).
    /// Kept alive via Arc - nodes reference it through ForgeNode.
    #[allow(dead_code)]
    blobs: Arc<InMemoryBlobStore>,
    /// Temporary directory for node data.
    _temp_dir: TempDir,
}

impl ForgeClusterTester {
    /// Create a new Forge cluster tester with the specified node count.
    async fn new(node_count: usize) -> Result<Self> {
        let temp_dir = TempDir::new().context("failed to create temp directory")?;
        let mut nodes = Vec::with_capacity(node_count);

        // Shared blob store (simulates iroh-blobs content-addressed storage)
        let blobs = Arc::new(InMemoryBlobStore::new());

        // Create all nodes
        info!(node_count, "creating Forge cluster nodes");
        for id in 1..=node_count as u64 {
            let node = Self::create_node(id, &temp_dir).await?;
            nodes.push(node);
        }

        // Initialize cluster on node 1
        let node1 = &nodes[0];
        let init_request = InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
        };

        timeout(CLUSTER_FORMATION_TIMEOUT, node1.raft_node().init(init_request))
            .await
            .context("cluster init timeout")?
            .context("cluster init failed")?;

        // Wait for leader election
        sleep(LEADER_ELECTION_WAIT).await;

        // Add other nodes as learners if more than 1 node
        if node_count > 1 {
            for id in 2..=node_count as u64 {
                let node = &nodes[(id - 1) as usize];
                let learner = ClusterNode::with_iroh_addr(id, node.endpoint_addr());
                let request = AddLearnerRequest { learner };

                match timeout(CLUSTER_FORMATION_TIMEOUT, node1.raft_node().add_learner(request)).await {
                    Ok(Ok(_)) => info!(node_id = id, "added learner"),
                    Ok(Err(e)) => tracing::warn!(node_id = id, error = %e, "failed to add learner"),
                    Err(_) => tracing::warn!(node_id = id, "add_learner timeout"),
                }
                sleep(Duration::from_millis(500)).await;
            }

            // Wait for learners to catch up
            sleep(Duration::from_secs(2)).await;

            // Promote all nodes to voters
            let members: Vec<u64> = (1..=node_count as u64).collect();
            let request = ChangeMembershipRequest { members };

            timeout(CLUSTER_FORMATION_TIMEOUT, node1.raft_node().change_membership(request))
                .await
                .context("change_membership timeout")?
                .context("change_membership failed")?;

            info!(node_count, "all nodes promoted to voters");
        }

        // Create ForgeNode instances for each cluster node
        let mut forge_nodes = Vec::with_capacity(node_count);
        for node in &nodes {
            let secret_key = iroh::SecretKey::generate(&mut rand::rng());
            let kv: Arc<dyn KeyValueStore> = node.raft_node().clone();
            let forge = ForgeNode::new(blobs.clone(), kv, secret_key);
            forge_nodes.push(forge);
        }

        Ok(Self {
            nodes,
            forge_nodes,
            blobs,
            _temp_dir: temp_dir,
        })
    }

    /// Create a single node.
    async fn create_node(node_id: u64, temp_dir: &TempDir) -> Result<Node> {
        let data_dir = temp_dir.path().join(format!("node-{}", node_id));
        let secret_key = format!("{:064x}", 3000 + node_id);

        let mut node = NodeBuilder::new(NodeId(node_id), &data_dir)
            .with_storage(StorageBackend::InMemory)
            .with_cookie(TEST_COOKIE)
            .with_gossip(true)
            .with_mdns(false) // Disable mDNS for CI compatibility
            .with_iroh_secret_key(&secret_key)
            .with_heartbeat_interval_ms(500)
            .with_election_timeout_ms(1500, 3000)
            .start()
            .await
            .context("failed to start node")?;

        // Spawn the router to enable inter-node communication
        node.spawn_router();

        info!(
            node_id,
            endpoint = %node.endpoint_addr().id.fmt_short(),
            "node created"
        );

        Ok(node)
    }

    /// Get a ForgeNode by index.
    fn forge(&self, idx: usize) -> &ForgeNode<InMemoryBlobStore, dyn KeyValueStore> {
        &self.forge_nodes[idx]
    }

    /// Wait for Raft replication to propagate.
    async fn wait_for_replication(&self) {
        sleep(REPLICATION_WAIT).await;
    }

    /// Shutdown the cluster gracefully.
    async fn shutdown(self) -> Result<()> {
        for node in self.nodes {
            if let Err(e) = node.shutdown().await {
                tracing::warn!(error = %e, "node shutdown error");
            }
        }
        info!("Forge cluster shutdown complete");
        Ok(())
    }
}

// ============================================================================
// Repository Operations with Real Consensus
// ============================================================================

/// Test creating a repository through Raft consensus.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_create_repo_real_cluster() {
    let tester = ForgeClusterTester::new(3).await.expect("cluster setup");

    // Create repo on node 0 (goes through Raft consensus)
    let forge = tester.forge(0);
    let identity = forge.create_repo("test-project", vec![forge.public_key()], 1).await.expect("create repo");

    assert_eq!(identity.name, "test-project");
    let repo_id = identity.repo_id();

    // Wait for replication
    tester.wait_for_replication().await;

    // Verify repo is visible on node 1 (read from replicated state)
    let forge1 = tester.forge(1);
    let retrieved = forge1.get_repo(&repo_id).await.expect("get repo from node 1");
    assert_eq!(retrieved.name, "test-project");

    // Verify repo is visible on node 2
    let forge2 = tester.forge(2);
    let retrieved2 = forge2.get_repo(&repo_id).await.expect("get repo from node 2");
    assert_eq!(retrieved2.name, "test-project");

    tester.shutdown().await.expect("shutdown");
}

/// Test creating commits with blobs, trees through real storage.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_commit_real_cluster() {
    let tester = ForgeClusterTester::new(3).await.expect("cluster setup");
    let forge = tester.forge(0);

    // Create repo
    let identity = forge.create_repo("commit-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create a blob
    let blob_hash = forge.git.store_blob(b"Hello, World!").await.expect("store blob");

    // Create a tree with the blob
    let tree_hash = forge.git.create_tree(&[TreeEntry::file("README.md", blob_hash)]).await.expect("create tree");

    // Create a commit
    let commit_hash = forge.git.commit(tree_hash, vec![], "Initial commit").await.expect("create commit");

    // Set the main ref (goes through Raft)
    forge.refs.set(&repo_id, "heads/main", commit_hash).await.expect("set ref");

    // Wait for replication
    tester.wait_for_replication().await;

    // Verify ref is visible on all nodes
    for (idx, f) in tester.forge_nodes.iter().enumerate() {
        let ref_value = f.refs.get(&repo_id, "heads/main").await.expect("get ref");
        assert_eq!(ref_value, Some(commit_hash), "node {} should have correct ref", idx);
    }

    tester.shutdown().await.expect("shutdown");
}

// ============================================================================
// Issue Lifecycle with Real Consensus
// ============================================================================

/// Test complete issue lifecycle through Raft consensus.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_issue_lifecycle_real_cluster() {
    let tester = ForgeClusterTester::new(3).await.expect("cluster setup");
    let forge = tester.forge(0);

    // Create repo
    let identity = forge.create_repo("issue-test", vec![forge.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Create an issue (returns COB ID)
    let issue_id = forge
        .cobs
        .create_issue(&repo_id, "Bug report", "Something is broken", vec!["bug".to_string()])
        .await
        .expect("create issue");

    // Wait for replication
    tester.wait_for_replication().await;

    // Add a comment from node 1
    let forge1 = tester.forge(1);
    forge1.cobs.add_comment(&repo_id, &issue_id, "I can reproduce this").await.expect("add comment");

    tester.wait_for_replication().await;

    // Close the issue from node 2
    let forge2 = tester.forge(2);
    forge2
        .cobs
        .close_issue(&repo_id, &issue_id, Some("Fixed in latest release".to_string()))
        .await
        .expect("close issue");

    tester.wait_for_replication().await;

    // Resolve the issue state and verify changes from all nodes
    let resolved = forge.cobs.resolve_issue(&repo_id, &issue_id).await.expect("resolve issue");

    assert_eq!(resolved.title, "Bug report");
    assert_eq!(resolved.body, "Something is broken");
    assert!(resolved.labels.contains("bug"));
    assert_eq!(resolved.comments.len(), 1);
    assert_eq!(resolved.comments[0].body, "I can reproduce this");
    assert!(resolved.state.is_closed());

    tester.shutdown().await.expect("shutdown");
}

// ============================================================================
// Multi-Node Ref Updates
// ============================================================================

/// Test ref updates from different nodes converge correctly.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_ref_updates_multi_node() {
    let tester = ForgeClusterTester::new(3).await.expect("cluster setup");

    // Create repo on node 0
    let forge0 = tester.forge(0);
    let identity = forge0.create_repo("ref-test", vec![forge0.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    tester.wait_for_replication().await;

    // Create commits on different nodes
    let commit1 = forge0
        .git
        .commit(forge0.git.create_tree(&[]).await.unwrap(), vec![], "Commit from node 0")
        .await
        .expect("commit 1");

    let forge1 = tester.forge(1);
    let commit2 = forge1
        .git
        .commit(forge1.git.create_tree(&[]).await.unwrap(), vec![commit1], "Commit from node 1")
        .await
        .expect("commit 2");

    let forge2 = tester.forge(2);
    let commit3 = forge2
        .git
        .commit(forge2.git.create_tree(&[]).await.unwrap(), vec![commit2], "Commit from node 2")
        .await
        .expect("commit 3");

    // Update refs through Raft (sequential to avoid conflicts)
    forge0.refs.set(&repo_id, "heads/main", commit1).await.expect("set ref 1");
    tester.wait_for_replication().await;

    forge1.refs.set(&repo_id, "heads/main", commit2).await.expect("set ref 2");
    tester.wait_for_replication().await;

    forge2.refs.set(&repo_id, "heads/main", commit3).await.expect("set ref 3");
    tester.wait_for_replication().await;

    // All nodes should see the final commit
    for (idx, f) in tester.forge_nodes.iter().enumerate() {
        let head = f.refs.get(&repo_id, "heads/main").await.expect("get ref");
        assert_eq!(head, Some(commit3), "node {} should have commit3 as head", idx);
    }

    tester.shutdown().await.expect("shutdown");
}

// ============================================================================
// Seeding and Peer Discovery
// ============================================================================

/// Test seeding peer registration through consensus.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_seeding_peers() {
    let tester = ForgeClusterTester::new(3).await.expect("cluster setup");
    let forge0 = tester.forge(0);

    // Create repo
    let identity = forge0.create_repo("seeding-test", vec![forge0.public_key()], 1).await.expect("create repo");
    let repo_id = identity.repo_id();

    // Add seeders from each node
    for (idx, forge) in tester.forge_nodes.iter().enumerate() {
        forge
            .add_seeding_peer(&repo_id, forge.public_key())
            .await
            .unwrap_or_else(|_| panic!("add seeder from node {}", idx));
        tester.wait_for_replication().await;
    }

    // Verify all seeders are visible
    let seeders = forge0.get_seeding_peers(&repo_id).await.expect("get seeders");
    assert_eq!(seeders.len(), 3, "should have 3 seeders");

    // Verify from another node
    let forge1 = tester.forge(1);
    let seeders1 = forge1.get_seeding_peers(&repo_id).await.expect("get seeders from node 1");
    assert_eq!(seeders1.len(), 3, "node 1 should also see 3 seeders");

    tester.shutdown().await.expect("shutdown");
}

// ============================================================================
// Error Handling
// ============================================================================

/// Test repo not found error handling.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_repo_not_found() {
    let tester = ForgeClusterTester::new(1).await.expect("cluster setup");
    let forge = tester.forge(0);

    let fake_id = RepoId::from_hash(blake3::hash(b"nonexistent"));
    let result = forge.get_repo(&fake_id).await;

    assert!(result.is_err(), "should return error for nonexistent repo");

    tester.shutdown().await.expect("shutdown");
}

/// Test duplicate repo creation error.
#[tokio::test]
#[ignore = "requires network access (Iroh P2P binding) - not available in Nix sandbox"]
async fn test_forge_duplicate_repo() {
    let tester = ForgeClusterTester::new(1).await.expect("cluster setup");
    let forge = tester.forge(0);

    // Create repo
    forge.create_repo("duplicate-test", vec![forge.public_key()], 1).await.expect("create repo");

    // Try to create again - should fail
    let result = forge.create_repo("duplicate-test", vec![forge.public_key()], 1).await;

    assert!(result.is_err(), "should return error for duplicate repo");

    tester.shutdown().await.expect("shutdown");
}
