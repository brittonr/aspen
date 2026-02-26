//! Forge (decentralized Git) handler sub-modules.
//!
//! Repos, objects, refs, issues, and patches have been migrated to
//! `aspen-forge-plugin` (WASM). This module retains only the handler
//! functions that require `ForgeNode` context or federation infrastructure:
//!
//! - Federation operations (9 ops)
//! - Git Bridge operations (6 ops)
//!
//! The `ForgeServiceExecutor` in `executor.rs` wraps these as a `ServiceExecutor`.

pub(crate) mod handlers;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use aspen_client_api::ClientRpcRequest;
    use aspen_rpc_core::ServiceExecutor;

    /// Verify the executor handles the correct set of operations.
    #[test]
    fn test_executor_handles_count() {
        let handles = crate::ForgeServiceExecutor::HANDLES;
        // 9 federation + 7 git bridge = 16
        assert_eq!(handles.len(), 16);
    }

    #[test]
    fn test_executor_handles_federation_ops() {
        let handles = crate::ForgeServiceExecutor::HANDLES;
        assert!(handles.contains(&"ForgeGetDelegateKey"));
        assert!(handles.contains(&"GetFederationStatus"));
        assert!(handles.contains(&"ListDiscoveredClusters"));
        assert!(handles.contains(&"GetDiscoveredCluster"));
        assert!(handles.contains(&"TrustCluster"));
        assert!(handles.contains(&"UntrustCluster"));
        assert!(handles.contains(&"FederateRepository"));
        assert!(handles.contains(&"ListFederatedRepositories"));
        assert!(handles.contains(&"ForgeFetchFederated"));
    }

    #[test]
    fn test_executor_handles_git_bridge_ops() {
        let handles = crate::ForgeServiceExecutor::HANDLES;
        assert!(handles.contains(&"GitBridgeListRefs"));
        assert!(handles.contains(&"GitBridgeFetch"));
        assert!(handles.contains(&"GitBridgePush"));
        assert!(handles.contains(&"GitBridgePushStart"));
        assert!(handles.contains(&"GitBridgePushChunk"));
        assert!(handles.contains(&"GitBridgePushComplete"));
        assert!(handles.contains(&"GitBridgeProbeObjects"));
    }

    #[test]
    fn test_executor_does_not_handle_migrated_ops() {
        let handles = crate::ForgeServiceExecutor::HANDLES;
        // These are handled by WASM forge plugin
        assert!(!handles.contains(&"ForgeCreateRepo"));
        assert!(!handles.contains(&"ForgeGetRef"));
        assert!(!handles.contains(&"ForgeCreateIssue"));
        assert!(!handles.contains(&"ForgeCreatePatch"));
    }

    #[test]
    fn test_executor_metadata() {
        use crate::ForgeServiceExecutor;
        assert_eq!(ForgeServiceExecutor::SERVICE_NAME, "forge");
        assert_eq!(ForgeServiceExecutor::PRIORITY, 540);
        assert_eq!(ForgeServiceExecutor::APP_ID, Some("forge"));
    }
}
