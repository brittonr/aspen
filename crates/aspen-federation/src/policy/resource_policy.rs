//! Per-resource federation policy.
//!
//! `ResourcePolicy` is the composite policy that any application configures
//! when it federates a resource. It combines access control, verification
//! requirements, selection strategy, and fork detection mode.

use serde::Deserialize;
use serde::Serialize;

use super::fork_detection::ForkDetectionMode;
use super::selection::SelectionStrategy;
use super::verification::VerificationConfig;
use crate::types::FederationSettings;

/// Per-resource federation policy.
///
/// Applications configure this when federating a resource. The federation
/// layer enforces it uniformly regardless of resource type.
///
/// # Examples
///
/// ```ignore
/// // Forge repo: strict quorum, halt on forks (code integrity matters)
/// let forge_policy = ResourcePolicy::new("forge:repo")
///     .with_access(FederationSettings::public())
///     .with_verification(VerificationConfig::quorum(2))
///     .with_selection(SelectionStrategy::TrustProximity)
///     .with_fork_detection(ForkDetectionMode::Halt);
///
/// // CI pipeline: origin authority, warn on forks
/// let ci_policy = ResourcePolicy::new("ci:pipeline")
///     .with_access(FederationSettings::allowlist(vec![...]))
///     .with_verification(VerificationConfig::origin_authority())
///     .with_selection(SelectionStrategy::LowestLatency)
///     .with_fork_detection(ForkDetectionMode::Warn);
///
/// // CRDT doc: no quorum needed (CRDTs self-resolve)
/// let docs_policy = ResourcePolicy::new("docs:crdt")
///     .with_verification(VerificationConfig::none())
///     .with_fork_detection(ForkDetectionMode::Disabled);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePolicy {
    /// Resource type identifier (e.g., "forge:repo", "ci:pipeline", "blob:collection").
    pub resource_type: String,

    /// Access control (mode + allowlist).
    #[serde(default)]
    pub access: FederationSettings,

    /// Verification requirements (quorum, signatures).
    #[serde(default)]
    pub verification: VerificationConfig,

    /// Cluster selection strategy.
    #[serde(default)]
    pub selection: SelectionStrategy,

    /// Fork detection mode.
    #[serde(default)]
    pub fork_detection: ForkDetectionMode,

    /// Opaque application-specific metadata (postcard-encoded bytes).
    ///
    /// Applications store their own data here. For example, Forge stores
    /// `delegates: Vec<[u8; 32]>` for ref signing. The federation layer
    /// does not interpret this field.
    #[serde(default)]
    pub app_metadata: Vec<u8>,
}

impl ResourcePolicy {
    /// Create a new resource policy with defaults.
    ///
    /// Defaults:
    /// - Access: disabled
    /// - Verification: none (no quorum)
    /// - Selection: scored (composite ranking)
    /// - Fork detection: warn
    pub fn new(resource_type: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            access: FederationSettings::default(),
            verification: VerificationConfig::default(),
            selection: SelectionStrategy::default(),
            fork_detection: ForkDetectionMode::default(),
            app_metadata: Vec::new(),
        }
    }

    /// Set access control.
    pub fn with_access(mut self, access: FederationSettings) -> Self {
        self.access = access;
        self
    }

    /// Set verification requirements.
    pub fn with_verification(mut self, verification: VerificationConfig) -> Self {
        self.verification = verification;
        self
    }

    /// Set selection strategy.
    pub fn with_selection(mut self, selection: SelectionStrategy) -> Self {
        self.selection = selection;
        self
    }

    /// Set fork detection mode.
    pub fn with_fork_detection(mut self, fork_detection: ForkDetectionMode) -> Self {
        self.fork_detection = fork_detection;
        self
    }

    /// Set opaque app metadata.
    pub fn with_app_metadata(mut self, metadata: Vec<u8>) -> Self {
        self.app_metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_policy_defaults() {
        let policy = ResourcePolicy::new("forge:repo");
        assert_eq!(policy.resource_type, "forge:repo");
        assert_eq!(policy.verification.quorum_threshold, 0);
        assert!(matches!(policy.selection, SelectionStrategy::Scored));
        assert!(matches!(policy.fork_detection, ForkDetectionMode::Warn));
    }

    #[test]
    fn test_resource_policy_builder() {
        let policy = ResourcePolicy::new("ci:pipeline")
            .with_access(FederationSettings::public())
            .with_verification(VerificationConfig::quorum(3))
            .with_selection(SelectionStrategy::LowestLatency)
            .with_fork_detection(ForkDetectionMode::Halt);

        assert_eq!(policy.resource_type, "ci:pipeline");
        assert_eq!(policy.verification.quorum_threshold, 3);
        assert!(matches!(policy.selection, SelectionStrategy::LowestLatency));
        assert!(matches!(policy.fork_detection, ForkDetectionMode::Halt));
    }

    #[test]
    fn test_resource_policy_roundtrip() {
        let policy = ResourcePolicy::new("blob:collection")
            .with_verification(VerificationConfig::quorum(2))
            .with_app_metadata(vec![1, 2, 3, 4]);

        let bytes = postcard::to_allocvec(&policy).unwrap();
        let parsed: ResourcePolicy = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.resource_type, "blob:collection");
        assert_eq!(parsed.verification.quorum_threshold, 2);
        assert_eq!(parsed.app_metadata, vec![1, 2, 3, 4]);
    }
}
