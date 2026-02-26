//! Cluster operations: crash/restart, leader checks, read/write, SQL, membership.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
#[cfg(feature = "sql")]
use aspen_core::SqlConsistency;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryError;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryRequest;
#[cfg(feature = "sql")]
use aspen_core::SqlQueryResult;
#[cfg(feature = "sql")]
use aspen_raft::StateMachineVariant;
use aspen_raft::madsim_network::MadsimNetworkFactory;
#[cfg(feature = "sql")]
use aspen_raft::node::RaftNode;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::types::AppRequest;
use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use openraft::Config;
use openraft::Raft;

use super::AspenRaftTester;
use super::config::DEFAULT_ELECTION_TIMEOUT_MAX_MS;
use super::config::DEFAULT_ELECTION_TIMEOUT_MIN_MS;
use super::config::DEFAULT_HEARTBEAT_INTERVAL_MS;
use super::config::LEADER_CHECK_BACKOFF_MAX_MS;
use super::config::LEADER_CHECK_BACKOFF_MIN_MS;
use super::config::LEADER_CHECK_RETRIES;
use super::node::RedbStoragePath;
use super::node::TestNode;
use super::node::create_test_raft_member_info;
use super::node::empty_artifact_builder;

impl AspenRaftTester {
    /// Crash a node (marks as failed in router).
    ///
    /// For persistent nodes, we need to properly shutdown the Raft instance
    /// to release database locks before the node can be restarted.
    pub async fn crash_node(&mut self, i: usize) {
        assert!(i < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(i as u64 + 1);

        self.router.mark_node_failed(node_id, true);
        self.nodes[i].connected().store(false, Ordering::SeqCst);

        // For Redb nodes, shutdown Raft to release database locks
        if self.nodes[i].redb_storage_path().is_some() {
            // Shutdown the Raft instance to release resources
            let _ = self.nodes[i].raft().shutdown().await;
        }

        self.metrics.node_crashes += 1;
        self.artifact =
            std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(format!("crash: node {}", i));
    }

    /// Restart a crashed node.
    ///
    /// For in-memory nodes, this only clears the failed status.
    /// For Redb nodes, this recreates the node with new storage to avoid lock conflicts,
    /// simulating a full crash recovery.
    pub async fn restart_node(&mut self, i: usize) {
        assert!(i < self.nodes.len(), "Invalid node index");
        let node_id = NodeId::from(i as u64 + 1);

        // For Redb nodes, actually recreate the node to simulate full restart
        if let Some(storage_path) = self.nodes[i].redb_storage_path() {
            let storage_path = storage_path.clone();
            self.restart_redb_node(i, node_id, &storage_path).await;
        } else {
            // For in-memory nodes, just clear the failed status
            self.nodes[i].connected().store(true, Ordering::SeqCst);
            self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                .add_event(format!("restart: node {} (in-memory, state preserved)", i));
        }

        self.router.mark_node_failed(node_id, false);
        self.metrics.node_restarts += 1;
    }

    /// Restart a Redb-backed node with fresh storage.
    async fn restart_redb_node(&mut self, i: usize, node_id: NodeId, storage_path: &RedbStoragePath) {
        // Recreate the node with the same Redb storage
        let raft_config = Config {
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL_MS,
            election_timeout_min: DEFAULT_ELECTION_TIMEOUT_MIN_MS,
            election_timeout_max: DEFAULT_ELECTION_TIMEOUT_MAX_MS,
            ..Default::default()
        };
        let raft_config = Arc::new(raft_config.validate().expect("invalid raft config"));

        // Create unique storage path for restart to avoid file lock conflicts in madsim
        // In madsim, all nodes run in the same process, so we can't reopen the same database
        // file that might still be locked. Instead, create a fresh database and let the
        // node rejoin the cluster and sync from peers (simulating full crash recovery).
        let restart_count = self.restart_counts.get(&i).copied().unwrap_or(0) + 1;
        self.restart_counts.insert(i, restart_count);

        let parent_dir = storage_path.db_path.parent().expect("storage path should have parent");
        let fresh_db_path = parent_dir.join(format!("shared-restart-{}.redb", restart_count));

        // Create fresh SharedRedbStorage (implements both log and state machine)
        let storage =
            SharedRedbStorage::new(&fresh_db_path, &node_id.to_string()).expect("failed to create fresh Redb storage");

        let network_factory = MadsimNetworkFactory::new(node_id, self.router.clone(), self.injector.clone());

        let raft = Raft::new(node_id, raft_config, network_factory, storage.clone(), storage.clone())
            .await
            .expect("failed to recreate raft instance");

        // Re-register with router
        self.router
            .register_node(node_id, format!("127.0.0.1:{}", 26000 + i), raft.clone())
            .expect("failed to re-register node");

        // Wrap storage in Arc and create RaftNode wrapper (only when sql feature is enabled)
        let storage = Arc::new(storage);
        #[cfg(feature = "sql")]
        let raft_node =
            Arc::new(RaftNode::new(node_id, Arc::new(raft.clone()), StateMachineVariant::Redb(storage.clone())));

        // Replace the node in our list with fresh storage path
        self.nodes[i] = TestNode::Redb {
            raft,
            #[cfg(feature = "sql")]
            raft_node,
            storage,
            connected: AtomicBool::new(true),
            storage_path: RedbStoragePath { db_path: fresh_db_path },
        };

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("restart: node {} with Redb storage", i));
    }

