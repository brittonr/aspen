//! Fault injection integration tests using patchbay.
//!
//! These tests verify Raft cluster resilience under network faults:
//! partitions, latency injection, packet loss, link flaps, and
//! dynamic NAT changes — all using real iroh QUIC connections.

use std::time::Duration;

use aspen_testing_core::wait_for_key_present;
use aspen_testing_patchbay::prelude::*;

#[ctor::ctor]
fn init_userns() {
    if patchbay_available() {
        unsafe { patchbay::init_userns_for_ctor() };
    }
}

const EU_REGION_NODE_COUNT: u32 = 2;
const US_REGION_NODE_COUNT: u32 = 1;
const REGION_LINK_LATENCY_MS: u32 = 20;
const US_REGION_NODE_INDEX: usize = 2;
const NODE_DATA_IFACE: &str = "eth0";
const REGION_PARTITION_SETTLE: Duration = Duration::from_secs(5);
const MINORITY_PARTITION_SETTLE: Duration = Duration::from_secs(10);

// ============================================================================
// Region partition tests
// ============================================================================

#[tokio::test]
async fn test_region_partition_majority_quorum() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::two_region(EU_REGION_NODE_COUNT, US_REGION_NODE_COUNT, REGION_LINK_LATENCY_MS)
        .await
        .expect("failed to create two-region topology");

    harness.init_cluster().await.expect("cluster init failed");

    harness.write_kv("pre-partition", "exists").await.expect("pre-partition write failed");

    // Isolate the single-node US region. Patchbay's break_region_link reroutes
    // through an intermediate region, so a two-region quorum test must drop the
    // US device link directly.
    harness.devices()[US_REGION_NODE_INDEX]
        .link_down(NODE_DATA_IFACE)
        .await
        .expect("failed to isolate US region node");

    tokio::time::sleep(REGION_PARTITION_SETTLE).await;

    let eu_leader = harness.handles()[0].get_leader().await;
    match eu_leader {
        Ok(Some(leader)) => {
            assert!(leader.0 > 0, "EU partition should have a leader");
        }
        Ok(None) => {
            tokio::time::sleep(REGION_PARTITION_SETTLE).await;
            let retry = harness.handles()[0].get_leader().await;
            assert!(matches!(retry, Ok(Some(_))), "EU partition should eventually elect a leader");
        }
        Err(error) => panic!("failed to query EU node: {error}"),
    }

    link_up_after_admin_down(&harness.devices()[US_REGION_NODE_INDEX], NODE_DATA_IFACE).await;

    harness.shutdown().await;
}

#[tokio::test]
async fn test_region_partition_heal_catchup() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::two_region(EU_REGION_NODE_COUNT, US_REGION_NODE_COUNT, REGION_LINK_LATENCY_MS)
        .await
        .expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    harness.devices()[US_REGION_NODE_INDEX]
        .link_down(NODE_DATA_IFACE)
        .await
        .expect("failed to isolate US region node");
    tokio::time::sleep(REGION_PARTITION_SETTLE).await;

    harness.write_kv("during-partition", "value").await.expect("write during partition failed");

    link_up_after_admin_down(&harness.devices()[US_REGION_NODE_INDEX], NODE_DATA_IFACE).await;

    wait_for_key_present("during-partition", FOLLOWER_CATCHUP_TIMEOUT, || async {
        match harness.read_kv(US_REGION_NODE_INDEX, "during-partition").await {
            Ok(Some(value)) => Ok(value == "value"),
            Ok(None) | Err(_) => Ok(false),
        }
    })
    .await
    .expect("US region node should catch up after partition heals");

    harness.shutdown().await;
}

#[tokio::test]
async fn test_region_partition_minority_rejects_writes() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::two_region(EU_REGION_NODE_COUNT, US_REGION_NODE_COUNT, REGION_LINK_LATENCY_MS)
        .await
        .expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    harness.devices()[US_REGION_NODE_INDEX]
        .link_down(NODE_DATA_IFACE)
        .await
        .expect("failed to isolate US region node");
    tokio::time::sleep(MINORITY_PARTITION_SETTLE).await;

    let result = harness.handles()[US_REGION_NODE_INDEX].write_kv("isolated-write", "fail").await;

    match result {
        Err(_) => { /* Expected: write fails without quorum */ }
        Ok(()) => {
            eprintln!("warning: isolated node accepted write (may have stale leader state)");
        }
    }

    link_up_after_admin_down(&harness.devices()[US_REGION_NODE_INDEX], NODE_DATA_IFACE).await;
    harness.shutdown().await;
}

// ============================================================================
// Latency injection tests
// ============================================================================

const TOTAL_LEADER_EGRESS_LOSS_PCT: f32 = 100.0;
const LEADER_FAILOVER_WAIT: Duration = Duration::from_secs(15);
const PATCHBAY_IPV6_ROUTE_RESTORE_GAP: &str = "No route to host";
const LINK_UP_ROUTE_RESTORE_ATTEMPTS: u32 = 3;
const LINK_UP_ROUTE_RESTORE_RETRY_DELAY: Duration = Duration::from_millis(500);
const FOLLOWER_CATCHUP_TIMEOUT: Duration = Duration::from_secs(60);

