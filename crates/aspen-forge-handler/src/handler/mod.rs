//! Forge (decentralized Git) request handler.
//!
//! Handles all Forge* operations for decentralized version control.
//! Operations are grouped into:
//! - Repository management (create, get, list)
//! - Git objects (blob, tree, commit)
//! - Refs (branches, tags)
//! - Issues (CRDT-based issue tracking)
//! - Patches (CRDT-based pull requests)
//! - Federation (cross-cluster sync)
//! - Git Bridge (git-remote-aspen interop)

mod handlers;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use handlers::cob::issues::*;
use handlers::cob::patches::*;
use handlers::federation::*;
use handlers::log::*;
use handlers::objects::*;
use handlers::refs::*;
use handlers::repo::*;

/// Handler for Forge operations.
pub struct ForgeHandler;

#[async_trait::async_trait]
impl RequestHandler for ForgeHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeCreateRepo { .. }
                | ClientRpcRequest::ForgeGetRepo { .. }
                | ClientRpcRequest::ForgeListRepos { .. }
                | ClientRpcRequest::ForgeStoreBlob { .. }
                | ClientRpcRequest::ForgeGetBlob { .. }
                | ClientRpcRequest::ForgeCreateTree { .. }
                | ClientRpcRequest::ForgeGetTree { .. }
                | ClientRpcRequest::ForgeCommit { .. }
                | ClientRpcRequest::ForgeGetCommit { .. }
                | ClientRpcRequest::ForgeLog { .. }
                | ClientRpcRequest::ForgeGetRef { .. }
                | ClientRpcRequest::ForgeSetRef { .. }
                | ClientRpcRequest::ForgeDeleteRef { .. }
                | ClientRpcRequest::ForgeCasRef { .. }
                | ClientRpcRequest::ForgeListBranches { .. }
                | ClientRpcRequest::ForgeListTags { .. }
                | ClientRpcRequest::ForgeCreateIssue { .. }
                | ClientRpcRequest::ForgeListIssues { .. }
                | ClientRpcRequest::ForgeGetIssue { .. }
                | ClientRpcRequest::ForgeCommentIssue { .. }
                | ClientRpcRequest::ForgeCloseIssue { .. }
                | ClientRpcRequest::ForgeReopenIssue { .. }
                | ClientRpcRequest::ForgeCreatePatch { .. }
                | ClientRpcRequest::ForgeListPatches { .. }
                | ClientRpcRequest::ForgeGetPatch { .. }
                | ClientRpcRequest::ForgeUpdatePatch { .. }
                | ClientRpcRequest::ForgeApprovePatch { .. }
                | ClientRpcRequest::ForgeMergePatch { .. }
                | ClientRpcRequest::ForgeClosePatch { .. }
                | ClientRpcRequest::ForgeGetDelegateKey { .. }
                | ClientRpcRequest::GetFederationStatus
                | ClientRpcRequest::ListDiscoveredClusters
                | ClientRpcRequest::GetDiscoveredCluster { .. }
                | ClientRpcRequest::TrustCluster { .. }
                | ClientRpcRequest::UntrustCluster { .. }
                | ClientRpcRequest::FederateRepository { .. }
                | ClientRpcRequest::ListFederatedRepositories
                | ClientRpcRequest::ForgeFetchFederated { .. }
                | ClientRpcRequest::GitBridgeListRefs { .. }
                | ClientRpcRequest::GitBridgeFetch { .. }
                | ClientRpcRequest::GitBridgePush { .. }
                | ClientRpcRequest::GitBridgePushStart { .. }
                | ClientRpcRequest::GitBridgePushChunk { .. }
                | ClientRpcRequest::GitBridgePushComplete { .. }
        )
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

        match request {
            // ================================================================
            // Repository Operations
            // ================================================================
            ClientRpcRequest::ForgeCreateRepo {
                name,
                description,
                default_branch,
            } => handle_create_repo(forge_node, ctx, name, description, default_branch).await,

            ClientRpcRequest::ForgeGetRepo { repo_id } => handle_get_repo(forge_node, repo_id).await,

            ClientRpcRequest::ForgeListRepos { limit, offset } => handle_list_repos(ctx, limit, offset).await,

            // ================================================================
            // Git Object Operations
            // ================================================================
            ClientRpcRequest::ForgeStoreBlob { repo_id, content } => {
                handle_store_blob(forge_node, repo_id, content).await
            }

            ClientRpcRequest::ForgeGetBlob { hash } => handle_get_blob(forge_node, hash).await,

            ClientRpcRequest::ForgeCreateTree { repo_id, entries_json } => {
                handle_create_tree(forge_node, repo_id, entries_json).await
            }

            ClientRpcRequest::ForgeGetTree { hash } => handle_get_tree(forge_node, hash).await,

            ClientRpcRequest::ForgeCommit {
                repo_id,
                tree,
                parents,
                message,
            } => handle_commit(forge_node, repo_id, tree, parents, message).await,

            ClientRpcRequest::ForgeGetCommit { hash } => handle_get_commit(forge_node, hash).await,

            ClientRpcRequest::ForgeLog {
                repo_id,
                ref_name,
                limit,
            } => handle_log(forge_node, repo_id, ref_name, limit).await,

            // ================================================================
            // Ref Operations
            // ================================================================
            ClientRpcRequest::ForgeGetRef { repo_id, ref_name } => handle_get_ref(forge_node, repo_id, ref_name).await,

            ClientRpcRequest::ForgeSetRef {
                repo_id,
                ref_name,
                hash,
                signer: _,
                signature: _,
                timestamp_ms: _,
            } => handle_set_ref(forge_node, repo_id, ref_name, hash).await,

            ClientRpcRequest::ForgeDeleteRef { repo_id, ref_name } => {
                handle_delete_ref(forge_node, repo_id, ref_name).await
            }

            ClientRpcRequest::ForgeCasRef {
                repo_id,
                ref_name,
                expected,
                new_hash,
                signer,
                signature,
                timestamp_ms,
            } => {
                handle_cas_ref(forge_node, repo_id, ref_name, expected, new_hash, signer, signature, timestamp_ms).await
            }

            ClientRpcRequest::ForgeListBranches { repo_id } => handle_list_branches(forge_node, repo_id).await,

            ClientRpcRequest::ForgeListTags { repo_id } => handle_list_tags(forge_node, repo_id).await,

            // ================================================================
            // Issue Operations
            // ================================================================
            ClientRpcRequest::ForgeCreateIssue {
                repo_id,
                title,
                body,
                labels,
            } => handle_create_issue(forge_node, repo_id, title, body, labels).await,

            ClientRpcRequest::ForgeListIssues { repo_id, state, limit } => {
                handle_list_issues(forge_node, repo_id, state, limit).await
            }

            ClientRpcRequest::ForgeGetIssue { repo_id, issue_id } => {
                handle_get_issue(forge_node, repo_id, issue_id).await
            }

            ClientRpcRequest::ForgeCommentIssue {
                repo_id,
                issue_id,
                body,
            } => handle_comment_issue(forge_node, repo_id, issue_id, body).await,

            ClientRpcRequest::ForgeCloseIssue {
                repo_id,
                issue_id,
                reason,
            } => handle_close_issue(forge_node, repo_id, issue_id, reason).await,

            ClientRpcRequest::ForgeReopenIssue { repo_id, issue_id } => {
                handle_reopen_issue(forge_node, repo_id, issue_id).await
            }

            // ================================================================
            // Patch Operations
            // ================================================================
            ClientRpcRequest::ForgeCreatePatch {
                repo_id,
                title,
                description,
                base,
                head,
            } => handle_create_patch(forge_node, repo_id, title, description, base, head).await,

            ClientRpcRequest::ForgeListPatches { repo_id, state, limit } => {
                handle_list_patches(forge_node, repo_id, state, limit).await
            }

            ClientRpcRequest::ForgeGetPatch { repo_id, patch_id } => {
                handle_get_patch(forge_node, repo_id, patch_id).await
            }

            ClientRpcRequest::ForgeUpdatePatch {
                repo_id,
                patch_id,
                head,
                message,
            } => handle_update_patch(forge_node, repo_id, patch_id, head, message).await,

            ClientRpcRequest::ForgeApprovePatch {
                repo_id,
                patch_id,
                commit,
                message,
            } => handle_approve_patch(forge_node, repo_id, patch_id, commit, message).await,

            ClientRpcRequest::ForgeMergePatch {
                repo_id,
                patch_id,
                merge_commit,
            } => handle_merge_patch(forge_node, repo_id, patch_id, merge_commit).await,

            ClientRpcRequest::ForgeClosePatch {
                repo_id,
                patch_id,
                reason,
            } => handle_close_patch(forge_node, repo_id, patch_id, reason).await,

            // ================================================================
            // Delegate Key
            // ================================================================
            ClientRpcRequest::ForgeGetDelegateKey { repo_id } => handle_get_delegate_key(forge_node, repo_id).await,

            // ================================================================
            // Federation Operations
            // ================================================================
            ClientRpcRequest::GetFederationStatus => handle_get_federation_status(ctx, forge_node).await,

            ClientRpcRequest::ListDiscoveredClusters => handle_list_discovered_clusters(ctx).await,

            ClientRpcRequest::GetDiscoveredCluster { cluster_key } => {
                handle_get_discovered_cluster(ctx, cluster_key).await
            }

            ClientRpcRequest::TrustCluster { cluster_key } => handle_trust_cluster(ctx, cluster_key).await,

            ClientRpcRequest::UntrustCluster { cluster_key } => handle_untrust_cluster(ctx, cluster_key).await,

            ClientRpcRequest::FederateRepository { repo_id, mode } => {
                handle_federate_repository(forge_node, repo_id, mode).await
            }

            ClientRpcRequest::ListFederatedRepositories => handle_list_federated_repositories(forge_node).await,

            ClientRpcRequest::ForgeFetchFederated {
                federated_id,
                remote_cluster,
            } => handle_fetch_federated(forge_node, federated_id, remote_cluster).await,

            // ================================================================
            // Git Bridge Operations (requires git-bridge feature)
            // ================================================================
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeListRefs { repo_id } => {
                handlers::git_bridge::handle_git_bridge_list_refs(forge_node, repo_id).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetch { repo_id, want, have } => {
                handlers::git_bridge::handle_git_bridge_fetch(forge_node, repo_id, want, have).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePush { repo_id, objects, refs } => {
                handlers::git_bridge::handle_git_bridge_push(forge_node, repo_id, objects, refs).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushStart {
                repo_id,
                total_objects,
                total_size_bytes,
                refs,
                metadata,
            } => {
                handlers::git_bridge::handle_git_bridge_push_start(
                    forge_node,
                    repo_id,
                    total_objects,
                    total_size_bytes,
                    refs,
                    metadata,
                )
                .await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushChunk {
                session_id,
                chunk_id,
                total_chunks,
                objects,
                chunk_hash,
            } => {
                handlers::git_bridge::handle_git_bridge_push_chunk(
                    forge_node,
                    session_id,
                    chunk_id,
                    total_chunks,
                    objects,
                    chunk_hash,
                )
                .await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushComplete {
                session_id,
                content_hash,
            } => handlers::git_bridge::handle_git_bridge_push_complete(forge_node, session_id, content_hash).await,

            // Return error when git-bridge feature is not enabled
            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgePush { .. }
            | ClientRpcRequest::GitBridgePushStart { .. }
            | ClientRpcRequest::GitBridgePushChunk { .. }
            | ClientRpcRequest::GitBridgePushComplete { .. } => Ok(ClientRpcResponse::error(
                "GIT_BRIDGE_UNAVAILABLE",
                "Git bridge feature not enabled. Rebuild with --features git-bridge",
            )),

            _ => Err(anyhow::anyhow!("request not handled by ForgeHandler")),
        }
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

    // --- Issue Operations ---

    #[test]
    fn test_can_handle_create_issue() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreateIssue {
            repo_id: "abcd1234".to_string(),
            title: "Bug: Something is broken".to_string(),
            body: "Description of the issue".to_string(),
            labels: vec!["bug".to_string(), "priority-high".to_string()],
        }));
    }

    #[test]
    fn test_can_handle_list_issues() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListIssues {
            repo_id: "abcd1234".to_string(),
            state: Some("open".to_string()),
            limit: Some(50),
        }));
    }

    #[test]
    fn test_can_handle_get_issue() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetIssue {
            repo_id: "abcd1234".to_string(),
            issue_id: "issue123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_comment_issue() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCommentIssue {
            repo_id: "abcd1234".to_string(),
            issue_id: "issue123".to_string(),
            body: "This is a comment".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_close_issue() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCloseIssue {
            repo_id: "abcd1234".to_string(),
            issue_id: "issue123".to_string(),
            reason: Some("Resolved".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_reopen_issue() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeReopenIssue {
            repo_id: "abcd1234".to_string(),
            issue_id: "issue123".to_string(),
        }));
    }

    // --- Patch Operations ---

    #[test]
    fn test_can_handle_create_patch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreatePatch {
            repo_id: "abcd1234".to_string(),
            title: "Add new feature".to_string(),
            description: "This patch adds a new feature".to_string(),
            base: "base_commit".to_string(),
            head: "head_commit".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_patches() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListPatches {
            repo_id: "abcd1234".to_string(),
            state: Some("open".to_string()),
            limit: Some(20),
        }));
    }

    #[test]
    fn test_can_handle_get_patch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetPatch {
            repo_id: "abcd1234".to_string(),
            patch_id: "patch123".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_update_patch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeUpdatePatch {
            repo_id: "abcd1234".to_string(),
            patch_id: "patch123".to_string(),
            head: "new_head".to_string(),
            message: Some("Updated the patch".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_approve_patch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeApprovePatch {
            repo_id: "abcd1234".to_string(),
            patch_id: "patch123".to_string(),
            commit: "approved_commit".to_string(),
            message: Some("LGTM".to_string()),
        }));
    }

    #[test]
    fn test_can_handle_merge_patch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeMergePatch {
            repo_id: "abcd1234".to_string(),
            patch_id: "patch123".to_string(),
            merge_commit: "merge_commit_hash".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_close_patch() {
        let handler = ForgeHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ForgeClosePatch {
            repo_id: "abcd1234".to_string(),
            patch_id: "patch123".to_string(),
            reason: Some("Superseded by another patch".to_string()),
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

        // Secrets requests
        #[cfg(feature = "secrets")]
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

    /// Test all 44 Forge request variants are handled by can_handle()
    #[test]
    fn test_all_forge_variants_covered() {
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

        // Issues (6)
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreateIssue {
            repo_id: "x".into(),
            title: "x".into(),
            body: "x".into(),
            labels: vec![]
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListIssues {
            repo_id: "x".into(),
            state: None,
            limit: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetIssue {
            repo_id: "x".into(),
            issue_id: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCommentIssue {
            repo_id: "x".into(),
            issue_id: "x".into(),
            body: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCloseIssue {
            repo_id: "x".into(),
            issue_id: "x".into(),
            reason: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeReopenIssue {
            repo_id: "x".into(),
            issue_id: "x".into()
        }));

        // Patches (7)
        assert!(handler.can_handle(&ClientRpcRequest::ForgeCreatePatch {
            repo_id: "x".into(),
            title: "x".into(),
            description: "x".into(),
            base: "x".into(),
            head: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeListPatches {
            repo_id: "x".into(),
            state: None,
            limit: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeGetPatch {
            repo_id: "x".into(),
            patch_id: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeUpdatePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            head: "x".into(),
            message: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeApprovePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            commit: "x".into(),
            message: None
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeMergePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            merge_commit: "x".into()
        }));
        assert!(handler.can_handle(&ClientRpcRequest::ForgeClosePatch {
            repo_id: "x".into(),
            patch_id: "x".into(),
            reason: None
        }));

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

        // Total: 3 + 7 + 6 + 6 + 7 + 1 + 8 + 6 = 44 distinct request types
        // (matches the can_handle() match block)
    }
}