    /// Check for exactly one leader among connected nodes.
    ///
    /// Returns the leader's index (0-indexed) if exactly one leader is found.
    /// Uses random backoff like MadRaft for better election handling.
    pub async fn check_one_leader(&mut self) -> Option<usize> {
        let mut retries = LEADER_CHECK_RETRIES;

        while retries > 0 {
            // Random backoff using madsim's deterministic random
            let backoff = LEADER_CHECK_BACKOFF_MIN_MS
                + (madsim::rand::random::<u64>() % (LEADER_CHECK_BACKOFF_MAX_MS - LEADER_CHECK_BACKOFF_MIN_MS));
            madsim::time::sleep(Duration::from_millis(backoff)).await;

            if let Some((idx, id)) = self.find_agreed_leader() {
                let metrics = self.nodes[idx].raft().metrics().borrow().clone();
                self.metrics.elections += 1;
                self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                    .add_event(format!("leader: node {} (id={}) elected for term {}", idx, id, metrics.current_term));
                return Some(idx);
            }

            retries -= 1;
        }

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event("leader: no leader found after retries");
        None
    }

    /// Find a leader that all connected nodes agree on.
    ///
    /// Returns Some((leader_idx, leader_id)) if all connected nodes agree, None otherwise.
    fn find_agreed_leader(&self) -> Option<(usize, NodeId)> {
        let mut leader_id: Option<NodeId> = None;
        let mut leader_idx: Option<usize> = None;

        for node in self.nodes.iter() {
            if node.connected().load(Ordering::Relaxed) {
                let metrics = node.raft().metrics().borrow().clone();
                if let Some(current_leader) = metrics.current_leader {
                    let current_idx = (current_leader.0 - 1) as usize;
                    if !self.nodes[current_idx].connected().load(Ordering::Relaxed) {
                        return None;
                    }

                    match leader_id {
                        None => {
                            leader_id = Some(current_leader);
                            leader_idx = Some(current_idx);
                        }
                        Some(existing) if existing != current_leader => {
                            return None;
                        }
                        _ => {} // Agreement continues
                    }
                }
            }
        }

        leader_idx.map(|idx| (idx, leader_id.unwrap()))
    }

    /// Verify no split brain (at most one leader per term).
    pub fn check_no_split_brain(&self) -> Result<()> {
        let mut leaders_per_term: BTreeMap<u64, Vec<usize>> = BTreeMap::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let metrics = node.raft().metrics().borrow().clone();
            let term = metrics.current_term;
            if let Some(leader_id) = metrics.current_leader
                && leader_id == NodeId::from(i as u64 + 1)
            {
                leaders_per_term.entry(term).or_default().push(i);
            }
        }

        for (term, leaders) in leaders_per_term {
            if leaders.len() > 1 {
                anyhow::bail!("Split brain detected: term {} has {} leaders: {:?}", term, leaders.len(), leaders);
            }
        }