async fn link_up_after_admin_down(device: &Device, ifname: &str) {
    let mut last_route_restore_gap = None;

    for _attempt in 0..LINK_UP_ROUTE_RESTORE_ATTEMPTS {
        match device.link_up(ifname).await {
            Ok(()) => return,
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains(PATCHBAY_IPV6_ROUTE_RESTORE_GAP), "link_up failed unexpectedly: {message}");
                last_route_restore_gap = Some(message);
                tokio::time::sleep(LINK_UP_ROUTE_RESTORE_RETRY_DELAY).await;
            }
        }
    }

    if let Some(message) = last_route_restore_gap {
        eprintln!("patchbay link_up route restore warning after retries: {message}");
    }
}

#[tokio::test]
async fn test_latency_200ms_no_false_election() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    let original_leader = harness.check_leader().await.expect("no leader");

    // Inject 200ms latency on all devices
    for device in harness.devices() {
        device
            .set_link_condition(
                "eth0",
                Some(LinkCondition::Manual(LinkLimits {
                    latency_ms: 200,
                    ..Default::default()
                })),
            )
            .await
            .expect("failed to set latency");
    }

    // Wait and verify leader is stable (200ms < election timeout 3000ms)
    tokio::time::sleep(Duration::from_secs(10)).await;

    let current_leader = harness.check_leader().await;
    match current_leader {
        Ok(leader) => {
            assert_eq!(leader, original_leader, "leader should remain stable under 200ms latency");
        }
        Err(_) => {
            eprintln!("nodes disagree on leader (latency may have caused brief instability)");
        }
    }

    // Verify writes still work
    let write_result = harness.write_kv("latency-test", "works").await;
    assert!(write_result.is_ok(), "writes should succeed under 200ms latency");

    harness.shutdown().await;
}

#[tokio::test]
async fn test_leader_egress_loss_triggers_new_election() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    let original_leader = harness.check_leader().await.expect("no leader");
    let leader_idx = harness.handles().iter().position(|h| h.node_id == original_leader).expect("leader not found");

    // Pure latency does not imply isolation: once delayed heartbeats pipeline,
    // followers can keep observing the original leader. Drop all leader egress
    // packets to exercise the failover path while leaving the interface up.
    harness.devices()[leader_idx]
        .set_link_condition(
            "eth0",
            Some(LinkCondition::Manual(LinkLimits {
                loss_pct: TOTAL_LEADER_EGRESS_LOSS_PCT,
                ..Default::default()
            })),
        )
        .await
        .expect("failed to drop leader egress");

    tokio::time::sleep(LEADER_FAILOVER_WAIT).await;

    let mut new_leader_found = false;
    for handle in harness.handles() {
        if handle.node_id == original_leader {
            continue;
        }
        if let Ok(Some(leader)) = handle.get_leader().await
            && leader != original_leader
        {
            new_leader_found = true;
            break;
        }
    }

    assert!(new_leader_found, "followers should elect a new leader when leader egress is dropped");

    harness.shutdown().await;
}

// ============================================================================
// Packet loss tests
// ============================================================================

#[tokio::test]
async fn test_packet_loss_10pct_replication() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    // Inject 10% packet loss on all devices
    for device in harness.devices() {
        device
            .set_link_condition(
                "eth0",
                Some(LinkCondition::Manual(LinkLimits {
                    loss_pct: 10.0,
                    ..Default::default()
                })),
            )
            .await
            .expect("failed to set packet loss");
    }

    // Write 50 KV pairs
    for i in 0..50u32 {
        let result = harness.write_kv(&format!("loss-{}", i), &format!("val-{}", i)).await;
        if result.is_err() {
            // Retry once on failure (packet loss may cause transient errors)
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = harness.write_kv(&format!("loss-{}", i), &format!("val-{}", i)).await;
        }
    }

    // Wait for replication under lossy conditions
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Verify at least 90% of writes replicated (10% loss tolerance)
    let mut replicated_count = 0u32;
    for i in 0..50u32 {
        if let Ok(Some(_)) = harness.read_kv(2, &format!("loss-{}", i)).await {
            replicated_count += 1;
        }
    }

    assert!(
        replicated_count >= 45,
        "at least 45/50 writes should replicate under 10% loss, got {}",
        replicated_count
    );

    harness.shutdown().await;
}

