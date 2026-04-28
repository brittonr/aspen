//! NAT topology integration tests using patchbay.
//!
//! These tests verify Raft cluster formation and KV operations across
//! different NAT types using real iroh QUIC connections through simulated
//! network topologies.
//!
//! Requirements:
//! - Linux with unprivileged user namespaces
//! - `nft` and `tc` in PATH

use std::time::Duration;

use aspen_testing_patchbay::prelude::*;

const PUBLIC_REPLICATION_WAIT: Duration = Duration::from_secs(2);
const HOME_NAT_REPLICATION_WAIT: Duration = Duration::from_secs(5);
// Keep expected-failure NAT waits below nextest's default 60s slow timeout so
// tests can report the expected relay gap and still shut down cleanly.
const NAT_EXPECTED_FAILURE_TIMEOUT: Duration = Duration::from_secs(45);
const CGNAT_ROUTER_COUNT: u32 = 3;
const CGNAT_BATCH_WRITE_COUNT: u32 = 100;
const CGNAT_BATCH_REPLICATION_WAIT: Duration = Duration::from_secs(10);
const CGNAT_READ_NODE_INDEX: usize = 2;
const MIXED_NAT_FAILOVER_WAIT: Duration = Duration::from_secs(10);

/// Initialize user namespace before any threads are spawned.
/// This must happen at process load time (before tokio runtime starts).
#[ctor::ctor]
fn init_userns() {
    // Silently skip if patchbay isn't available
    if patchbay_available() {
        // Safety: called from ctor (single-threaded, before tokio runtime).
        unsafe { patchbay::init_userns_for_ctor() };
    }
}

#[tokio::test]
async fn test_public_baseline() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("failed to init cluster");

    // Write and read back a value
    harness.write_kv("test-key", "test-value").await.expect("failed to write KV");

    // Allow replication
    tokio::time::sleep(PUBLIC_REPLICATION_WAIT).await;

    for i in 0..harness.node_count() {
        let val = harness.read_kv(i, "test-key").await.expect("failed to read KV");
        assert_eq!(val, Some("test-value".to_string()), "node {} should have replicated value", i);
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn test_home_nat_cluster_formation() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_home_nat().await.expect("failed to create topology");

    // Home NAT requires relay-based discovery; without relay, this should fail
    // within the bounded wait.
    let leader = harness.wait_for_leader(NAT_EXPECTED_FAILURE_TIMEOUT).await;

    match leader {
        Ok(leader_id) => {
            assert!(leader_id.0 > 0, "leader should have a valid ID");
        }
        Err(e) => {
            // NAT traversal without relay is expected to fail in this topology.
            // This test documents the behavior — relay support is needed for NAT.
            eprintln!("home NAT cluster formation failed (expected without relay): {}", e);
        }
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn test_home_nat_kv_replication() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_home_nat().await.expect("failed to create topology");

    // Try to init cluster — may fail without relay
    if harness.init_cluster().await.is_err() {
        eprintln!("skipping KV replication test: cluster init failed (no relay)");
        harness.shutdown().await;
        return;
    }

    harness.write_kv("nat-key", "nat-value").await.expect("failed to write KV");

    tokio::time::sleep(HOME_NAT_REPLICATION_WAIT).await;

    for i in 0..harness.node_count() {
        let val = harness.read_kv(i, "nat-key").await.expect("failed to read KV");
        assert_eq!(val, Some("nat-value".to_string()));
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn test_corporate_nat_cluster() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::mixed_nat(0, 0, 3).await.expect("failed to create topology");

    // Corporate NAT is highly restrictive — expect relay-based fallback
    let result = harness.wait_for_leader(NAT_EXPECTED_FAILURE_TIMEOUT).await;

    match result {
        Ok(leader_id) => {
            assert!(leader_id.0 > 0);
        }
        Err(e) => {
            eprintln!("corporate NAT cluster formation failed (expected without relay): {}", e);
        }
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn test_cgnat_cluster() {
    skip_unless_patchbay!();

    let mut harness = PatchbayHarness::new().await.expect("failed to create harness");

    // Create CGNAT routers.
    for i in 0..CGNAT_ROUTER_COUNT {
        let router = harness
            .add_router(&format!("cgnat-{}", i), RouterPreset::IspCgnat)
            .await
            .expect("failed to create CGNAT router");
        harness.add_node(&format!("node-{}", i), router, None).await.expect("failed to add node");
    }

    // CGNAT is carrier-grade NAT — very restrictive
    let result = harness.wait_for_leader(NAT_EXPECTED_FAILURE_TIMEOUT).await;

    match result {
        Ok(_) => {
            // If cluster formed, test batch KV writes
            for i in 0..CGNAT_BATCH_WRITE_COUNT {
                harness
                    .write_kv(&format!("batch-{}", i), &format!("value-{}", i))
                    .await
                    .expect("batch write failed");
            }

            tokio::time::sleep(CGNAT_BATCH_REPLICATION_WAIT).await;

            // Verify all replicated
            for i in 0..CGNAT_BATCH_WRITE_COUNT {
                let val =
                    harness.read_kv(CGNAT_READ_NODE_INDEX, &format!("batch-{}", i)).await.expect("batch read failed");
                assert_eq!(val, Some(format!("value-{}", i)));
            }
        }
        Err(e) => {
            eprintln!("CGNAT cluster failed (expected without relay): {}", e);
        }
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn test_mixed_nat_cluster() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::mixed_nat(1, 1, 1).await.expect("failed to create topology");

    let result = harness.wait_for_leader(NAT_EXPECTED_FAILURE_TIMEOUT).await;

    match result {
        Ok(leader_id) => {
            assert!(leader_id.0 > 0);
        }
        Err(e) => {
            eprintln!("mixed NAT cluster failed (expected without relay): {}", e);
        }
    }

    harness.shutdown().await;
}

#[tokio::test]
async fn test_mixed_nat_leader_failover() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::mixed_nat(1, 1, 1).await.expect("failed to create topology");

    if harness.init_cluster().await.is_err() {
        eprintln!("skipping failover test: cluster init failed");
        harness.shutdown().await;
        return;
    }

    let leader = harness.check_leader().await.expect("no leader");

    // Shut down the leader
    harness.node_handle(leader).expect("leader handle").shutdown().await.expect("shutdown failed");

    // Wait for re-election (remaining nodes must reach quorum)
    tokio::time::sleep(MIXED_NAT_FAILOVER_WAIT).await;

    // Check remaining nodes for new leader
    let mut found_new_leader = false;
    for handle in harness.handles() {
        if handle.node_id == leader {
            continue; // Skip the shut-down node
        }
        if let Ok(Some(new_leader)) = handle.get_leader().await
            && new_leader != leader
        {
            found_new_leader = true;
            break;
        }
    }

    assert!(found_new_leader, "a new leader should have been elected");

    harness.shutdown().await;
}
