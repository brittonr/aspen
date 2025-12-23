//! Common benchmark infrastructure for Aspen.
//!
//! Provides cluster setup helpers and workload generators following
//! Tiger Style principles (bounded resources, pre-allocated collections).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use openraft::{Config, ServerState};

use aspen::raft::types::{NodeId, RaftMemberInfo};
use aspen::testing::{AspenRouter, create_test_raft_member_info};

/// Default timeout for cluster operations.
pub fn default_timeout() -> Option<Duration> {
    Some(Duration::from_secs(10))
}

/// Setup a single-node cluster for benchmarking.
///
/// Returns the router and the leader node ID.
/// The cluster is fully initialized and ready for operations.
pub async fn setup_single_node_cluster() -> Result<(AspenRouter, NodeId)> {
    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config);
    router.new_raft_node(0).await?;

    // Initialize single-node cluster
    let node = router.get_raft_handle(0)?;
    let mut members: BTreeMap<NodeId, RaftMemberInfo> = BTreeMap::new();
    members.insert(NodeId::from(0), create_test_raft_member_info(0));
    node.initialize(members).await?;

    // Wait for leader election
    router
        .wait(0, default_timeout())
        .state(ServerState::Leader, "node 0 is leader")
        .await?;

    Ok((router, NodeId::from(0)))
}

/// Setup a three-node cluster for benchmarking.
///
/// Returns the router and the leader node ID.
/// The cluster is fully initialized with all nodes as voters.
#[allow(dead_code)]
pub async fn setup_three_node_cluster() -> Result<(AspenRouter, NodeId)> {
    let config = Arc::new(
        Config {
            enable_tick: true,
            heartbeat_interval: 100,
            election_timeout_min: 500,
            election_timeout_max: 1000,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config);

    // Create all nodes
    router.new_raft_node(0).await?;
    router.new_raft_node(1).await?;
    router.new_raft_node(2).await?;

    // Initialize cluster with node 0 as initial leader
    let node0 = router.get_raft_handle(0)?;
    let mut members: BTreeMap<NodeId, RaftMemberInfo> = BTreeMap::new();
    members.insert(NodeId::from(0), create_test_raft_member_info(0));
    members.insert(NodeId::from(1), create_test_raft_member_info(1));
    members.insert(NodeId::from(2), create_test_raft_member_info(2));
    node0.initialize(members).await?;

    // Wait for leader election
    router
        .wait(0, default_timeout())
        .state(ServerState::Leader, "leader elected")
        .await?;

    Ok((router, NodeId::from(0)))
}

/// Pre-populate a cluster with test data.
///
/// Writes `count` key-value pairs with format "key-{i}" -> "value-{i}".
/// Uses bounded batch size for Tiger Style compliance.
pub async fn populate_test_data(router: &AspenRouter, node_id: NodeId, count: usize) -> Result<()> {
    // Tiger Style: process in bounded batches to avoid memory spikes
    const BATCH_SIZE: usize = 100;

    for batch_start in (0..count).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(count);

        for i in batch_start..batch_end {
            let key = format!("key-{:06}", i);
            let value = format!("value-{:06}", i);
            router
                .write(node_id, key, value)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
    }

    // Wait for all writes to be applied
    let expected_log_index = count as u64 + 1; // +1 for initialization
    router
        .wait(node_id, default_timeout())
        .log_index_at_least(Some(expected_log_index), "data populated")
        .await?;

    Ok(())
}

/// Generate key-value pairs for batch operations.
///
/// Returns a pre-allocated Vec with the specified count.
/// Tiger Style: bounded size, pre-allocated capacity.
pub fn generate_kv_pairs(count: usize, prefix: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::with_capacity(count);
    for i in 0..count {
        pairs.push((format!("{}-{:06}", prefix, i), format!("value-{:06}", i)));
    }
    pairs
}

/// YCSB-style workload patterns.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum WorkloadPattern {
    /// 50% read, 50% update (session store pattern)
    ReadHeavy,
    /// 95% read, 5% update (photo tagging pattern)
    MostlyRead,
    /// 5% read, 95% write (write-heavy ingest)
    WriteHeavy,
    /// 50% read, 50% write (mixed workload)
    Mixed,
}

impl WorkloadPattern {
    /// Returns (read_percentage, write_percentage).
    #[allow(dead_code)]
    pub fn ratios(&self) -> (u8, u8) {
        match self {
            Self::ReadHeavy => (50, 50),
            Self::MostlyRead => (95, 5),
            Self::WriteHeavy => (5, 95),
            Self::Mixed => (50, 50),
        }
    }
}

/// Operation type for workload generation.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Operation {
    Read { key: String },
    Write { key: String, value: String },
}

/// Generate a deterministic workload based on pattern.
///
/// Tiger Style: bounded size, pre-allocated capacity.
#[allow(dead_code)]
pub fn generate_workload(
    pattern: WorkloadPattern,
    count: usize,
    key_space: usize,
) -> Vec<Operation> {
    let (read_pct, _write_pct) = pattern.ratios();
    let mut ops = Vec::with_capacity(count);

    for i in 0..count {
        let key_idx = i % key_space;
        let key = format!("key-{:06}", key_idx);

        // Deterministic pattern based on operation index
        if (i * 100 / count) < read_pct as usize {
            ops.push(Operation::Read { key });
        } else {
            let value = format!("value-{:06}-updated", i);
            ops.push(Operation::Write { key, value });
        }
    }

    ops
}
