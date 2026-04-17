//! ClusterController trait implementation for RaftNode.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use aspen_cluster_types::AddLearnerRequest;
use aspen_cluster_types::ChangeMembershipRequest;
use aspen_cluster_types::ClusterMetrics;
use aspen_cluster_types::ClusterState;
use aspen_cluster_types::ControlPlaneError;
use aspen_cluster_types::InitRequest;
use aspen_cluster_types::SnapshotLogId;
use aspen_raft_types::MEMBERSHIP_OPERATION_TIMEOUT;
use aspen_traits::ClusterController;
use async_trait::async_trait;
use tracing::info;
use tracing::instrument;

use super::RaftNode;
use super::conversions::cluster_metrics_from_openraft;
use super::conversions::snapshot_log_id_from_openraft;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;

#[inline]
fn duration_ms_u64(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[async_trait]
impl ClusterController for RaftNode {
    #[instrument(skip(self))]
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        // Acquire permit to limit concurrency
        let _permit = self.semaphore().acquire().await.map_err(|_| ControlPlaneError::Failed {
            reason: "semaphore closed".into(),
        })?;

        // Validate request (checks empty, node ID 0, duplicates)
        request.validate()?;

        // Build RaftMemberInfo map
        let nodes: BTreeMap<NodeId, RaftMemberInfo> = request
            .initial_members
            .iter()
            .map(|cluster_node| {
                let iroh_addr = cluster_node.iroh_addr().ok_or_else(|| ControlPlaneError::InvalidRequest {
                    reason: format!("node_addr must be set for node {}", cluster_node.id),
                })?;
                let mut member_info = RaftMemberInfo::new(iroh_addr.clone());
                member_info.relay_url = cluster_node.relay_url.clone();
                Ok::<_, ControlPlaneError>((cluster_node.id.into(), member_info))
            })
            .collect::<Result<_, _>>()?;

        // Tiger Style: nodes map must match input
        debug_assert!(
            nodes.len() == request.initial_members.len(),
            "CLUSTER: nodes map size ({}) must match initial_members ({})",
            nodes.len(),
            request.initial_members.len()
        );

        info!("calling raft.initialize() with {} nodes", nodes.len());
        // Tiger Style: Explicit timeout prevents indefinite hang if quorum unavailable
        tokio::time::timeout(MEMBERSHIP_OPERATION_TIMEOUT, self.raft().initialize(nodes))
            .await
            .map_err(|_| ControlPlaneError::Timeout {
                duration_ms: duration_ms_u64(MEMBERSHIP_OPERATION_TIMEOUT),
            })?
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;
        info!("raft.initialize() completed successfully");

        self.initialized_ref().store(true, Ordering::Release);
        info!("initialized flag set to true");

        // Initialize trust (Shamir secret sharing) if enabled
        #[cfg(feature = "trust")]
        if request.trust.enabled {
            self.initialize_trust(&request).await.map_err(|e| ControlPlaneError::Failed {
                reason: format!("trust initialization failed: {e}"),
            })?;
        }

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
        // Tiger Style: learner id must be positive
        debug_assert!(request.learner.id > 0, "CLUSTER: learner id must be positive");

        let _permit = self.semaphore().acquire().await.map_err(|_| ControlPlaneError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized()?;

        let learner = request.learner;
        let iroh_addr = learner.iroh_addr().ok_or_else(|| ControlPlaneError::InvalidRequest {
            reason: format!("node_addr must be set for node {}", learner.id),
        })?;

        let mut node = RaftMemberInfo::new(iroh_addr.clone());
        node.relay_url = learner.relay_url.clone();

        info!(
            learner_id = learner.id,
            endpoint_id = %iroh_addr.id,
            relay_url = ?learner.relay_url,
            "adding learner with Iroh address"
        );

        // Tiger Style: Explicit timeout prevents indefinite hang if leader unavailable
        tokio::time::timeout(MEMBERSHIP_OPERATION_TIMEOUT, self.raft().add_learner(learner.id.into(), node, true))
            .await
            .map_err(|_| ControlPlaneError::Timeout {
                duration_ms: duration_ms_u64(MEMBERSHIP_OPERATION_TIMEOUT),
            })?
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| ControlPlaneError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized()?;

        if request.members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "members must include at least one voter".into(),
            });
        }

        let members: std::collections::BTreeSet<NodeId> = request.members.iter().map(|&id| id.into()).collect();
        #[cfg(feature = "trust")]
        let old_members: std::collections::BTreeSet<u64> = {
            let metrics = self.raft().metrics().borrow().clone();
            metrics.membership_config.membership().voter_ids().map(|id| id.0).collect()
        };

        let membership_response =
            tokio::time::timeout(MEMBERSHIP_OPERATION_TIMEOUT, self.raft().change_membership(members, false))
                .await
                .map_err(|_| ControlPlaneError::Timeout {
                    duration_ms: duration_ms_u64(MEMBERSHIP_OPERATION_TIMEOUT),
                })?
                .map_err(|err| ControlPlaneError::Failed {
                    reason: err.to_string(),
                })?;

        #[cfg(feature = "trust")]
        self.rotate_trust_after_membership_change(
            old_members,
            request.members.iter().copied().collect(),
            membership_response.log_id.index(),
        )
        .await
        .map_err(|reason| ControlPlaneError::Failed {
            reason: format!("trust reconfiguration failed after membership change: {reason}"),
        })?;

        #[cfg(not(feature = "trust"))]
        let _ = membership_response;

        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        self.ensure_initialized()?;
        Ok(self.build_cluster_state())
    }

    #[instrument(skip(self))]
    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        self.ensure_initialized()?;
        let metrics = self.raft().metrics().borrow().clone();
        Ok(metrics.current_leader.map(|id| id.0))
    }

    #[instrument(skip(self))]
    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
        self.ensure_initialized()?;
        let metrics = self.raft().metrics().borrow().clone();
        Ok(cluster_metrics_from_openraft(&metrics))
    }

    #[instrument(skip(self))]
    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError> {
        self.ensure_initialized()?;

        // Trigger a snapshot (returns () on success)
        self.raft().trigger().snapshot().await.map_err(|err| ControlPlaneError::Failed {
            reason: err.to_string(),
        })?;

        // Get the current snapshot from metrics and convert to wrapper type
        let metrics = self.raft().metrics().borrow().clone();
        Ok(metrics.snapshot.as_ref().map(snapshot_log_id_from_openraft))
    }

    #[instrument(skip(self))]
    async fn transfer_leader(&self, target: u64) -> Result<(), ControlPlaneError> {
        self.ensure_initialized()?;

        info!(target, "triggering leadership transfer");
        self.raft()
            .trigger()
            .transfer_leader(target.into())
            .await
            .map_err(|err| ControlPlaneError::Failed {
                reason: err.to_string(),
            })?;

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        // Fast path: check atomic flag (Acquire ensures we see prior writes)
        if self.initialized_ref().load(Ordering::Acquire) {
            return true;
        }

        // Slow path: check if membership exists via Raft replication
        // A node may have received membership through replication without explicit init()
        let metrics = self.raft().metrics().borrow().clone();
        metrics.membership_config.membership().nodes().next().is_some()
    }
}
