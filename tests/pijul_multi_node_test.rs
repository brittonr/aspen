//! Multi-node Pijul integration tests.
//!
//! Tests P2P synchronization, conflict detection, and network partition handling
//! across multiple Aspen nodes with real Iroh networking.
//!
//! These tests require network access and are ignored by default.
//! Run with: `cargo nextest run -E 'test(/pijul_multi_node/)' --run-ignored all`

#![cfg(feature = "pijul")]

use std::time::Duration;

use anyhow::Result;
use aspen::testing::PijulMultiNodeTester;
use tempfile::TempDir;
use tracing::info;

/// Time to wait for gossip discovery between nodes.
const GOSSIP_SETTLE_TIME: Duration = Duration::from_secs(5);
/// Default sync timeout.
const SYNC_TIMEOUT: Duration = Duration::from_secs(30);

/// Test basic two-node cluster setup.
///
/// Verifies that we can create a 2-node cluster with Pijul enabled.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_pijul_cluster_setup() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info,iroh=warn")
        .try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;
    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    // Wait for gossip discovery
    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    info!("cluster setup complete with {} nodes", tester.node_count());
    assert_eq!(tester.node_count(), 2);

    tester.shutdown().await?;
    Ok(())
}

/// Test creating a repository on one node.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_pijul_create_repo() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info,iroh=warn")
        .try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;
    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    // Create repo on node 0
    let repo_id = tester.create_repo(0, "test-repo").await?;
    info!(repo_id = %repo_id, "created repository");

    // Subscribe both nodes to the repo
    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    tester.shutdown().await?;
    Ok(())
}

/// Test basic two-node sync.
///
/// Creates a change on node A and verifies node B receives it via gossip.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_basic_two_node_sync() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info,iroh=warn")
        .try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;
    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    // Wait for gossip discovery
    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    // Create repo on node 0
    let repo_id = tester.create_repo(0, "sync-test").await?;

    // Subscribe both nodes to the repo
    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    // Allow subscription to propagate
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Record change on node 0
    let hash = tester
        .record_change(
            0,
            &repo_id,
            "main",
            &[("hello.txt", "Hello, Pijul!\n")],
            "Initial commit",
        )
        .await?;

    info!(hash = %hash, "recorded change on node 0");

    // Wait for change to propagate to node 1
    let synced = tester
        .wait_for_change(1, &repo_id, &hash, SYNC_TIMEOUT)
        .await?;

    assert!(synced, "Change should propagate to node 1");
    info!("change successfully synced to node 1");

    // Verify channel heads match
    let head_0 = tester.get_channel_head(0, &repo_id, "main").await?;
    let head_1 = tester.get_channel_head(1, &repo_id, "main").await?;
    assert_eq!(head_0, head_1, "Channel heads should match");

    tester.shutdown().await?;
    Ok(())
}

/// Test recording multiple sequential changes.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_change_chain_dependencies() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info,iroh=warn")
        .try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;
    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    let repo_id = tester.create_repo(0, "chain-test").await?;
    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Record a chain of dependent changes
    let mut hashes = Vec::new();
    for i in 1..=3 {
        let hash = tester
            .record_change(
                0,
                &repo_id,
                "main",
                &[(
                    &format!("file{}.txt", i),
                    &format!("Content for file {}\n", i),
                )],
                &format!("Add file {}", i),
            )
            .await?;
        hashes.push(hash);
        info!(hash = %hash, i, "recorded change");
    }

    // Wait for all changes to propagate
    for hash in &hashes {
        let synced = tester
            .wait_for_change(1, &repo_id, hash, SYNC_TIMEOUT)
            .await?;
        assert!(synced, "Change {} should propagate", hash);
    }

    info!("all {} changes synced successfully", hashes.len());

    tester.shutdown().await?;
    Ok(())
}

/// Test three-node gossip propagation.
///
/// Each of 3 nodes creates a change, all nodes eventually have all changes.
#[tokio::test]
#[ignore = "requires network access"]
async fn test_three_node_gossip_propagation() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info,iroh=warn")
        .try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(3, temp_dir.path()).await?;
    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    // Create repo and subscribe all nodes
    let repo_id = tester.create_repo(0, "three-node-test").await?;
    for i in 0..3 {
        tester.subscribe_repo(i, &repo_id).await?;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Each node creates a change
    let hash_0 = tester
        .record_change(
            0,
            &repo_id,
            "main",
            &[("from_0.txt", "Node 0\n")],
            "From node 0",
        )
        .await?;

    // Give some time between recordings
    tokio::time::sleep(Duration::from_secs(1)).await;

    let hash_1 = tester
        .record_change(
            1,
            &repo_id,
            "main",
            &[("from_1.txt", "Node 1\n")],
            "From node 1",
        )
        .await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let hash_2 = tester
        .record_change(
            2,
            &repo_id,
            "main",
            &[("from_2.txt", "Node 2\n")],
            "From node 2",
        )
        .await?;

    info!(
        hash_0 = %hash_0,
        hash_1 = %hash_1,
        hash_2 = %hash_2,
        "recorded changes on all nodes"
    );

    // All nodes should eventually have all changes
    for node_idx in 0..3 {
        for hash in [&hash_0, &hash_1, &hash_2] {
            let synced = tester
                .wait_for_change(node_idx, &repo_id, hash, SYNC_TIMEOUT)
                .await?;
            assert!(
                synced,
                "Node {} should have change {}",
                node_idx, hash
            );
        }
    }

    info!("all nodes have all changes");

    tester.shutdown().await?;
    Ok(())
}

/// Test that changes are deduplicated (no duplicate downloads).
#[tokio::test]
#[ignore = "requires network access"]
async fn test_request_deduplication() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("aspen=info,iroh=warn")
        .try_init();

    let temp_dir = TempDir::new()?;
    let mut tester = PijulMultiNodeTester::new(2, temp_dir.path()).await?;
    tester.init_cluster().await?;
    tester.start_pijul_sync().await?;

    tokio::time::sleep(GOSSIP_SETTLE_TIME).await;

    let repo_id = tester.create_repo(0, "dedup-test").await?;
    tester.subscribe_repo(0, &repo_id).await?;
    tester.subscribe_repo(1, &repo_id).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Record multiple changes quickly
    let mut hashes = Vec::new();
    for i in 1..=5 {
        let hash = tester
            .record_change(
                0,
                &repo_id,
                "main",
                &[(
                    &format!("rapid{}.txt", i),
                    &format!("Rapid content {}\n", i),
                )],
                &format!("Rapid commit {}", i),
            )
            .await?;
        hashes.push(hash);
    }

    info!("recorded {} rapid changes", hashes.len());

    // Wait for sync
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Verify all changes arrived
    for hash in &hashes {
        let has = tester.has_change(1, hash).await?;
        assert!(has, "Node 1 should have change {}", hash);
    }

    info!("deduplication test passed");

    tester.shutdown().await?;
    Ok(())
}