        Ok(())
    }

    /// Get the maximum log index across all nodes.
    pub fn max_log_size(&self) -> u64 {
        self.nodes
            .iter()
            .map(|n| {
                let metrics = n.raft().metrics().borrow().clone();
                metrics.last_log_index.unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }

    /// Perform a write operation through the leader.
    pub async fn write(&mut self, key: String, value: String) -> Result<()> {
        let leader_idx =
            self.check_one_leader().await.ok_or_else(|| anyhow::anyhow!("No leader available for write"))?;

        self.nodes[leader_idx]
            .raft()
            .client_write(AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            })
            .await
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("write: key='{}' completed", key));
        Ok(())
    }

    /// Read a value from the leader's state machine.
    ///
    /// Reads directly from the state machine (bypassing Raft read API).
    /// Works for in-memory and Redb nodes.
    pub async fn read(&mut self, key: &str) -> Result<Option<String>> {
        let leader_idx =
            self.check_one_leader().await.ok_or_else(|| anyhow::anyhow!("No leader available for read"))?;

        // Read depends on the node type
        let value = match &self.nodes[leader_idx] {
            TestNode::InMemory { state_machine, .. } => state_machine.get(key).await,
            TestNode::Redb { storage, .. } => {
                // SharedRedbStorage.get returns Result<Option<KvEntry>, SharedStorageError>
                storage.get(key).map(|opt| opt.map(|e| e.value)).unwrap_or(None)
            }
        };
        Ok(value)
    }

    // =========================================================================
    // SQL Query Operations (for Redb nodes)
    // =========================================================================

    /// Execute a SQL query through the current leader.
    ///
    /// Uses Linearizable consistency by default. Only works with Redb nodes.
    #[cfg(feature = "sql")]
    pub async fn execute_sql(&mut self, query: &str) -> Result<SqlQueryResult> {
        self.execute_sql_with_consistency(query, SqlConsistency::Linearizable).await
    }

    /// Execute a SQL query with specific consistency level.
    #[cfg(feature = "sql")]
    pub async fn execute_sql_with_consistency(
        &mut self,
        query: &str,
        consistency: SqlConsistency,
    ) -> Result<SqlQueryResult> {
        let leader_idx =
            self.check_one_leader().await.ok_or_else(|| anyhow::anyhow!("No leader available for SQL query"))?;

        self.execute_sql_on_node(leader_idx, query, consistency).await
    }

    /// Execute a SQL query on a specific node.
    ///
    /// Useful for testing stale reads on followers or verifying data replication.
    #[cfg(feature = "sql")]
    pub async fn execute_sql_on_node(
        &mut self,
        node_idx: usize,
        query: &str,
        consistency: SqlConsistency,
    ) -> Result<SqlQueryResult> {
        assert!(node_idx < self.nodes.len(), "Invalid node index");

        let raft_node = self.nodes[node_idx]
            .raft_node()
            .ok_or_else(|| anyhow::anyhow!("SQL not supported on this node type (use Redb backend)"))?;

        let request = SqlQueryRequest {
            query: query.to_string(),
            params: vec![],
            consistency,
            limit: None,
            timeout_ms: Some(10_000), // 10 second timeout for tests
        };

        use aspen_core::SqlQueryExecutor;
        let result = raft_node.execute_sql(request).await.map_err(sql_query_error_to_anyhow)?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(format!(
            "sql: query='{}' returned {} rows",
            query.chars().take(50).collect::<String>(),
            result.row_count
        ));

        Ok(result)
    }

    // =========================================================================
    // Membership Change Operations
    // =========================================================================

    /// Add a learner node to the cluster.
    ///
    /// Learners replicate data but don't participate in consensus votes.
    /// This is typically used before promoting a node to voter.
    pub async fn add_learner(&mut self, node_idx: usize) -> Result<()> {
        assert!(node_idx < self.nodes.len(), "Invalid node index");

        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for add_learner"))?;

        let node_id = NodeId::from(node_idx as u64 + 1);
        let member_info = create_test_raft_member_info(node_id);

        self.nodes[leader_idx]
            .raft()
            .add_learner(node_id, member_info, true)
            .await
            .map_err(|e| anyhow::anyhow!("add_learner failed: {}", e))?;

        self.metrics.membership_changes += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("membership: added node {} as learner", node_idx));

        Ok(())
    }

    /// Change cluster membership to a new set of voters.
    ///
    /// This reconfigures the Raft cluster to use a new set of voting members.
    /// The change is applied through joint consensus for safety.
    pub async fn change_membership(&mut self, voter_indices: &[usize]) -> Result<()> {
        assert!(!voter_indices.is_empty(), "Must have at least one voter");
        for &idx in voter_indices {
            assert!(idx < self.nodes.len(), "Invalid node index: {}", idx);
        }

        let leader_idx = self
            .check_one_leader()
            .await
            .ok_or_else(|| anyhow::anyhow!("No leader available for change_membership"))?;

        let members: BTreeSet<NodeId> = voter_indices.iter().map(|&i| NodeId::from(i as u64 + 1)).collect();

        self.nodes[leader_idx]
            .raft()
            .change_membership(members.clone(), false)
            .await
            .map_err(|e| anyhow::anyhow!("change_membership failed: {}", e))?;

        self.metrics.membership_changes += 1;
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("membership: changed voters to {:?}", voter_indices));

        Ok(())
    }

    /// Get the current cluster membership state.
    ///
    /// Returns a tuple of (voters, learners) as 0-based node indices.
    pub fn get_membership(&self) -> (Vec<usize>, Vec<usize>) {
        // Find a connected node to query
        for node in &self.nodes {
            if node.connected().load(Ordering::Relaxed) {
                let metrics = node.raft().metrics().borrow().clone();
                let membership = metrics.membership_config.membership();
                let voters: Vec<usize> = membership.voter_ids().map(|id| (id.0 - 1) as usize).collect();
                let learners: Vec<usize> = membership.learner_ids().map(|id| (id.0 - 1) as usize).collect();
                return (voters, learners);
            }
        }
        (vec![], vec![])
    }

    /// Wait for all connected nodes to reach the same log index.
    ///
    /// This is useful after membership changes to ensure replication.
    pub async fn wait_for_log_sync(&mut self, timeout_secs: u64) -> Result<()> {
        let deadline = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        while start.elapsed() < deadline {
            let mut indices: Vec<u64> = Vec::new();

            for node in &self.nodes {
                if node.connected().load(Ordering::Relaxed) {
                    let metrics = node.raft().metrics().borrow().clone();
                    if let Some(applied) = metrics.last_applied {
                        indices.push(applied.index);
                    }
                }
            }

            // Check if all connected nodes have the same applied index
            if !indices.is_empty() && indices.iter().all(|&i| i == indices[0]) {
                self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
                    .add_event(format!("sync: all nodes at log index {}", indices[0]));
                return Ok(());
            }

            madsim::time::sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!("Timeout waiting for log sync after {} seconds", timeout_secs)
    }

    /// Add a custom event to the artifact trace.
    pub fn add_event(&mut self, event: impl Into<String>) {
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder()).add_event(event);
    }

    /// Delete a key from the key-value store.
    pub async fn delete(&mut self, key: String) -> Result<()> {
        let leader_idx = self.check_one_leader().await.ok_or_else(|| anyhow::anyhow!("No leader available"))?;

        let req = AppRequest::Delete { key: key.clone() };

        let res = self.nodes[leader_idx].raft().client_write(req).await?;

        // Track event
        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("Delete key '{}' via node {} (response: {:?})", key, leader_idx, res.data));

        Ok(())
    }

    /// Trigger an election on a specific node.
    pub async fn trigger_election(&mut self, node_id: u64) -> Result<()> {
        if node_id >= self.nodes.len() as u64 {
            return Err(anyhow::anyhow!("Invalid node ID: {}", node_id));
        }

        self.nodes[node_id as usize].raft().trigger().elect().await?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("Triggered election on node {}", node_id));

        Ok(())
    }

    /// Trigger a snapshot on a specific node.
    ///
    /// This manually triggers the state machine to build a snapshot.
    /// Typically called on the leader to force log compaction.
    pub async fn trigger_snapshot(&mut self, node_id: u64) -> Result<()> {
        if node_id >= self.nodes.len() as u64 {
            return Err(anyhow::anyhow!("Invalid node ID: {}", node_id));
        }

        self.nodes[node_id as usize].raft().trigger().snapshot().await?;

        self.artifact = std::mem::replace(&mut self.artifact, empty_artifact_builder())
            .add_event(format!("Triggered snapshot on node {}", node_id));

        Ok(())
    }

    /// Get metrics for a specific node.
    pub fn get_metrics(&self, node_id: u64) -> Option<openraft::RaftMetrics<AppTypeConfig>> {
        if node_id >= self.nodes.len() as u64 {
            return None;
        }
        Some(self.nodes[node_id as usize].raft().metrics().borrow().clone())
    }
}

