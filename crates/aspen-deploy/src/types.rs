//! Deployment type definitions.
//!
//! These types represent the deployment state machine, stored in KV at
//! `_sys:deploy:current` and `_sys:deploy:history:{timestamp}`.
//!
//! Formally verified — see `verus/` for invariant proofs.

use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// KV key prefixes
// ============================================================================

/// KV key for the current active deployment.
pub const DEPLOY_CURRENT_KEY: &str = "_sys:deploy:current";

/// KV key prefix for deployment history entries.
/// Full key: `_sys:deploy:history:{timestamp_ms:020}`
pub const DEPLOY_HISTORY_PREFIX: &str = "_sys:deploy:history:";

/// KV key prefix for per-node deployment status.
/// Full key: `_sys:deploy:node:{node_id}`
pub const DEPLOY_NODE_PREFIX: &str = "_sys:deploy:node:";

// ============================================================================
// DeploymentStatus
// ============================================================================

/// Overall deployment lifecycle state.
///
/// State transitions:
/// ```text
/// Pending → Deploying → Completed
///                     → Failed → RollingBack → RolledBack
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    /// Deployment created, not yet started.
    Pending,
    /// Nodes are being upgraded.
    Deploying,
    /// All nodes upgraded and healthy.
    Completed,
    /// One or more nodes failed health checks after upgrade.
    Failed,
    /// Rollback in progress.
    RollingBack,
    /// Rollback completed.
    RolledBack,
}

impl DeploymentStatus {
    /// Returns true if this status represents a terminal state.
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::RolledBack)
    }

    /// Returns true if a new deployment can be started (no active deployment).
    #[inline]
    pub fn allows_new_deployment(&self) -> bool {
        self.is_terminal()
    }
}

// ============================================================================
// NodeDeployStatus
// ============================================================================

/// Per-node upgrade state within a deployment.
///
/// State transitions:
/// ```text
/// Pending → Draining → Upgrading → Restarting → Healthy
///                                             → Failed(reason)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeDeployStatus {
    /// Node has not started upgrading yet.
    Pending,
    /// Node is draining in-flight operations.
    Draining,
    /// Node is replacing its binary.
    Upgrading,
    /// Node has restarted and is rejoining the cluster.
    Restarting,
    /// Node passed all health checks after upgrade.
    Healthy,
    /// Node failed during upgrade.
    Failed(String),
}

impl NodeDeployStatus {
    /// Returns true if this node has completed its upgrade (success or failure).
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Healthy | Self::Failed(_))
    }

    /// Returns a string representation for status reporting.
    pub fn as_status_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Draining => "draining",
            Self::Upgrading => "upgrading",
            Self::Restarting => "restarting",
            Self::Healthy => "healthy",
            Self::Failed(_) => "failed",
        }
    }
}

// ============================================================================
// DeployArtifact
// ============================================================================

/// What binary to deploy.
///
/// On Nix systems, a store path provides atomic profile switching with
/// rollback via generations. On non-Nix systems, a blob hash triggers
/// download + SHA-256 validation + atomic rename.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployArtifact {
    /// Nix store path (e.g., "/nix/store/abc...-aspen-node-0.1.0").
    NixStorePath(String),
    /// BLAKE3 blob hash (hex-encoded) from the cluster's blob store.
    BlobHash(String),
}

impl DeployArtifact {
    /// Returns true if this is a Nix store path.
    #[inline]
    pub fn is_nix(&self) -> bool {
        matches!(self, Self::NixStorePath(_))
    }

    /// Returns a display-friendly string for status output.
    pub fn display_ref(&self) -> &str {
        match self {
            Self::NixStorePath(path) => path,
            Self::BlobHash(hash) => hash,
        }
    }

    /// Parse an artifact string: store paths start with "/nix/store/",
    /// everything else is treated as a blob hash.
    pub fn parse(s: &str) -> Self {
        if s.starts_with("/nix/store/") {
            Self::NixStorePath(s.to_string())
        } else {
            Self::BlobHash(s.to_string())
        }
    }
}

// ============================================================================
// DeployStrategy
// ============================================================================

/// How to roll out the deployment across nodes.
///
/// Currently only rolling is supported. Future strategies could include
/// blue-green or canary with traffic splitting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployStrategy {
    /// Upgrade nodes one at a time (or up to `max_concurrent`).
    Rolling {
        /// Maximum number of nodes upgrading simultaneously.
        /// Clamped to `(voter_count - 1) / 2` for quorum safety.
        max_concurrent: u32,
    },
}

impl DeployStrategy {
    /// Create a rolling strategy with the given concurrency.
    pub fn rolling(max_concurrent: u32) -> Self {
        Self::Rolling {
            max_concurrent: max_concurrent.max(1),
        }
    }

    /// Returns the requested max concurrent value.
    pub fn max_concurrent(&self) -> u32 {
        match self {
            Self::Rolling { max_concurrent } => *max_concurrent,
        }
    }
}

