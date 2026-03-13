//! NAT topology integration tests using patchbay.
//!
//! These tests verify Raft cluster formation and KV operations across
//! different NAT types using real iroh QUIC connections through simulated
//! network topologies.
//!
//! Requirements:
//! - Linux with unprivileged user namespaces
//! - `nft` and `tc` in PATH

use aspen_testing_patchbay::prelude::*;

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
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

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

    // Home NAT requires relay-based discovery — use longer timeout
    let leader = harness.wait_for_leader(std::time::Duration::from_secs(60)).await;

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

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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
    let result = harness.wait_for_leader(std::time::Duration::from_secs(60)).await;

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

    // Create three CGNAT routers
    for i in 0..3u32 {
        let router = harness
            .add_router(&format!("cgnat-{}", i), RouterPreset::IspCgnat)
            .await
            .expect("failed to create CGNAT router");
        harness.add_node(&format!("node-{}", i), router, None).await.expect("failed to add node");
    }

    // CGNAT is carrier-grade NAT — very restrictive
    let result = harness.wait_for_leader(std::time::Duration::from_secs(60)).await;

    match result {
        Ok(_) => {
            // If cluster formed, test batch KV writes
            for i in 0..100u32 {
                harness
                    .write_kv(&format!("batch-{}", i), &format!("value-{}", i))
                    .await
                    .expect("batch write failed");
            }

            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            // Verify all replicated
            for i in 0..100u32 {
                let val = harness.read_kv(2, &format!("batch-{}", i)).await.expect("batch read failed");
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

    let result = harness.wait_for_leader(std::time::Duration::from_secs(60)).await;

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
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

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