/// Convert SqlQueryError to anyhow::Error.
#[cfg(feature = "sql")]
fn sql_query_error_to_anyhow(e: SqlQueryError) -> anyhow::Error {
    match e {
        SqlQueryError::NotLeader { leader } => {
            anyhow::anyhow!("Not leader, leader hint: {:?}", leader)
        }
        SqlQueryError::NotSupported { backend } => {
            anyhow::anyhow!("SQL not supported on {} backend", backend)
        }
        SqlQueryError::SyntaxError { message } => {
            anyhow::anyhow!("SQL syntax error: {}", message)
        }
        SqlQueryError::ExecutionFailed { reason } => {
            anyhow::anyhow!("SQL execution failed: {}", reason)
        }
        SqlQueryError::Timeout { duration_ms } => {
            anyhow::anyhow!("SQL query timed out after {}ms", duration_ms)
        }
        SqlQueryError::QueryNotAllowed { reason } => {
            anyhow::anyhow!("SQL query not allowed: {}", reason)
        }
        SqlQueryError::QueryTooLarge { size, max } => {
            anyhow::anyhow!("SQL query too large: {} bytes (max {})", size, max)
        }
        SqlQueryError::TooManyParams { count, max } => {
            anyhow::anyhow!("Too many SQL params: {} (max {})", count, max)
        }
    }
}