impl Default for DeployStrategy {
    fn default() -> Self {
        Self::rolling(1)
    }
}

// ============================================================================
// NodeDeployState
// ============================================================================

/// Per-node state within a deployment record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDeployState {
    /// Node ID.
    pub node_id: u64,
    /// Current status of this node's upgrade.
    pub status: NodeDeployStatus,
    /// Timestamp of last status change (milliseconds since epoch).
    pub updated_at_ms: u64,
}

// ============================================================================
// DeploymentRecord
// ============================================================================

/// Full deployment record stored in KV.
///
/// Stored at `_sys:deploy:current` during active deployments and
/// archived to `_sys:deploy:history:{timestamp}` on completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentRecord {
    /// Unique deployment ID (UUID or timestamp-based).
    pub deploy_id: String,
    /// What binary to deploy.
    pub artifact: DeployArtifact,
    /// Deployment strategy.
    pub strategy: DeployStrategy,
    /// Overall deployment status.
    pub status: DeploymentStatus,
    /// Per-node deployment state.
    pub nodes: Vec<NodeDeployState>,
    /// Deployment creation timestamp (milliseconds since epoch).
    pub created_at_ms: u64,
    /// Last status update timestamp (milliseconds since epoch).
    pub updated_at_ms: u64,
    /// Error message if the deployment failed.
    pub error: Option<String>,
    /// Binary to validate inside a Nix store path (`None` → default `bin/aspen-node`).
    /// Stored in the record so resume-after-failover preserves the setting.
    #[serde(default)]
    pub expected_binary: Option<String>,
}

impl DeploymentRecord {
    /// Create a new pending deployment record.
    pub fn new(
        deploy_id: String,
        artifact: DeployArtifact,
        strategy: DeployStrategy,
        node_ids: &[u64],
        now_ms: u64,
    ) -> Self {
        Self::with_expected_binary(deploy_id, artifact, strategy, node_ids, now_ms, None)
    }

    /// Create a new pending deployment record with a custom expected binary.
    pub fn with_expected_binary(
        deploy_id: String,
        artifact: DeployArtifact,
        strategy: DeployStrategy,
        node_ids: &[u64],
        now_ms: u64,
        expected_binary: Option<String>,
    ) -> Self {
        let nodes = node_ids
            .iter()
            .map(|&node_id| NodeDeployState {
                node_id,
                status: NodeDeployStatus::Pending,
                updated_at_ms: now_ms,
            })
            .collect();

        Self {
            deploy_id,
            artifact,
            strategy,
            status: DeploymentStatus::Pending,
            nodes,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            error: None,
            expected_binary,
        }
    }

    /// Count nodes in a given status.
    pub fn count_nodes_in_status(&self, status: &NodeDeployStatus) -> u32 {
        self.nodes.iter().filter(|n| &n.status == status).count() as u32
    }

    /// Count nodes that have completed their upgrade (healthy).
    pub fn count_healthy(&self) -> u32 {
        self.count_nodes_in_status(&NodeDeployStatus::Healthy)
    }

