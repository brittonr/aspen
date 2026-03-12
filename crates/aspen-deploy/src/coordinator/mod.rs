//! Deployment coordinator — orchestrates rolling upgrades across a cluster.
//!
//! The coordinator runs on the Raft leader and manages the full deployment lifecycle:
//! 1. Accept a deployment request, write initial state to KV
//! 2. Iterate through nodes (followers first, leader last)
//! 3. For each node: check quorum safety → send NodeUpgrade RPC → poll health
//! 4. On completion, archive to history and clean up
//!
//! State is persisted in KV via CAS writes, so a new leader can resume
//! after failover.

mod health;
mod history;
#[cfg(feature = "iroh")]
pub mod iroh_rpc;
pub mod rpc;

use std::sync::Arc;

use aspen_constants::api::DEPLOY_HEALTH_TIMEOUT_SECS;
use aspen_constants::api::DEPLOY_STATUS_POLL_INTERVAL_SECS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteRequest;
use aspen_traits::ClusterController;
use aspen_traits::KeyValueStore;
pub use rpc::NodeRpcClient;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::DEPLOY_CURRENT_KEY;
use crate::DeployArtifact;
use crate::DeployStrategy;
use crate::DeploymentRecord;
use crate::DeploymentStatus;
use crate::NodeDeployState;
use crate::NodeDeployStatus;
use crate::error::DeployError;
use crate::error::Result;
use crate::verified::quorum;

/// Deployment coordinator — manages the lifecycle of rolling deployments.
///
/// The coordinator uses the KV store for persistent state and a `NodeRpcClient`
/// trait for sending upgrade/rollback/health RPCs to individual nodes. This
/// makes it testable with mock implementations.
pub struct DeploymentCoordinator<
    K: KeyValueStore + ?Sized,
    R: NodeRpcClient,
    C: ClusterController + ?Sized = dyn ClusterController,
> {
    /// KV store for deployment state persistence.
    kv: Arc<K>,
    /// RPC client for communicating with individual nodes.
    rpc_client: Arc<R>,
    /// Cluster controller for leadership transfer.
    cluster_controller: Option<Arc<C>>,
    /// This node's ID (for leader-last ordering).
    node_id: u64,
    /// Health check timeout per node.
    health_timeout_secs: u64,
    /// Poll interval for health checks.
    poll_interval_secs: u64,
}

impl<K: KeyValueStore + ?Sized, R: NodeRpcClient, C: ClusterController + ?Sized> DeploymentCoordinator<K, R, C> {
    /// Create a new deployment coordinator (without cluster controller).
    ///
    /// Leadership transfer during deploy will be skipped. Use
    /// `with_cluster_controller` for production deployments.
    pub fn new(kv: Arc<K>, rpc_client: Arc<R>, node_id: u64) -> Self {
        Self {
            kv,
            rpc_client,
            cluster_controller: None,
            node_id,
            health_timeout_secs: DEPLOY_HEALTH_TIMEOUT_SECS,
            poll_interval_secs: DEPLOY_STATUS_POLL_INTERVAL_SECS,
        }
    }

