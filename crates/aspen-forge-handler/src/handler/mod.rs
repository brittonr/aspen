//! Forge (decentralized Git) request handler.
//!
//! Handles all Forge* operations for decentralized version control.
//! Operations are grouped into sub-handlers:
//! - `RepoSubHandler` for repository management (create, get, list)
//! - `ObjectsSubHandler` for git objects (blob, tree, commit, log)
//! - `RefsSubHandler` for refs (branches, tags)
//! - `IssuesSubHandler` for CRDT-based issue tracking
//! - `PatchesSubHandler` for CRDT-based pull requests
//! - `FederationSubHandler` for cross-cluster sync and delegate keys
//! - `GitBridgeSubHandler` for git-remote-aspen interop

mod federation;
mod git_bridge;
mod handlers;
mod objects;
mod refs;
mod repo;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use federation::FederationSubHandler;
use git_bridge::GitBridgeSubHandler;
use objects::ObjectsSubHandler;
use refs::RefsSubHandler;
use repo::RepoSubHandler;

/// Handler for Forge operations.
pub struct ForgeHandler;

#[async_trait::async_trait]
impl RequestHandler for ForgeHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        let repo = RepoSubHandler;
        let objects = ObjectsSubHandler;
        let refs = RefsSubHandler;
        let federation = FederationSubHandler;
        let git_bridge = GitBridgeSubHandler;

        repo.can_handle(request)
            || objects.can_handle(request)
            || refs.can_handle(request)
            || federation.can_handle(request)
            || git_bridge.can_handle(request)
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if forge node is available
        let forge_node = match &ctx.forge_node {
            Some(node) => node,
            None => {
                return Ok(ClientRpcResponse::error("FORGE_UNAVAILABLE", "Forge feature not configured on this node"));
            }
        };

        let repo = RepoSubHandler;
        let objects = ObjectsSubHandler;
        let refs = RefsSubHandler;
        let federation = FederationSubHandler;
        let git_bridge = GitBridgeSubHandler;

        if repo.can_handle(&request) {
            return repo.handle(request, ctx, forge_node).await;
        }
        if objects.can_handle(&request) {
            return objects.handle(request, ctx, forge_node).await;
        }
        if refs.can_handle(&request) {
            return refs.handle(request, ctx, forge_node).await;
        }
        if federation.can_handle(&request) {
            return federation.handle(request, ctx, forge_node).await;
        }
        if git_bridge.can_handle(&request) {
            return git_bridge.handle(request, ctx, forge_node).await;
        }

        Err(anyhow::anyhow!("request not handled by ForgeHandler"))
    }

    fn name(&self) -> &'static str {
        "ForgeHandler"
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use aspen_client_api::ClientRpcRequest;

    use super::*;

    // =========================================================================
    // Handler Dispatch Tests (can_handle)
    // =========================================================================

    // --- Repository Operations ---

    #[test]
    fn test_can_handle_create_repo() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreateRepo {
            name: "my-project".to_string(),
            description: Some("A test project".to_string()),
            default_branch: Some("main".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_get_repo() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetRepo {
            repo_id: "abcd1234".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_repos() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListRepos {
            limit: Some(10),
            offset: Some(0),
        }));
    }

    // --- Git Object Operations ---

    #[test]
    fn test_can_handle_store_blob() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeStoreBlob {
            repo_id: "abcd1234".to_string(),
            content: b"file content".to_vec(),
        }));
    }

    #[test]
    fn test_can_handle_get_blob() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetBlob {
            hash: "abcd1234".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_create_tree() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreateTree {
            repo_id: "abcd1234".to_string(),
            entries_json: r#"[]"#.to_string(),
        }));
    }

    #[test]
    fn test_can_handle_get_tree() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetTree {
            hash: "abcd1234".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_commit() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCommit {
            repo_id: "abcd1234".to_string(),
            tree: "tree_hash".to_string(),
            parents: vec![],
            message: "Initial commit".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_get_commit() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetCommit {
            hash: "abcd1234".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_log() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeLog {
            repo_id: "abcd1234".to_string(),
            ref_name: Some("heads/main".to_string()),
            limit: Some(10),
        }));
    }

    // --- Ref Operations ---

    #[test]
    fn test_can_handle_get_ref() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetRef {
            repo_id: "abcd1234".to_string(),
            ref_name: "heads/main".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_set_ref() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeSetRef {
            repo_id: "abcd1234".to_string(),
            ref_name: "heads/main".to_string(),
            hash: "commit_hash".to_string(),
            signer: None,
            signature: None,
            timestamp_ms: None,
        }));
    }

    #[test]
    fn test_can_handle_delete_ref() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeDeleteRef {
            repo_id: "abcd1234".to_string(),
            ref_name: "heads/feature".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_cas_ref() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCasRef {
            repo_id: "abcd1234".to_string(),
            ref_name: "heads/main".to_string(),
            expected: Some("old_hash".to_string()),
            new_hash: "new_hash".to_string(),
            signer: None,
            signature: None,
            timestamp_ms: None,
        }));
    }

    #[test]
    fn test_can_handle_list_branches() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListBranches {
            repo_id: "abcd1234".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_tags() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListTags {
            repo_id: "abcd1234".to_string(),
        }));
    }

    // --- Issue/Patch Operations (migrated to aspen-forge-plugin WASM) ---

    #[test]
    fn test_issues_migrated_to_plugin() {
        let handler = ForgeHandler;
        // Issues are now handled by the WASM forge plugin, not the native handler
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeCreateIssue {
            repo_id: "x".into(),
            title: "x".into(),
            body: "x".into(),
            labels: vec![],
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeListIssues {
            repo_id: "x".into(),
            state: None,
            limit: None,
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeGetIssue {
            repo_id: "x".into(),
            issue_id: "x".into(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeCommentIssue {
            repo_id: "x".into(),
            issue_id: "x".into(),
            body: "x".into(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeCloseIssue {
            repo_id: "x".into(),
            issue_id: "x".into(),
            reason: None,
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeReopenIssue {
            repo_id: "x".into(),
            issue_id: "x".into(),
        }));
    }

    #[test]
    fn test_patches_migrated_to_plugin() {
        let handler = ForgeHandler;
        // Patches are now handled by the WASM forge plugin, not the native handler
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeCreatePatch {
            repo_id: "x".into(),
            title: "x".into(),
            description: "x".into(),
            base: "x".into(),
            head: "x".into(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeListPatches {
            repo_id: "x".into(),
            state: None,
            limit: None,
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeGetPatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeUpdatePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            head: "x".into(),
            message: None,
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeApprovePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            commit: "x".into(),
            message: None,
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeMergePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            merge_commit: "x".into(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::ForgeClosePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            reason: None,
        }));
    }

    // --- Delegate Key ---

    #[test]
    fn test_can_handle_get_delegate_key() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetDelegateKey {
            repo_id: "abcd1234".to_string(),
        }));
    }

    // --- Federation Operations ---

    #[test]
    fn test_can_handle_get_federation_status() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetFederationStatus));
    }

    #[test]
    fn test_can_handle_list_discovered_clusters() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ListDiscoveredClusters));
    }

    #[test]
    fn test_can_handle_get_discovered_cluster() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetDiscoveredCluster {
            cluster_key: "cluster_key".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_trust_cluster() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TrustCluster {
            cluster_key: "cluster_key".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_untrust_cluster() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::UntrustCluster {
            cluster_key: "cluster_key".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_federate_repository() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::FederateRepository {
            repo_id: "abcd1234".to_string(),
            mode: "push".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_federated_repositories() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ListFederatedRepositories));
    }

    #[test]
    fn test_can_handle_forge_fetch_federated() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeFetchFederated {
            federated_id: "fed123".to_string(),
            remote_cluster: "remote".to_string(),
        }));
    }

    // --- Git Bridge Operations ---

    #[test]
    fn test_can_handle_git_bridge_list_refs() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgeListRefs {
            repo_id: "abcd1234".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_git_bridge_fetch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgeFetch {
            repo_id: "abcd1234".to_string(),
            want: vec!["sha1_want".to_string()],
            have: vec!["sha1_have".to_string()],
        }));
    }

    #[test]
    fn test_can_handle_git_bridge_push() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePush {
            repo_id: "abcd1234".to_string(),
            objects: vec![],
            refs: vec![],
        }));
    }

    #[test]
    fn test_can_handle_git_bridge_push_start() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePushStart {
            repo_id: "abcd1234".to_string(),
            total_objects: 100,
            total_size_bytes: 1024,
            refs: vec![],
            metadata: None,
        }));
    }

    #[test]
    fn test_can_handle_git_bridge_push_chunk() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePushChunk {
            session_id: "session123".to_string(),
            chunk_id: 1,
            total_chunks: 5,
            objects: vec![],
            chunk_hash: [0; 32],
        }));
    }

    #[test]
    fn test_can_handle_git_bridge_push_complete() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePushComplete {
            session_id: "session123".to_string(),
            content_hash: [0; 32],
        }));
    }

    // --- Rejection Tests ---

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = ForgeHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));

        // Secrets requests (forge handler should NOT handle these)
        {
            use std::collections::HashMap;
            assert!(!handler.can_handle(&ClientRpcRequest::SecretsKvRead {
                mount: "secret".to_string(),
                path: "test/path".to_string(),
                version: None,
            }));
            assert!(!handler.can_handle(&ClientRpcRequest::SecretsKvWrite {
                mount: "secret".to_string(),
                path: "test/path".to_string(),
                data: HashMap::new(),
                cas: None,
            }));
        }
    }

    #[test]
    fn test_handler_name() {
        let handler = ForgeHandler;
        assert_eq!(handler.name(), "ForgeHandler");
    }

    // =========================================================================
    // Request Variant Coverage Tests
    // =========================================================================

    /// Test native handler covers repos, objects, refs, delegate key, federation, git bridge.
    /// Issues (6) and Patches (7) migrated to aspen-forge-plugin (WASM).
    /// Native handler: 3 + 7 + 6 + 1 + 8 + 6 = 31 request types.
    #[test]
    fn test_native_handler_variants_covered() {
        let handler = ForgeHandler;

        // Repository Operations (3)
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreateRepo {
            name: "x".into(),
            description: None,
            default_branch: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetRepo { repo_id: "x".into() }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListRepos {
            limit: None,
            offset: None
        }));

        // Git Objects (7)
        assert!(handler.can_handle(&ClientRpcRequest::ForgeStoreBlob {
            repo_id: "x".into(),
            content: vec![]
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetBlob { hash: "x".into() }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreateTree {
            repo_id: "x".into(),
            entries_json: "[]".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetTree { hash: "x".into() }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCommit {
            repo_id: "x".into(),
            tree: "x".into(),
            parents: vec![],
            message: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetCommit { hash: "x".into() }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeLog {
            repo_id: "x".into(),
            ref_name: None,
            limit: None
        }));

        // Refs (6)
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetRef {
            repo_id: "x".into(),
            ref_name: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeSetRef {
            repo_id: "x".into(),
            ref_name: "x".into(),
            hash: "x".into(),
            signer: None,
            signature: None,
            timestamp_ms: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeDeleteRef {
            repo_id: "x".into(),
            ref_name: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCasRef {
            repo_id: "x".into(),
            ref_name: "x".into(),
            expected: None,
            new_hash: "x".into(),
            signer: None,
            signature: None,
            timestamp_ms: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListBranches { repo_id: "x".into() }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListTags { repo_id: "x".into() }));

        // Delegate Key (1)
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetDelegateKey { repo_id: "x".into() }));

        // Federation (8)
        assert!(handler.can_handle(&ClientRpcRequest::GetFederationStatus));
        assert!(handler.can_handle(&ClientRpcRequest::ListDiscoveredClusters));
        assert!(handler.can_handle(&ClientRpcRequest::GetDiscoveredCluster {
            cluster_key: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::TrustCluster {
            cluster_key: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::UntrustCluster {
            cluster_key: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::FederateRepository {
            repo_id: "x".into(),
            mode: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ListFederatedRepositories));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeFetchFederated {
            federated_id: "x".into(),
            remote_cluster: "x".into()
        }));

        // Git Bridge (6)
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgeListRefs { repo_id: "x".into() }));
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgeFetch {
            repo_id: "x".into(),
            want: vec![],
            have: vec![]
        }));
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePush {
            repo_id: "x".into(),
            objects: vec![],
            refs: vec![]
        }));
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePushStart {
            repo_id: "x".into(),
            total_objects: 1,
            total_size_bytes: 1024,
            refs: vec![],
            metadata: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePushChunk {
            session_id: "x".into(),
            chunk_id: 1,
            total_chunks: 1,
            objects: vec![],
            chunk_hash: [0; 32]
        }));
        assert!(handler.can_handle(&ClientRpcRequest::GitBridgePushComplete {
            session_id: "x".into(),
            content_hash: [0; 32]
        }));
    }
}