    /// Count nodes currently in a non-terminal upgrade state.
    pub fn count_upgrading(&self) -> u32 {
        self.nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.status,
                    NodeDeployStatus::Draining | NodeDeployStatus::Upgrading | NodeDeployStatus::Restarting
                )
            })
            .count() as u32
    }

    /// Count nodes that failed.
    pub fn count_failed(&self) -> u32 {
        self.nodes.iter().filter(|n| matches!(n.status, NodeDeployStatus::Failed(_))).count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_status_terminal() {
        assert!(!DeploymentStatus::Pending.is_terminal());
        assert!(!DeploymentStatus::Deploying.is_terminal());
        assert!(DeploymentStatus::Completed.is_terminal());
        assert!(DeploymentStatus::Failed.is_terminal());
        assert!(!DeploymentStatus::RollingBack.is_terminal());
        assert!(DeploymentStatus::RolledBack.is_terminal());
    }

    #[test]
    fn test_deployment_status_allows_new() {
        assert!(!DeploymentStatus::Pending.allows_new_deployment());
        assert!(!DeploymentStatus::Deploying.allows_new_deployment());
        assert!(DeploymentStatus::Completed.allows_new_deployment());
        assert!(DeploymentStatus::Failed.allows_new_deployment());
        assert!(!DeploymentStatus::RollingBack.allows_new_deployment());
        assert!(DeploymentStatus::RolledBack.allows_new_deployment());
    }

    #[test]
    fn test_node_deploy_status_terminal() {
        assert!(!NodeDeployStatus::Pending.is_terminal());
        assert!(!NodeDeployStatus::Draining.is_terminal());
        assert!(!NodeDeployStatus::Upgrading.is_terminal());
        assert!(!NodeDeployStatus::Restarting.is_terminal());
        assert!(NodeDeployStatus::Healthy.is_terminal());
        assert!(NodeDeployStatus::Failed("reason".into()).is_terminal());
    }

    #[test]
    fn test_node_deploy_status_str() {
        assert_eq!(NodeDeployStatus::Pending.as_status_str(), "pending");
        assert_eq!(NodeDeployStatus::Draining.as_status_str(), "draining");
        assert_eq!(NodeDeployStatus::Upgrading.as_status_str(), "upgrading");
        assert_eq!(NodeDeployStatus::Restarting.as_status_str(), "restarting");
        assert_eq!(NodeDeployStatus::Healthy.as_status_str(), "healthy");
        assert_eq!(NodeDeployStatus::Failed("x".into()).as_status_str(), "failed");
    }

    #[test]
    fn test_deploy_artifact_parse_nix() {
        let art = DeployArtifact::parse("/nix/store/abc123-aspen-node-0.1.0");
        assert!(art.is_nix());
        assert_eq!(art.display_ref(), "/nix/store/abc123-aspen-node-0.1.0");
    }

    #[test]
    fn test_deploy_artifact_parse_blob() {
        let hash = "a".repeat(64);
        let art = DeployArtifact::parse(&hash);
        assert!(!art.is_nix());
        assert_eq!(art.display_ref(), hash);
    }

    #[test]
    fn test_deploy_strategy_default() {
        let strat = DeployStrategy::default();
        assert_eq!(strat.max_concurrent(), 1);
    }

    #[test]
    fn test_deploy_strategy_rolling_clamped() {
        let strat = DeployStrategy::rolling(0);
        assert_eq!(strat.max_concurrent(), 1); // min 1
    }

    #[test]
    fn test_deployment_record_new() {
        let record = DeploymentRecord::new(
            "deploy-1".into(),
            DeployArtifact::NixStorePath("/nix/store/abc-aspen".into()),
            DeployStrategy::rolling(1),
            &[1, 2, 3],
            1000,
        );
        assert_eq!(record.deploy_id, "deploy-1");
        assert_eq!(record.status, DeploymentStatus::Pending);
        assert_eq!(record.nodes.len(), 3);
        assert_eq!(record.created_at_ms, 1000);
        assert_eq!(record.updated_at_ms, 1000);
        assert!(record.error.is_none());

        for node in &record.nodes {
            assert_eq!(node.status, NodeDeployStatus::Pending);
        }
    }

    #[test]
    fn test_deployment_record_counts() {
        let mut record = DeploymentRecord::new(
            "deploy-2".into(),
            DeployArtifact::BlobHash("a".repeat(64)),
            DeployStrategy::rolling(1),
            &[1, 2, 3],
            1000,
        );

        assert_eq!(record.count_healthy(), 0);
        assert_eq!(record.count_upgrading(), 0);
        assert_eq!(record.count_failed(), 0);

        record.nodes[0].status = NodeDeployStatus::Healthy;
        record.nodes[1].status = NodeDeployStatus::Upgrading;
        record.nodes[2].status = NodeDeployStatus::Failed("timeout".into());

        assert_eq!(record.count_healthy(), 1);
        assert_eq!(record.count_upgrading(), 1);
        assert_eq!(record.count_failed(), 1);
    }

    #[test]
    fn test_deployment_record_serde_roundtrip() {
        let record = DeploymentRecord::new(
            "deploy-3".into(),
            DeployArtifact::NixStorePath("/nix/store/xyz-aspen".into()),
            DeployStrategy::rolling(2),
            &[1, 2, 3, 4, 5],
            12345,
        );

        let json = serde_json::to_string(&record).expect("serialize");
        let decoded: DeploymentRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_deploy_artifact_serde_roundtrip() {
        let nix = DeployArtifact::NixStorePath("/nix/store/abc-aspen".into());
        let json = serde_json::to_string(&nix).expect("serialize");
        let decoded: DeployArtifact = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(nix, decoded);

        let blob = DeployArtifact::BlobHash("deadbeef".repeat(8));
        let json = serde_json::to_string(&blob).expect("serialize");
        let decoded: DeployArtifact = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(blob, decoded);
    }

    #[test]
    fn test_deploy_status_serde_roundtrip() {
        for status in [
            DeploymentStatus::Pending,
            DeploymentStatus::Deploying,
            DeploymentStatus::Completed,
            DeploymentStatus::Failed,
            DeploymentStatus::RollingBack,
            DeploymentStatus::RolledBack,
        ] {
            let json = serde_json::to_string(&status).expect("serialize");
            let decoded: DeploymentStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, decoded);
        }
    }

    #[test]
    fn test_kv_key_constants() {
        assert_eq!(DEPLOY_CURRENT_KEY, "_sys:deploy:current");
        assert!(DEPLOY_HISTORY_PREFIX.starts_with("_sys:deploy:history:"));
        assert!(DEPLOY_NODE_PREFIX.starts_with("_sys:deploy:node:"));
    }
}