    /// Create with custom timeouts (for testing).
    pub fn with_timeouts(
        kv: Arc<K>,
        rpc_client: Arc<R>,
        node_id: u64,
        health_timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            kv,
            rpc_client,
            cluster_controller: None,
            node_id,
            health_timeout_secs,
            poll_interval_secs,
        }
    }

    /// Create a coordinator with a cluster controller for leadership transfer.
    pub fn with_cluster_controller(kv: Arc<K>, rpc_client: Arc<R>, cluster_controller: Arc<C>, node_id: u64) -> Self {
        Self {
            kv,
            rpc_client,
            cluster_controller: Some(cluster_controller),
            node_id,
            health_timeout_secs: DEPLOY_HEALTH_TIMEOUT_SECS,
            poll_interval_secs: DEPLOY_STATUS_POLL_INTERVAL_SECS,
        }
    }

    /// Create with cluster controller and custom timeouts (for testing).
    pub fn with_cluster_controller_and_timeouts(
        kv: Arc<K>,
        rpc_client: Arc<R>,
        cluster_controller: Arc<C>,
        node_id: u64,
        health_timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            kv,
            rpc_client,
            cluster_controller: Some(cluster_controller),
            node_id,
            health_timeout_secs,
            poll_interval_secs,
        }
    }

    // ========================================================================
    // 5.2: start_deployment
    // ========================================================================

    /// Start a new deployment. CAS-writes `_sys:deploy:current` with `Pending` status.
    ///
    /// Fails if an existing deployment is in a non-terminal state (Pending, Deploying,
    /// RollingBack).
    pub async fn start_deployment(
        &self,
        deploy_id: String,
        artifact: DeployArtifact,
        strategy: DeployStrategy,
        node_ids: &[u64],
        now_ms: u64,
    ) -> Result<DeploymentRecord> {
        // Check for existing active deployment
        match self.read_current_deployment().await {
            Ok(existing) => {
                if !existing.status.is_terminal() {
                    return Err(DeployError::DeploymentInProgress {
                        deploy_id: existing.deploy_id,
                    });
                }
                // Terminal deployment exists — overwrite it
            }
            Err(DeployError::NoDeploymentFound) => {
                // No existing deployment — proceed
            }
            Err(e) => return Err(e),
        }

        let record = DeploymentRecord::new(deploy_id, artifact, strategy, node_ids, now_ms);
        self.write_current_deployment(&record, None).await?;

        info!(
            deploy_id = %record.deploy_id,
            nodes = node_ids.len(),
            "deployment created with Pending status"
        );

        Ok(record)
    }

    // ========================================================================
    // 5.3: run_deployment
    // ========================================================================

    /// Run the deployment: iterate nodes, check quorum, send upgrade RPCs, poll health.
    ///
    /// Followers are upgraded first, the leader (self) is upgraded last.
    /// State is persisted to KV at each step so failover recovery works.
    pub async fn run_deployment(&self, deploy_id: &str) -> Result<DeploymentRecord> {
        // Transition to Deploying
        let mut record = self.read_current_deployment().await?;
        if record.deploy_id != deploy_id {
            return Err(DeployError::NoDeploymentFound);
        }

        record.status = DeploymentStatus::Deploying;
        record.updated_at_ms = self.now_ms();
        let expected = self.read_current_value().await?;
        self.write_current_deployment(&record, expected.as_deref()).await?;

        // Sort nodes: followers first, leader (self) last
        let (followers, is_leader_in_list) = self.partition_nodes(&record.nodes);

        let voter_count = record.nodes.len() as u32;
        let max_concurrent = match &record.strategy {
            DeployStrategy::Rolling { max_concurrent } => {
                let safe_max = quorum::max_concurrent_upgrades(voter_count);
                (*max_concurrent).min(safe_max)
            }
        };

        // Upgrade followers
        for follower_id in &followers {
            record = self.upgrade_single_node(&record, *follower_id, voter_count, max_concurrent).await?;
        }

        // Upgrade leader last (self)
        if is_leader_in_list {
            match self.prepare_leader_upgrade(&record).await {
                Ok(r) => {
                    // No leadership transfer happened (single-node, no controller, or
                    // transfer timed out) — proceed with direct self-upgrade.
                    record = r;
                    record = self.upgrade_single_node(&record, self.node_id, voter_count, max_concurrent).await?;
                }
                Err(DeployError::LeadershipTransferred { target_node_id }) => {
                    // Leadership moved to another node. The new leader's
                    // leader_resume.rs watcher will pick up the deployment and
                    // upgrade us (the old leader, now a follower) via iroh RPC.
                    info!(
                        deploy_id = %record.deploy_id,
                        target_node_id,
                        "leadership transferred, new leader will resume deployment"
                    );
                    return Ok(record);
                }
                Err(e) => return Err(e),
            }
        }

        // All nodes upgraded — finalize
        record = self.finalize_deployment(record).await?;

        Ok(record)
    }

    // ========================================================================
    // 5.4 + 5.5: Node upgrade with health polling and failure handling
    // ========================================================================

    /// Upgrade a single node: quorum check → send RPC → poll health → update status.
    async fn upgrade_single_node(
        &self,
        record: &DeploymentRecord,
        target_node_id: u64,
        voter_count: u32,
        _max_concurrent: u32,
    ) -> Result<DeploymentRecord> {
        let mut record = record.clone();

        // Quorum safety check.
        // For clusters with <= 2 voters, quorum can't be maintained during any
        // upgrade — accept brief unavailability (same as single-node).
        // For larger clusters, verify we won't drop below quorum.
        let healthy = record.count_healthy();
        let upgrading = record.count_upgrading();
        let available = healthy + self.count_pending(&record);
        if voter_count > 2 && !quorum::can_upgrade_node(available, upgrading, voter_count) {
            return Err(DeployError::QuorumSafetyViolation {
                reason: format!(
                    "cannot upgrade node {target_node_id}: {healthy} healthy, {upgrading} upgrading, {voter_count} total voters"
                ),
            });
        }

        // Update node status to Draining
        self.update_node_status(&mut record, target_node_id, NodeDeployStatus::Draining).await?;

        // Send NodeUpgrade RPC
        let artifact_ref = record.artifact.display_ref().to_string();
        match self.rpc_client.send_upgrade(target_node_id, &record.deploy_id, &artifact_ref).await {
            Ok(()) => {
                self.update_node_status(&mut record, target_node_id, NodeDeployStatus::Upgrading).await?;
            }
            Err(e) => {
                let reason = e.to_string();
                self.update_node_status(&mut record, target_node_id, NodeDeployStatus::Failed(reason.clone()))
                    .await?;
                return self.handle_node_failure(record, target_node_id, &reason).await;
            }
        }

        // Poll health
        match self.poll_node_health(target_node_id).await {
            Ok(()) => {
                self.update_node_status(&mut record, target_node_id, NodeDeployStatus::Healthy).await?;
                info!(node_id = target_node_id, deploy_id = %record.deploy_id, "node upgrade complete");
            }
            Err(e) => {
                let reason = e.to_string();
                self.update_node_status(&mut record, target_node_id, NodeDeployStatus::Failed(reason.clone()))
                    .await?;
                return self.handle_node_failure(record, target_node_id, &reason).await;
            }
        }

        Ok(record)
    }

    /// Handle a node failure: mark deployment Failed, stop upgrading.
    async fn handle_node_failure(
        &self,
        mut record: DeploymentRecord,
        failed_node_id: u64,
        reason: &str,
    ) -> Result<DeploymentRecord> {
        error!(
            deploy_id = %record.deploy_id,
            node_id = failed_node_id,
            reason = reason,
            "node upgrade failed, halting deployment"
        );
        record.status = DeploymentStatus::Failed;
        record.error = Some(format!("node {failed_node_id}: {reason}"));
        record.updated_at_ms = self.now_ms();
        let expected = self.read_current_value().await?;
        self.write_current_deployment(&record, expected.as_deref()).await?;
        Err(DeployError::NodeUpgradeFailed {
            node_id: failed_node_id,
            reason: reason.to_string(),
        })
    }

    // ========================================================================
    // 5.6: Leader-last logic
    // ========================================================================

    /// Transfer leadership to an already-upgraded follower before self-upgrade.
    ///
    /// 1. Find first `Healthy` follower (already upgraded and health-checked)
    /// 2. Persist the deployment record (so the new leader can find it)
    /// 3. Call `cluster_controller.transfer_leader(target_id)`
    /// 4. Poll metrics until leadership transfers (bounded timeout)
    /// 5. Return `LeadershipTransferred` so `run_deployment()` returns early
    ///
    /// The new leader's `leader_resume.rs` watcher picks up the in-progress
    /// deployment and upgrades the old leader as a regular follower over iroh RPC.
    async fn prepare_leader_upgrade(&self, record: &DeploymentRecord) -> Result<DeploymentRecord> {
        let cc = match &self.cluster_controller {
            Some(cc) => cc,
            None => {
                // No cluster controller — fall back to direct self-upgrade (single-node or test)
                info!(
                    deploy_id = %record.deploy_id,
                    node_id = self.node_id,
                    "no cluster controller, skipping leadership transfer"
                );
                return Ok(record.clone());
            }
        };

        // Find first Healthy follower to transfer to
        let target = record
            .nodes
            .iter()
            .find(|n| n.node_id != self.node_id && matches!(n.status, NodeDeployStatus::Healthy))
            .map(|n| n.node_id);

        let target_node_id = match target {
            Some(id) => id,
            None => {
                // Single-node cluster or no healthy followers
                info!(
                    deploy_id = %record.deploy_id,
                    "no healthy follower for leadership transfer, proceeding with direct upgrade"
                );
                return Ok(record.clone());
            }
        };

        info!(
            deploy_id = %record.deploy_id,
            node_id = self.node_id,
            target_node_id,
            "transferring leadership to upgraded follower"
        );

        // Persist record before transfer so the new leader can find it
        let expected = self.read_current_value().await?;
        self.write_current_deployment(record, expected.as_deref()).await?;

        // Transfer leadership
        cc.transfer_leader(target_node_id).await.map_err(|e| DeployError::ControlPlaneError {
            reason: format!("leadership transfer to node {target_node_id} failed: {e}"),
        })?;

        // Poll metrics until leadership actually moves (bounded timeout)
        let timeout = std::time::Duration::from_secs(aspen_constants::api::DEPLOY_LEADER_TRANSFER_TIMEOUT_SECS);
        let start = tokio::time::Instant::now();
        loop {
            if tokio::time::Instant::now() - start > timeout {
                warn!(
                    deploy_id = %record.deploy_id,
                    target_node_id,
                    "leadership transfer timed out, proceeding with direct upgrade"
                );
                return Ok(record.clone());
            }

            if let Ok(metrics) = cc.get_metrics().await
                && metrics.current_leader != Some(self.node_id)
            {
                info!(
                    deploy_id = %record.deploy_id,
                    new_leader = ?metrics.current_leader,
                    "leadership transferred successfully"
                );
                return Err(DeployError::LeadershipTransferred { target_node_id });
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    // ========================================================================
    // 5.7: Failover recovery
    // ========================================================================

    /// Check for an in-progress deployment on leader election / startup.
    ///
    /// If `_sys:deploy:current` has status `Deploying`, attempt to resume.
    /// Returns `Ok(Some(record))` if a deployment was resumed and completed,
    /// `Ok(None)` if no in-progress deployment was found.
    pub async fn check_and_resume(&self) -> Result<Option<DeploymentRecord>> {
        let record = match self.read_current_deployment().await {
            Ok(r) => r,
            Err(DeployError::NoDeploymentFound) => return Ok(None),
            Err(e) => return Err(e),
        };

        match record.status {
            DeploymentStatus::Deploying => {
                info!(
                    deploy_id = %record.deploy_id,
                    "found in-progress deployment after leader election, resuming"
                );
                // Check each node's actual state
                let resumed = self.resume_deployment(record).await?;
                Ok(Some(resumed))
            }
            _ => {
                // Not in deploying state — nothing to resume
                Ok(None)
            }
        }
    }

    /// Resume a deployment from the persisted node states.
    ///
    /// Nodes already marked Healthy are skipped. Nodes in Draining/Upgrading/Restarting
    /// are re-checked via health poll. Pending nodes are upgraded normally.
    async fn resume_deployment(&self, record: DeploymentRecord) -> Result<DeploymentRecord> {
        let mut record = record;
        let voter_count = record.nodes.len() as u32;
        let max_concurrent = match &record.strategy {
            DeployStrategy::Rolling { max_concurrent } => {
                let safe_max = quorum::max_concurrent_upgrades(voter_count);
                (*max_concurrent).min(safe_max)
            }
        };

        // Process nodes that aren't terminal
        let node_ids: Vec<(u64, NodeDeployStatus)> =
            record.nodes.iter().map(|n| (n.node_id, n.status.clone())).collect();

        for (nid, status) in &node_ids {
            match status {
                NodeDeployStatus::Healthy => continue,
                NodeDeployStatus::Failed(_) => {
                    // A previous node failed — deployment stays Failed
                    if !matches!(record.status, DeploymentStatus::Failed) {
                        record.status = DeploymentStatus::Failed;
                        record.error = Some(format!("node {nid} was in failed state during resume"));
                        record.updated_at_ms = self.now_ms();
                        let expected = self.read_current_value().await?;
                        self.write_current_deployment(&record, expected.as_deref()).await?;
                    }
                    return Err(DeployError::NodeUpgradeFailed {
                        node_id: *nid,
                        reason: "node was in failed state when deployment was resumed".to_string(),
                    });
                }
                NodeDeployStatus::Draining | NodeDeployStatus::Upgrading | NodeDeployStatus::Restarting => {
                    // Node was mid-upgrade — check if it's now healthy
                    match self.poll_node_health(*nid).await {
                        Ok(()) => {
                            self.update_node_status(&mut record, *nid, NodeDeployStatus::Healthy).await?;
                        }
                        Err(e) => {
                            let reason = e.to_string();
                            self.update_node_status(&mut record, *nid, NodeDeployStatus::Failed(reason.clone()))
                                .await?;
                            return self.handle_node_failure(record, *nid, &reason).await;
                        }
                    }
                }
                NodeDeployStatus::Pending => {
                    // Not yet started — upgrade normally
                    record = self.upgrade_single_node(&record, *nid, voter_count, max_concurrent).await?;
                }
            }
        }

        // All nodes processed — check if we can finalize
        if record.count_healthy() == voter_count {
            record = self.finalize_deployment(record).await?;
        }

        Ok(record)
    }

    // ========================================================================
    // 5.8: rollback_deployment
    // ========================================================================

    /// Roll back the current or most recent deployment.
    ///
    /// Sends `NodeRollback` to each node that was upgraded (Healthy status),
    /// in rolling fashion.
    pub async fn rollback_deployment(&self) -> Result<DeploymentRecord> {
        let mut record = self.read_current_or_latest().await?;

        // Only Completed or Failed deployments can be rolled back
        if !matches!(record.status, DeploymentStatus::Completed | DeploymentStatus::Failed) {
            return Err(DeployError::NotRollbackEligible {
                deploy_id: record.deploy_id.clone(),
                status: format!("{:?}", record.status),
            });
        }

        record.status = DeploymentStatus::RollingBack;
        record.updated_at_ms = self.now_ms();
        let expected = self.read_current_value().await?;
        self.write_current_deployment(&record, expected.as_deref()).await?;

        info!(deploy_id = %record.deploy_id, "starting rollback");

        // Rollback nodes that were successfully upgraded (Healthy)
        let upgraded_nodes: Vec<u64> = record
            .nodes
            .iter()
            .filter(|n| matches!(n.status, NodeDeployStatus::Healthy))
            .map(|n| n.node_id)
            .collect();

        let mut has_rollback_failure = false;
        for nid in &upgraded_nodes {
            match self.rpc_client.send_rollback(*nid, &record.deploy_id).await {
                Ok(()) => {
                    self.update_node_status(&mut record, *nid, NodeDeployStatus::Pending).await?;
                    info!(node_id = nid, "node rollback complete");
                }
                Err(e) => {
                    let reason = e.to_string();
                    warn!(node_id = nid, reason = %reason, "node rollback failed");
                    self.update_node_status(&mut record, *nid, NodeDeployStatus::Failed(reason)).await?;
                    has_rollback_failure = true;
                }
            }
        }

        if has_rollback_failure {
            record.status = DeploymentStatus::Failed;
            record.error = Some("one or more nodes failed during rollback".to_string());
        } else {
            record.status = DeploymentStatus::RolledBack;
        }

        record.updated_at_ms = self.now_ms();
        let expected = self.read_current_value().await?;
        self.write_current_deployment(&record, expected.as_deref()).await?;

        // Archive
        history::archive_deployment(&*self.kv, &record).await?;

        Ok(record)
    }

    // ========================================================================
    // 5.9: get_status
    // ========================================================================

    /// Get the current deployment status.
    ///
    /// Reads `_sys:deploy:current`. If not found, falls back to the latest
    /// history entry.
    pub async fn get_status(&self) -> Result<DeploymentRecord> {
        self.read_current_or_latest().await
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Read the current deployment record from KV.
    async fn read_current_deployment(&self) -> Result<DeploymentRecord> {
        match self.kv.read(ReadRequest::new(DEPLOY_CURRENT_KEY)).await {
            Ok(result) => match result.kv {
                Some(entry) => {
                    serde_json::from_str(&entry.value).map_err(|e| DeployError::SerdeError { reason: e.to_string() })
                }
                None => Err(DeployError::NoDeploymentFound),
            },
            Err(KeyValueStoreError::NotFound { .. }) => Err(DeployError::NoDeploymentFound),
            Err(e) => Err(DeployError::KvError { reason: e.to_string() }),
        }
    }

    /// Read the raw value of the current deployment key (for CAS).
    async fn read_current_value(&self) -> Result<Option<String>> {
        match self.kv.read(ReadRequest::new(DEPLOY_CURRENT_KEY)).await {
            Ok(result) => Ok(result.kv.map(|e| e.value)),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(DeployError::KvError { reason: e.to_string() }),
        }
    }

    /// Write the deployment record to `_sys:deploy:current` with CAS.
    async fn write_current_deployment(&self, record: &DeploymentRecord, expected: Option<&str>) -> Result<()> {
        let value = serde_json::to_string(record).map_err(|e| DeployError::SerdeError { reason: e.to_string() })?;

        let request = match expected {
            Some(exp) => WriteRequest::compare_and_swap(DEPLOY_CURRENT_KEY, Some(exp.to_string()), value),
            None => WriteRequest::set(DEPLOY_CURRENT_KEY, value),
        };

        self.kv.write(request).await.map_err(|e| match &e {
            KeyValueStoreError::CompareAndSwapFailed { .. } => DeployError::ConcurrentModification {
                key: DEPLOY_CURRENT_KEY.to_string(),
            },
            _ => DeployError::KvError { reason: e.to_string() },
        })?;

        Ok(())
    }

    /// Read current deployment, falling back to latest history entry.
    async fn read_current_or_latest(&self) -> Result<DeploymentRecord> {
        match self.read_current_deployment().await {
            Ok(record) => Ok(record),
            Err(DeployError::NoDeploymentFound) => {
                // Try latest history entry
                history::read_latest_history(&*self.kv).await
            }
            Err(e) => Err(e),
        }
    }

    /// Update a node's status in the record and persist to KV.
    async fn update_node_status(
        &self,
        record: &mut DeploymentRecord,
        target_node_id: u64,
        status: NodeDeployStatus,
    ) -> Result<()> {
        let now = self.now_ms();
        if let Some(node) = record.nodes.iter_mut().find(|n| n.node_id == target_node_id) {
            node.status = status;
            node.updated_at_ms = now;
        }
        record.updated_at_ms = now;

        let expected = self.read_current_value().await?;
        self.write_current_deployment(record, expected.as_deref()).await
    }

    /// Partition nodes into followers and leader.
    /// Returns (follower_ids, is_leader_in_list).
    fn partition_nodes(&self, nodes: &[NodeDeployState]) -> (Vec<u64>, bool) {
        let mut followers = Vec::new();
        let mut has_leader = false;

        for node in nodes {
            if node.node_id == self.node_id {
                has_leader = true;
            } else {
                followers.push(node.node_id);
            }
        }

        (followers, has_leader)
    }

    /// Count nodes in Pending status.
    fn count_pending(&self, record: &DeploymentRecord) -> u32 {
        record.count_nodes_in_status(&NodeDeployStatus::Pending)
    }

    /// Finalize: mark Completed, archive to history.
    async fn finalize_deployment(&self, mut record: DeploymentRecord) -> Result<DeploymentRecord> {
        record.status = DeploymentStatus::Completed;
        record.updated_at_ms = self.now_ms();
        let expected = self.read_current_value().await?;
        self.write_current_deployment(&record, expected.as_deref()).await?;

        // Archive to history
        history::archive_deployment(&*self.kv, &record).await?;

        info!(deploy_id = %record.deploy_id, "deployment completed successfully");
        Ok(record)
    }

    /// Get current time in milliseconds. Uses system clock.
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }
}
