//! Federation policy defaults for Forge repositories.
//!
//! Forge repos require the strictest verification because code integrity
//! is critical — accepting a bad ref could mean running compromised code.

use aspen_cluster::federation::ForkDetectionMode;
use aspen_cluster::federation::ResourcePolicy;
use aspen_cluster::federation::SelectionStrategy;
use aspen_cluster::federation::VerificationConfig;

/// The resource type identifier for Forge repositories.
pub const FORGE_RESOURCE_TYPE: &str = "forge:repo";

/// Create the default federation policy for Forge repositories.
///
/// Forge repos are verified strictly:
/// - **Quorum of 2** trusted seeders must agree on ref heads
/// - **Delegate signatures required** on ref updates
/// - **Trust proximity** selection (prefer close trust graph neighbors)
/// - **Halt on fork** detection (code integrity is critical)
///
/// Applications can override these defaults when configuring individual repos.
pub fn forge_repo_policy() -> ResourcePolicy {
    ResourcePolicy::new(FORGE_RESOURCE_TYPE)
        .with_verification(VerificationConfig::quorum(2).with_delegate_signatures())
        .with_selection(SelectionStrategy::TrustProximity)
        .with_fork_detection(ForkDetectionMode::Halt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_repo_policy_defaults() {
        let policy = forge_repo_policy();
        assert_eq!(policy.resource_type, "forge:repo");
        assert_eq!(policy.verification.quorum_threshold, 2);
        assert!(policy.verification.require_delegate_signatures);
        assert!(matches!(policy.selection, SelectionStrategy::TrustProximity));
        assert!(matches!(policy.fork_detection, ForkDetectionMode::Halt));
    }

    #[test]
    fn test_forge_resource_type_constant() {
        assert_eq!(FORGE_RESOURCE_TYPE, "forge:repo");
    }
}