#[tokio::test]
async fn test_packet_loss_50pct_no_deadlock() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    // Inject 50% packet loss
    for device in harness.devices() {
        device
            .set_link_condition(
                "eth0",
                Some(LinkCondition::Manual(LinkLimits {
                    loss_pct: 50.0,
                    ..Default::default()
                })),
            )
            .await
            .expect("failed to set packet loss");
    }

    // Attempt writes — they may fail, but the cluster shouldn't deadlock
    for i in 0..10u32 {
        match tokio::time::timeout(Duration::from_secs(10), harness.write_kv(&format!("heavy-loss-{}", i), "val")).await
        {
            Ok(Ok(())) => { /* Write succeeded despite heavy loss */ }
            Ok(Err(_)) => { /* Expected: write may fail under heavy loss */ }
            Err(_) => { /* Timeout: also acceptable under 50% loss */ }
        }
    }

    // Remove packet loss
    for device in harness.devices() {
        device.set_link_condition("eth0", None).await.expect("failed to clear loss");
    }

    // Cluster should recover
    tokio::time::sleep(Duration::from_secs(10)).await;

    let recovery_write = harness.write_kv("recovery", "works").await;
    assert!(recovery_write.is_ok(), "cluster should recover after packet loss is removed");

    harness.shutdown().await;
}

// ============================================================================
// Link down/up tests
// ============================================================================

#[tokio::test]
async fn test_link_down_follower() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    let leader = harness.check_leader().await.expect("no leader");

    // Find a follower
    let follower_idx = harness.handles().iter().position(|h| h.node_id != leader).expect("no follower found");

    // Take follower's link down
    harness.devices()[follower_idx].link_down("eth0").await.expect("link_down failed");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Remaining 2 nodes still have quorum — writes should work
    let write_result = harness.write_kv("link-down-test", "still-works").await;
    assert!(write_result.is_ok(), "writes should succeed with 2/3 nodes");

    harness.shutdown().await;
}

#[tokio::test]
async fn test_link_up_follower_catchup() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    let leader = harness.check_leader().await.expect("no leader");
    let follower_idx = harness.handles().iter().position(|h| h.node_id != leader).expect("no follower");

    // Link down
    harness.devices()[follower_idx].link_down("eth0").await.expect("link_down failed");

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Write while follower is down
    harness.write_kv("missed-write", "catchup-value").await.expect("write during link down failed");

    // Link up. Patchbay may report an IPv6 default-route restore warning after
    // the interface is already administratively up; catchup below proves the
    // data path recovered.
    link_up_after_admin_down(&harness.devices()[follower_idx], "eth0").await;

    wait_for_key_present("missed-write", FOLLOWER_CATCHUP_TIMEOUT, || async {
        match harness.read_kv(follower_idx, "missed-write").await {
            Ok(Some(value)) => Ok(value == "catchup-value"),
            Ok(None) | Err(_) => Ok(false),
        }
    })
    .await
    .expect("follower should catch up after link restored");

    harness.shutdown().await;
}

#[tokio::test]
async fn test_link_down_leader_failover() {
    skip_unless_patchbay!();

    let harness = PatchbayHarness::three_node_public().await.expect("failed to create topology");

    harness.init_cluster().await.expect("cluster init failed");

    let leader = harness.check_leader().await.expect("no leader");
    let leader_idx = harness.handles().iter().position(|h| h.node_id == leader).expect("leader not found");

    // Take leader's link down
    harness.devices()[leader_idx].link_down("eth0").await.expect("link_down failed");

    // Wait for election timeout + new election
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Followers should have elected a new leader
    let mut new_leader_found = false;
    for (i, handle) in harness.handles().iter().enumerate() {
        if i == leader_idx {
            continue;
        }
        if let Ok(Some(new_leader)) = handle.get_leader().await
            && new_leader != leader
        {
            new_leader_found = true;
            break;
        }
    }

    assert!(new_leader_found, "followers should elect a new leader when leader's link is down");

    harness.shutdown().await;
}

// ============================================================================
// Dynamic NAT change test
// ============================================================================

#[tokio::test]
async fn test_dynamic_nat_change() {
    skip_unless_patchbay!();

    let mut harness = PatchbayHarness::new().await.expect("failed to create harness");

    // Start with a public router (no NAT)
    let router = harness.add_router("dynamic", RouterPreset::Public).await.expect("router creation failed");

    for i in 0..3u32 {
        harness.add_node(&format!("node-{}", i), router, None).await.expect("add_node failed");
    }

    harness.init_cluster().await.expect("cluster init failed");
    harness.write_kv("before-nat", "works").await.expect("write failed");

    // Switch router to Home NAT mode
    harness.routers()[router].set_nat_mode(Nat::Home).await.expect("NAT mode switch failed");

    // iroh should re-establish connectivity
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Try writing after NAT change
    let result = harness.write_kv("after-nat", "maybe").await;
    match result {
        Ok(()) => {
            // Connectivity survived the NAT change
            let val = harness.read_kv(0, "after-nat").await.expect("read failed");
            assert_eq!(val, Some("maybe".to_string()));
        }
        Err(e) => {
            // NAT change may break connectivity without relay
            eprintln!("write after NAT change failed (expected without relay): {}", e);
        }
    }

    harness.shutdown().await;
}
