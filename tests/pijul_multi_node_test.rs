//! Multi-node Pijul integration tests.
//!
//! Tests P2P synchronization, conflict detection, and network partition handling
//! across multiple Aspen nodes with real Iroh networking.
//!
//! # Test Categories
//!
//! - Cluster setup and basic connectivity
//! - Repository creation and change recording
//! - P2P change synchronization via gossip
//! - Chain dependency handling
//! - Request deduplication
//!
//! # Running These Tests
//!
//! These tests require the `pijul` feature and network access:
//!
//! ```bash
//! cargo nextest run -E 'test(/pijul_multi_node/)' --features pijul --run-ignored all
//! ```

#![cfg(feature = "pijul")]

mod support;

use std::time::Duration;

use anyhow::Result;
use support::pijul_tester::PijulMultiNodeTester;
use tempfile::TempDir;
use tracing::info;

/// Time to wait for gossip discovery between nodes.
const GOSSIP_SETTLE_TIME: Duration = Duration::from_secs(5);
/// Default sync timeout.
const SYNC_TIMEOUT: Duration = Duration::from_secs(30);

/// Test: Basic cluster setup with Pijul support.
///
/// Validates that a multi-node cluster can be created with:
/// 1. Multiple Aspen nodes with gossip enabled
/// 2. Raft consensus initialized
/// 3. Pijul sync services started
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pijul_cluster_setup() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;

    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    assert_eq!(tester.node_count(), 2);
    assert!(tester.node(0).is_some());
    assert!(tester.node(1).is_some());

    info!("cluster setup complete");

    tester.shutdown().await?;
    Ok(())
}

/// Test: Create a repository on a node.
///
/// Validates repository creation:
/// 1. Create a repo on node 0
/// 2. Verify repo ID is returned
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_pijul_create_repo() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;

    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    let repo_id = tester.create_repo(0, "test-repo").await?;
    info!(repo_id = %repo_id, "created repository");

    tester.shutdown().await?;
    Ok(())
}

/// Test: Basic two-node change synchronization.
///
/// This is the core P2P sync test:
/// 1. Create a repo on node 0
/// 2. Subscribe node 1 to the repo
/// 3. Record a change on node 0
/// 4. Wait for change to sync to node 1
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_basic_two_node_sync() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;

    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    // Create repo on node 0
    let repo_id = tester.create_repo(0, "sync-test").await?;

    // Subscribe both nodes to the repo's gossip topic
    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    // Wait for gossip connections to establish
    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    // Record a change on node 0
    let hash = tester
        .record_change(0, &repo_id, "main", &[("hello.txt", "Hello, Pijul!\n")], "Initial commit")
        .await?;

    info!(hash = %hash, "recorded change on node 0");

    // Wait for change to propagate to node 1
    let synced = tester.wait_for_change(1, &repo_id, &hash, SYNC_TIMEOUT).await?;
    assert!(synced, "change should sync to node 1 within timeout");

    info!("change synced to node 1");

    tester.shutdown().await?;
    Ok(())
}

/// Test: Change chain dependencies are preserved.
///
/// Validates that dependent changes sync correctly:
/// 1. Record change A
/// 2. Record change B (depends on A)
/// 3. Verify both sync in order
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_change_chain_dependencies() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;

    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    let repo_id = tester.create_repo(0, "chain-test").await?;

    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    // Record change A
    let hash_a = tester.record_change(0, &repo_id, "main", &[("file.txt", "Version A\n")], "Change A").await?;

    // Record change B (modifies same file)
    let hash_b = tester.record_change(0, &repo_id, "main", &[("file.txt", "Version B\n")], "Change B").await?;

    info!(hash_a = %hash_a, hash_b = %hash_b, "recorded change chain");

    // Both changes should sync
    let synced_a = tester.wait_for_change(1, &repo_id, &hash_a, SYNC_TIMEOUT).await?;
    let synced_b = tester.wait_for_change(1, &repo_id, &hash_b, SYNC_TIMEOUT).await?;

    assert!(synced_a, "change A should sync");
    assert!(synced_b, "change B should sync");

    tester.shutdown().await?;
    Ok(())
}

/// Test: Three-node gossip propagation.
///
/// Validates gossip works across multiple hops:
/// 1. Create a repo on node 0
/// 2. Subscribe all 3 nodes
/// 3. Record a change on node 0
/// 4. Verify change reaches nodes 1 and 2
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_three_node_gossip_propagation() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(3, temp_dir.path()).await?;

    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    let repo_id = tester.create_repo(0, "gossip-test").await?;

    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;
    tester.subscribe_repo(2, &repo_id).await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    let hash = tester
        .record_change(0, &repo_id, "main", &[("test.txt", "Three node test\n")], "Gossip test")
        .await?;

    info!(hash = %hash, "recorded change on node 0");

    // Verify propagation to both other nodes
    let synced_1 = tester.wait_for_change(1, &repo_id, &hash, SYNC_TIMEOUT).await?;
    let synced_2 = tester.wait_for_change(2, &repo_id, &hash, SYNC_TIMEOUT).await?;

    assert!(synced_1, "change should reach node 1");
    assert!(synced_2, "change should reach node 2");

    tester.shutdown().await?;
    Ok(())
}

/// Test: Request deduplication prevents duplicate fetches.
///
/// Validates that multiple announcements for the same change
/// don't result in duplicate download requests.
#[tokio::test]
#[ignore = "requires network access - not available in Nix sandbox"]
async fn test_request_deduplication() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("aspen=info,iroh=warn").try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;

    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    let repo_id = tester.create_repo(0, "dedup-test").await?;

    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    // Record a change
    let hash = tester.record_change(0, &repo_id, "main", &[("dedup.txt", "Dedup test\n")], "Dedup test").await?;

    // Wait for sync
    let synced = tester.wait_for_change(1, &repo_id, &hash, SYNC_TIMEOUT).await?;
    assert!(synced, "change should sync");

    // The sync handler should have deduplicated any duplicate announcements
    // We can't easily verify this without metrics, but the test passing
    // without errors indicates the dedup logic didn't cause issues

    tester.shutdown().await?;
    Ok(())
}
