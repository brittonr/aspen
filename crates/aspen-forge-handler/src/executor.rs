//! Forge service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` to handle native forge operations
//! (federation + git bridge) when invoked via the RPC protocol.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;

use crate::handler::handlers::ForgeNodeRef;

// ── Conversion helpers ──────────────────────────────────────────────

fn commit_to_info(hash: blake3::Hash, c: &aspen_forge::git::CommitObject) -> aspen_client_api::ForgeCommitInfo {
    aspen_client_api::ForgeCommitInfo {
        hash: hash.to_hex().to_string(),
        tree: hex::encode(c.tree),
        parents: c.parents.iter().map(hex::encode).collect(),
        author_name: c.author.name.clone(),
        author_email: Some(c.author.email.clone()),
        author_key: c.author.public_key.as_ref().map(|k| k.to_string()),
        author_npub: c.author.npub.clone(),
        message: c.message.clone(),
        timestamp_ms: c.author.timestamp_ms,
    }
}

fn issue_to_info(id: &blake3::Hash, issue: &aspen_forge::cob::Issue) -> aspen_client_api::ForgeIssueInfo {
    aspen_client_api::ForgeIssueInfo {
        id: id.to_hex().to_string(),
        title: issue.title.clone(),
        body: issue.body.clone(),
        state: if issue.state.is_open() { "open" } else { "closed" }.into(),
        labels: issue.labels.iter().cloned().collect(),
        comment_count: issue.comments.len() as u32,
        assignees: issue.assignees.iter().map(hex::encode).collect(),
        created_at_ms: issue.created_at_ms,
        updated_at_ms: issue.updated_at_ms,
    }
}

fn patch_to_info(id: &blake3::Hash, patch: &aspen_forge::cob::Patch) -> aspen_client_api::ForgePatchInfo {
    let state_str = match &patch.state {
        aspen_forge::cob::PatchState::Open => "open",
        aspen_forge::cob::PatchState::Merged { .. } => "merged",
        aspen_forge::cob::PatchState::Closed { .. } => "closed",
    };
    aspen_client_api::ForgePatchInfo {
        id: id.to_hex().to_string(),
        title: patch.title.clone(),
        description: patch.description.clone(),
        state: state_str.into(),
        base: hex::encode(patch.base),
        head: hex::encode(patch.head),
        labels: patch.labels.iter().cloned().collect(),
        revision_count: patch.revisions.len() as u32,
        approval_count: patch.approvals.len() as u32,
        assignees: patch.assignees.iter().map(hex::encode).collect(),
        created_at_ms: patch.created_at_ms,
        updated_at_ms: patch.updated_at_ms,
    }
}

/// Service executor for Forge operations (federation + git bridge).
///
/// Repos, objects, refs, issues, and patches are handled by the WASM
/// `aspen-forge-plugin`. This executor retains only operations that
/// require `ForgeNode` context or federation infrastructure.
pub struct ForgeServiceExecutor {
    forge_node: ForgeNodeRef,
    #[cfg(feature = "global-discovery")]
    content_discovery: Option<Arc<dyn aspen_core::ContentDiscovery>>,
    #[cfg(feature = "global-discovery")]
    federation_discovery: Option<Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
    federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
    /// Optional hook service for emitting forge events.
    #[cfg(all(feature = "hooks", feature = "git-bridge"))]
    hook_service: Option<Arc<aspen_hooks::HookService>>,
    /// Node ID for hook event metadata.
    #[cfg_attr(not(all(feature = "hooks", feature = "git-bridge")), allow(dead_code))]
    node_id: u64,
}

impl ForgeServiceExecutor {
    /// Variant names handled by this executor (for testing without constructing).
    pub const HANDLES: &'static [&'static str] = &[
        "ForgeCreateRepo",
        "ForgeGetRepo",
        "ForgeListRepos",
        "ForgeStoreBlob",
        "ForgeGetBlob",
        "ForgeCreateTree",
        "ForgeGetTree",
        "ForgeCommit",
        "ForgeGetCommit",
        "ForgeLog",
        "ForgeGetRef",
        "ForgeSetRef",
        "ForgeDeleteRef",
        "ForgeCasRef",
        "ForgeListBranches",
        "ForgeListTags",
        "ForgeCreateIssue",
        "ForgeListIssues",
        "ForgeGetIssue",
        "ForgeCommentIssue",
        "ForgeCloseIssue",
        "ForgeReopenIssue",
        "ForgeCreatePatch",
        "ForgeListPatches",
        "ForgeGetPatch",
        "ForgeUpdatePatch",
        "ForgeApprovePatch",
        "ForgeMergePatch",
        "ForgeClosePatch",
        "ForgeGetDelegateKey",
        "GetFederationStatus",
        "ListDiscoveredClusters",
        "GetDiscoveredCluster",
        "TrustCluster",
        "UntrustCluster",
        "FederateRepository",
        "ListFederatedRepositories",
        "ForgeFetchFederated",
        "GitBridgeListRefs",
        "GitBridgeFetch",
        "GitBridgePush",
        "GitBridgePushStart",
        "GitBridgePushChunk",
        "GitBridgePushComplete",
        "GitBridgeProbeObjects",
    ];

    pub const SERVICE_NAME: &'static str = "forge";
    pub const PRIORITY: u32 = 540;
    pub const APP_ID: Option<&'static str> = Some("forge");

    /// Create a new forge service executor with captured dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        forge_node: ForgeNodeRef,
        #[cfg(feature = "global-discovery")] content_discovery: Option<Arc<dyn aspen_core::ContentDiscovery>>,
        #[cfg(feature = "global-discovery")] federation_discovery: Option<
            Arc<aspen_cluster::federation::FederationDiscoveryService>,
        >,
        federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
        federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
        #[cfg(all(feature = "hooks", feature = "git-bridge"))] hook_service: Option<Arc<aspen_hooks::HookService>>,
        node_id: u64,
    ) -> Self {
        Self {
            forge_node,
            #[cfg(feature = "global-discovery")]
            content_discovery,
            #[cfg(feature = "global-discovery")]
            federation_discovery,
            federation_identity,
            federation_trust_manager,
            #[cfg(all(feature = "hooks", feature = "git-bridge"))]
            hook_service,
            node_id,
        }
    }
}

impl ForgeServiceExecutor {
    // ========================================================================
    // Helper Methods for Git Operations
    // ========================================================================

    async fn handle_get_repo(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRepoInfo;
        use aspen_client_api::ForgeRepoResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {}", e))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        match self.forge_node.get_repo(&repo_id).await {
            Ok(identity) => {
                let repo_info = ForgeRepoInfo {
                    id: identity.repo_id().to_hex(),
                    name: identity.name.clone(),
                    description: None,
                    default_branch: "main".to_string(),
                    delegates: identity.delegates.iter().map(|d| d.to_string()).collect(),
                    threshold_delegates: identity.threshold,
                    created_at_ms: identity.created_at_ms,
                };
                Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                    is_success: true,
                    repo: Some(repo_info),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                is_success: false,
                repo: None,
                error: Some(format!("{}", e)),
            })),
        }
    }

    async fn handle_store_blob(&self, _repo_id: String, content: Vec<u8>) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeBlobResultResponse;

        match self.forge_node.git.store_blob(content.clone()).await {
            Ok(hash) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: true,
                hash: Some(hash.to_hex().to_string()),
                content: None,
                size: Some(content.len() as u64),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: false,
                hash: None,
                content: None,
                size: None,
                error: Some(format!("{}", e)),
            })),
        }
    }

    async fn handle_get_blob(&self, hash: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeBlobResultResponse;

        let hash = blake3::Hash::from_hex(&hash).map_err(|e| anyhow::anyhow!("invalid hash: {}", e))?;

        match self.forge_node.git.get_blob(&hash).await {
            Ok(content) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: true,
                hash: Some(hash.to_hex().to_string()),
                content: Some(content.clone()),
                size: Some(content.len() as u64),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                is_success: false,
                hash: None,
                content: None,
                size: None,
                error: Some(format!("{}", e)),
            })),
        }
    }

    async fn handle_create_tree(&self, entries_json: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeTreeEntry;
        use aspen_client_api::ForgeTreeResultResponse;

        let entries: Vec<ForgeTreeEntry> =
            serde_json::from_str(&entries_json).map_err(|e| anyhow::anyhow!("invalid tree entries JSON: {}", e))?;

        // Convert to TreeEntry objects
        let tree_entries: Result<Vec<_>, _> = entries
            .into_iter()
            .map(|e| -> Result<aspen_forge::git::TreeEntry, anyhow::Error> {
                let hash = blake3::Hash::from_hex(&e.hash)?;
                Ok(aspen_forge::git::TreeEntry {
                    mode: e.mode,
                    name: e.name,
                    hash: hash.into(),
                })
            })
            .collect();

        let tree_entries = tree_entries?;

        match self.forge_node.git.create_tree(&tree_entries).await {
            Ok(hash) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: true,
                hash: Some(hash.to_hex().to_string()),
                entries: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("{}", e)),
            })),
        }
    }

    async fn handle_get_tree(&self, hash: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeTreeEntry;
        use aspen_client_api::ForgeTreeResultResponse;

        let hash = blake3::Hash::from_hex(&hash).map_err(|e| anyhow::anyhow!("invalid hash: {}", e))?;

        match self.forge_node.git.get_tree(&hash).await {
            Ok(tree) => {
                let entries: Vec<ForgeTreeEntry> = tree
                    .entries
                    .into_iter()
                    .map(|e| ForgeTreeEntry {
                        mode: e.mode,
                        name: e.name,
                        hash: hex::encode(e.hash),
                    })
                    .collect();

                Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                    is_success: true,
                    hash: Some(hash.to_hex().to_string()),
                    entries: Some(entries),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                is_success: false,
                hash: None,
                entries: None,
                error: Some(format!("{}", e)),
            })),
        }
    }

    // ========================================================================
    // Commit Operations
    // ========================================================================

    async fn handle_commit(
        &self,
        repo_id: String,
        tree: String,
        parents: Vec<String>,
        message: String,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeCommitResultResponse;

        let tree_hash = blake3::Hash::from_hex(&tree).map_err(|e| anyhow::anyhow!("invalid tree hash: {}", e))?;
        let parent_hashes: Vec<blake3::Hash> = parents
            .iter()
            .map(|p| blake3::Hash::from_hex(p).map_err(|e| anyhow::anyhow!("invalid parent hash: {}", e)))
            .collect::<Result<_, _>>()?;

        // repo_id needed only for future per-repo auth; commit() is content-addressed
        let _repo_hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {}", e))?;

        match self.forge_node.git.commit(tree_hash, parent_hashes, message).await {
            Ok(hash) => {
                let commit = self.forge_node.git.get_commit(&hash).await?;
                Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                    is_success: true,
                    commit: Some(commit_to_info(hash, &commit)),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_get_commit(&self, hash: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeCommitResultResponse;

        let hash = blake3::Hash::from_hex(&hash).map_err(|e| anyhow::anyhow!("invalid hash: {}", e))?;
        match self.forge_node.git.get_commit(&hash).await {
            Ok(commit) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: true,
                commit: Some(commit_to_info(hash, &commit)),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                is_success: false,
                commit: None,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_log(
        &self,
        repo_id: String,
        ref_name: Option<String>,
        limit: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeLogResultResponse;

        let limit = limit.unwrap_or(20).min(100) as usize;
        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {}", e))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let ref_name = ref_name.unwrap_or_else(|| "main".to_string());

        // Try exact name, then with common prefixes (git bridge strips "refs/")
        let candidates = [
            ref_name.clone(),
            format!("heads/{ref_name}"),
            format!("refs/heads/{ref_name}"),
        ];
        let mut start_hash = None;
        for candidate in &candidates {
            if let Some(h) = self.forge_node.refs.get(&repo_id, candidate).await? {
                start_hash = Some(h);
                break;
            }
        }

        let start = match start_hash {
            Some(h) => h,
            None => {
                return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                    is_success: true,
                    commits: vec![],
                    count: 0,
                    error: None,
                }));
            }
        };

        let commits = self.walk_commit_history(start, limit).await?;
        let count = commits.len() as u32;
        Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
            is_success: true,
            commits,
            count,
            error: None,
        }))
    }

    /// Walk the commit graph from `start` collecting up to `limit` commits.
    async fn walk_commit_history(
        &self,
        start: blake3::Hash,
        limit: usize,
    ) -> Result<Vec<aspen_client_api::ForgeCommitInfo>> {
        let mut result = Vec::with_capacity(limit);
        let mut queue = std::collections::VecDeque::new();
        let mut visited = std::collections::HashSet::new();
        queue.push_back(start);

        while let Some(hash) = queue.pop_front() {
            if !visited.insert(hash) || result.len() >= limit {
                continue;
            }
            let commit = match self.forge_node.git.get_commit(&hash).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            for parent in &commit.parents {
                queue.push_back(blake3::Hash::from(*parent));
            }
            result.push(commit_to_info(hash, &commit));
        }
        Ok(result)
    }

    // ========================================================================
    // Ref Operations
    // ========================================================================

    async fn handle_get_ref(&self, repo_id: String, ref_name: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRefInfo;
        use aspen_client_api::ForgeRefResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        // Try exact name first, then common prefixes.
        // Git bridge strips "refs/" before storing, so "refs/heads/main" → "heads/main".
        // Web UI may pass bare branch names like "main".
        let candidates = [
            ref_name.clone(),
            format!("heads/{ref_name}"),
            format!("refs/heads/{ref_name}"),
        ];
        for candidate in &candidates {
            if let Some(h) = self.forge_node.refs.get(&repo_id, candidate).await? {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    is_success: true,
                    was_found: true,
                    ref_info: Some(ForgeRefInfo {
                        name: ref_name,
                        hash: h.to_hex().to_string(),
                    }),
                    previous_hash: None,
                    error: None,
                }));
            }
        }

        Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            is_success: true,
            was_found: false,
            ref_info: None,
            previous_hash: None,
            error: None,
        }))
    }

    async fn handle_set_ref(&self, repo_id: String, ref_name: String, hash: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let rid = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(rid);
        let target = blake3::Hash::from_hex(&hash).map_err(|e| anyhow::anyhow!("invalid hash: {e}"))?;

        match self.forge_node.refs.set(&repo_id, &ref_name, target).await {
            Ok(()) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_delete_ref(&self, repo_id: String, ref_name: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let rid = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(rid);

        match self.forge_node.refs.delete(&repo_id, &ref_name).await {
            Ok(()) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_cas_ref(
        &self,
        repo_id: String,
        ref_name: String,
        expected: Option<String>,
        new_hash: String,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRefResultResponse;

        let rid = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(rid);
        let new = blake3::Hash::from_hex(&new_hash).map_err(|e| anyhow::anyhow!("invalid new hash: {e}"))?;
        let exp = expected
            .as_deref()
            .map(|h| blake3::Hash::from_hex(h).map_err(|e| anyhow::anyhow!("invalid expected hash: {e}")))
            .transpose()?;

        match self.forge_node.refs.compare_and_set(&repo_id, &ref_name, exp, new).await {
            Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                is_success: true,
                was_found: true,
                ref_info: None,
                previous_hash: expected,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                is_success: false,
                was_found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_list_branches(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRefInfo;
        use aspen_client_api::ForgeRefListResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        let branches = self.forge_node.refs.list_branches(&repo_id).await?;
        let count = branches.len() as u32;
        let refs: Vec<ForgeRefInfo> = branches
            .into_iter()
            .map(|(name, h)| ForgeRefInfo {
                name,
                hash: h.to_hex().to_string(),
            })
            .collect();
        Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
            is_success: true,
            refs,
            count,
            error: None,
        }))
    }

    async fn handle_list_tags(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRefInfo;
        use aspen_client_api::ForgeRefListResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        let tags = self.forge_node.refs.list_tags(&repo_id).await?;
        let count = tags.len() as u32;
        let refs: Vec<ForgeRefInfo> = tags
            .into_iter()
            .map(|(name, h)| ForgeRefInfo {
                name,
                hash: h.to_hex().to_string(),
            })
            .collect();
        Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
            is_success: true,
            refs,
            count,
            error: None,
        }))
    }

    // ========================================================================
    // Issue Operations
    // ========================================================================

    async fn handle_create_issue(
        &self,
        repo_id: String,
        title: String,
        body: String,
        labels: Vec<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        match self.forge_node.cobs.create_issue(&repo_id, title, body, labels).await {
            Ok(_id) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_list_issues(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeIssueListResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        let ids = self.forge_node.cobs.list_issues(&repo_id).await?;
        let mut issues = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Ok(issue) = self.forge_node.cobs.resolve_issue(&repo_id, id).await {
                issues.push(issue_to_info(id, &issue));
            }
        }
        let count = issues.len() as u32;
        Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
            is_success: true,
            issues,
            count,
            error: None,
        }))
    }

    async fn handle_get_issue(&self, repo_id: String, issue_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeCommentInfo;
        use aspen_client_api::ForgeIssueResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let iid = blake3::Hash::from_hex(&issue_id).map_err(|e| anyhow::anyhow!("invalid issue ID: {e}"))?;

        let issue = self.forge_node.cobs.resolve_issue(&repo_id, &iid).await?;
        let comments: Vec<ForgeCommentInfo> = issue
            .comments
            .iter()
            .map(|c| ForgeCommentInfo {
                hash: hex::encode(c.change_hash),
                author: hex::encode(c.author),
                body: c.body.clone(),
                timestamp_ms: c.timestamp_ms,
            })
            .collect();

        Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            is_success: true,
            issue: Some(issue_to_info(&iid, &issue)),
            comments: Some(comments),
            error: None,
        }))
    }

    async fn handle_comment_issue(&self, repo_id: String, issue_id: String, body: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let iid = blake3::Hash::from_hex(&issue_id).map_err(|e| anyhow::anyhow!("invalid issue ID: {e}"))?;

        match self.forge_node.cobs.add_comment(&repo_id, &iid, body).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_close_issue(
        &self,
        repo_id: String,
        issue_id: String,
        reason: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let iid = blake3::Hash::from_hex(&issue_id).map_err(|e| anyhow::anyhow!("invalid issue ID: {e}"))?;

        match self.forge_node.cobs.close_issue(&repo_id, &iid, reason).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_reopen_issue(&self, repo_id: String, issue_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let iid = blake3::Hash::from_hex(&issue_id).map_err(|e| anyhow::anyhow!("invalid issue ID: {e}"))?;

        match self.forge_node.cobs.reopen_issue(&repo_id, &iid).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    // ========================================================================
    // Patch Operations
    // ========================================================================

    async fn handle_create_patch(
        &self,
        repo_id: String,
        title: String,
        description: String,
        base: String,
        head: String,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let base_hash = blake3::Hash::from_hex(&base).map_err(|e| anyhow::anyhow!("invalid base hash: {e}"))?;
        let head_hash = blake3::Hash::from_hex(&head).map_err(|e| anyhow::anyhow!("invalid head hash: {e}"))?;

        match self.forge_node.cobs.create_patch(&repo_id, title, description, base_hash, head_hash).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_list_patches(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgePatchListResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        let ids = self.forge_node.cobs.list_patches(&repo_id).await?;
        let mut patches = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Ok(patch) = self.forge_node.cobs.resolve_patch(&repo_id, id).await {
                patches.push(patch_to_info(id, &patch));
            }
        }
        let count = patches.len() as u32;
        Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
            is_success: true,
            patches,
            count,
            error: None,
        }))
    }

    async fn handle_get_patch(&self, repo_id: String, patch_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgePatchApproval;
        use aspen_client_api::ForgePatchResultResponse;
        use aspen_client_api::ForgePatchRevision;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;

        let patch = self.forge_node.cobs.resolve_patch(&repo_id, &pid).await?;
        let revisions: Vec<ForgePatchRevision> = patch
            .revisions
            .iter()
            .map(|r| ForgePatchRevision {
                hash: hex::encode(r.head),
                head: hex::encode(r.head),
                message: r.message.clone(),
                author: hex::encode(r.author),
                timestamp_ms: r.timestamp_ms,
            })
            .collect();
        let approvals: Vec<ForgePatchApproval> = patch
            .approvals
            .iter()
            .map(|a| ForgePatchApproval {
                author: hex::encode(a.author),
                commit: hex::encode(a.commit),
                message: a.message.clone(),
                timestamp_ms: a.timestamp_ms,
            })
            .collect();

        Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            is_success: true,
            patch: Some(patch_to_info(&pid, &patch)),
            comments: None,
            revisions: Some(revisions),
            approvals: Some(approvals),
            error: None,
        }))
    }

    async fn handle_update_patch(
        &self,
        repo_id: String,
        patch_id: String,
        head: String,
        message: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;
        let head_hash = blake3::Hash::from_hex(&head).map_err(|e| anyhow::anyhow!("invalid head hash: {e}"))?;

        match self.forge_node.cobs.update_patch(&repo_id, &pid, head_hash, message).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_approve_patch(
        &self,
        repo_id: String,
        patch_id: String,
        commit: String,
        message: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;
        let commit_hash = blake3::Hash::from_hex(&commit).map_err(|e| anyhow::anyhow!("invalid commit hash: {e}"))?;

        match self.forge_node.cobs.approve_patch(&repo_id, &pid, commit_hash, message).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_merge_patch(
        &self,
        repo_id: String,
        patch_id: String,
        merge_commit: String,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;
        let mc =
            blake3::Hash::from_hex(&merge_commit).map_err(|e| anyhow::anyhow!("invalid merge commit hash: {e}"))?;

        match self.forge_node.cobs.merge_patch(&repo_id, &pid, mc).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_close_patch(
        &self,
        repo_id: String,
        patch_id: String,
        reason: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;

        match self.forge_node.cobs.close_patch(&repo_id, &pid, reason).await {
            Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: true,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                is_success: false,
                error: Some(format!("{e}")),
            })),
        }
    }

    /// Emit a ForgePushCompleted hook event for successful ref updates.
    ///
    /// This is fire-and-forget — hook dispatch failures are logged but don't
    /// affect the push response.
    #[cfg(all(feature = "hooks", feature = "git-bridge"))]
    fn emit_push_hook(&self, repo_id: &str, ref_results: &[aspen_client_api::GitBridgeRefResult]) {
        use aspen_hooks_types::event::ForgePushCompletedPayload;
        use aspen_hooks_types::event::HookEvent;
        use aspen_hooks_types::event::HookEventType;

        let hook_service = match &self.hook_service {
            Some(s) => s.clone(),
            None => return,
        };

        // Emit one hook per successful ref update
        for ref_result in ref_results {
            if !ref_result.is_success {
                continue;
            }

            let payload = ForgePushCompletedPayload {
                repo_id: repo_id.to_string(),
                ref_name: ref_result.ref_name.clone(),
                new_hash: String::new(), // Hash details available in gossip announcements
                old_hash: None,
                pusher: String::new(),
            };

            let event = HookEvent::new(
                HookEventType::ForgePushCompleted,
                self.node_id,
                serde_json::to_value(&payload).unwrap_or_default(),
            );

            let service = hook_service.clone();
            let ref_name = ref_result.ref_name.clone();
            tokio::spawn(async move {
                if let Err(e) = service.dispatch(&event).await {
                    tracing::warn!(
                        error = %e,
                        ref_name = %ref_name,
                        "failed to dispatch forge push hook event"
                    );
                }
            });
        }
    }
}

#[async_trait]
impl ServiceExecutor for ForgeServiceExecutor {
    fn service_name(&self) -> &'static str {
        "forge"
    }

    fn handles(&self) -> &'static [&'static str] {
        Self::HANDLES
    }

    fn priority(&self) -> u32 {
        540
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("forge")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        use crate::handler::handlers::federation::*;

        match request {
            // Repository operations (previously WASM-only, now native for self-hosting)
            ClientRpcRequest::ForgeCreateRepo {
                name,
                description,
                default_branch: _,
            } => {
                use aspen_client_api::ForgeRepoInfo;
                use aspen_client_api::ForgeRepoResultResponse;

                // Use the node's own public key as the default delegate
                let delegates = vec![self.forge_node.public_key()];
                match self.forge_node.create_repo(&name, delegates, 1).await {
                    Ok(identity) => {
                        let repo_info = ForgeRepoInfo {
                            id: identity.repo_id().to_hex(),
                            name: name.clone(),
                            description,
                            default_branch: "main".to_string(),
                            delegates: identity.delegates.iter().map(|d| d.to_string()).collect(),
                            threshold_delegates: identity.threshold,
                            created_at_ms: identity.created_at_ms,
                        };
                        Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                            is_success: true,
                            repo: Some(repo_info),
                            error: None,
                        }))
                    }
                    Err(e) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        is_success: false,
                        repo: None,
                        error: Some(format!("{}", e)),
                    })),
                }
            }
            ClientRpcRequest::ForgeListRepos { limit, offset } => {
                use aspen_client_api::ForgeRepoInfo;
                use aspen_client_api::ForgeRepoListResultResponse;

                let limit = limit.unwrap_or(100).min(1000) as usize;
                let offset = offset.unwrap_or(0) as usize;
                match self.forge_node.list_repos().await {
                    Ok(repos) => {
                        let count = repos.len() as u32;
                        let repos: Vec<ForgeRepoInfo> = repos
                            .into_iter()
                            .skip(offset)
                            .take(limit)
                            .map(|identity| ForgeRepoInfo {
                                id: identity.repo_id().to_hex(),
                                name: identity.name.clone(),
                                description: None,
                                default_branch: "main".to_string(),
                                delegates: identity.delegates.iter().map(|d| d.to_string()).collect(),
                                threshold_delegates: identity.threshold,
                                created_at_ms: identity.created_at_ms,
                            })
                            .collect();
                        Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                            is_success: true,
                            repos,
                            count,
                            error: None,
                        }))
                    }
                    Err(e) => Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                        is_success: false,
                        repos: vec![],
                        count: 0,
                        error: Some(format!("{}", e)),
                    })),
                }
            }
            ClientRpcRequest::ForgeGetRepo { repo_id } => self.handle_get_repo(repo_id).await,

            // Blob operations
            ClientRpcRequest::ForgeStoreBlob { repo_id, content } => self.handle_store_blob(repo_id, content).await,
            ClientRpcRequest::ForgeGetBlob { hash } => self.handle_get_blob(hash).await,

            // Tree operations
            ClientRpcRequest::ForgeCreateTree {
                repo_id: _,
                entries_json,
            } => self.handle_create_tree(entries_json).await,
            ClientRpcRequest::ForgeGetTree { hash } => self.handle_get_tree(hash).await,

            // Commit operations
            ClientRpcRequest::ForgeCommit {
                repo_id,
                tree,
                parents,
                message,
            } => self.handle_commit(repo_id, tree, parents, message).await,
            ClientRpcRequest::ForgeGetCommit { hash } => self.handle_get_commit(hash).await,
            ClientRpcRequest::ForgeLog {
                repo_id,
                ref_name,
                limit,
            } => self.handle_log(repo_id, ref_name, limit).await,

            // Ref operations
            ClientRpcRequest::ForgeGetRef { repo_id, ref_name } => self.handle_get_ref(repo_id, ref_name).await,
            ClientRpcRequest::ForgeSetRef {
                repo_id,
                ref_name,
                hash,
                signer: _,
                signature: _,
                timestamp_ms: _,
            } => self.handle_set_ref(repo_id, ref_name, hash).await,
            ClientRpcRequest::ForgeDeleteRef { repo_id, ref_name } => self.handle_delete_ref(repo_id, ref_name).await,
            ClientRpcRequest::ForgeCasRef {
                repo_id,
                ref_name,
                expected,
                new_hash,
                signer: _,
                signature: _,
                timestamp_ms: _,
            } => self.handle_cas_ref(repo_id, ref_name, expected, new_hash).await,
            ClientRpcRequest::ForgeListBranches { repo_id } => self.handle_list_branches(repo_id).await,
            ClientRpcRequest::ForgeListTags { repo_id } => self.handle_list_tags(repo_id).await,

            // Issue operations
            ClientRpcRequest::ForgeCreateIssue {
                repo_id,
                title,
                body,
                labels,
            } => self.handle_create_issue(repo_id, title, body, labels).await,
            ClientRpcRequest::ForgeListIssues {
                repo_id,
                state: _,
                limit: _,
            } => self.handle_list_issues(repo_id).await,
            ClientRpcRequest::ForgeGetIssue { repo_id, issue_id } => self.handle_get_issue(repo_id, issue_id).await,
            ClientRpcRequest::ForgeCommentIssue {
                repo_id,
                issue_id,
                body,
            } => self.handle_comment_issue(repo_id, issue_id, body).await,
            ClientRpcRequest::ForgeCloseIssue {
                repo_id,
                issue_id,
                reason,
            } => self.handle_close_issue(repo_id, issue_id, reason).await,
            ClientRpcRequest::ForgeReopenIssue { repo_id, issue_id } => {
                self.handle_reopen_issue(repo_id, issue_id).await
            }

            // Patch operations
            ClientRpcRequest::ForgeCreatePatch {
                repo_id,
                title,
                description,
                base,
                head,
            } => self.handle_create_patch(repo_id, title, description, base, head).await,
            ClientRpcRequest::ForgeListPatches {
                repo_id,
                state: _,
                limit: _,
            } => self.handle_list_patches(repo_id).await,
            ClientRpcRequest::ForgeGetPatch { repo_id, patch_id } => self.handle_get_patch(repo_id, patch_id).await,
            ClientRpcRequest::ForgeUpdatePatch {
                repo_id,
                patch_id,
                head,
                message,
            } => self.handle_update_patch(repo_id, patch_id, head, message).await,
            ClientRpcRequest::ForgeApprovePatch {
                repo_id,
                patch_id,
                commit,
                message,
            } => self.handle_approve_patch(repo_id, patch_id, commit, message).await,
            ClientRpcRequest::ForgeMergePatch {
                repo_id,
                patch_id,
                merge_commit,
            } => self.handle_merge_patch(repo_id, patch_id, merge_commit).await,
            ClientRpcRequest::ForgeClosePatch {
                repo_id,
                patch_id,
                reason,
            } => self.handle_close_patch(repo_id, patch_id, reason).await,

            // Federation operations
            ClientRpcRequest::ForgeGetDelegateKey { repo_id } => {
                handle_get_delegate_key(&self.forge_node, repo_id).await
            }
            ClientRpcRequest::GetFederationStatus => {
                #[cfg(feature = "global-discovery")]
                {
                    handle_get_federation_status(
                        &self.forge_node,
                        self.content_discovery.as_ref(),
                        self.federation_discovery.as_ref(),
                        self.federation_identity.as_ref(),
                    )
                    .await
                }
                #[cfg(not(feature = "global-discovery"))]
                {
                    handle_get_federation_status(&self.forge_node, self.federation_identity.as_ref()).await
                }
            }
            ClientRpcRequest::ListDiscoveredClusters => {
                #[cfg(feature = "global-discovery")]
                {
                    handle_list_discovered_clusters(self.federation_discovery.as_ref()).await
                }
                #[cfg(not(feature = "global-discovery"))]
                {
                    handle_list_discovered_clusters().await
                }
            }
            ClientRpcRequest::GetDiscoveredCluster { cluster_key } => {
                #[cfg(feature = "global-discovery")]
                {
                    handle_get_discovered_cluster(self.federation_discovery.as_ref(), cluster_key).await
                }
                #[cfg(not(feature = "global-discovery"))]
                {
                    handle_get_discovered_cluster(cluster_key).await
                }
            }
            ClientRpcRequest::TrustCluster { cluster_key } => {
                handle_trust_cluster(self.federation_trust_manager.as_ref(), cluster_key).await
            }
            ClientRpcRequest::UntrustCluster { cluster_key } => {
                handle_untrust_cluster(self.federation_trust_manager.as_ref(), cluster_key).await
            }
            ClientRpcRequest::FederateRepository { repo_id, mode } => {
                handle_federate_repository(&self.forge_node, repo_id, mode).await
            }
            ClientRpcRequest::ListFederatedRepositories => handle_list_federated_repositories(&self.forge_node).await,
            ClientRpcRequest::ForgeFetchFederated {
                federated_id,
                remote_cluster,
            } => handle_fetch_federated(&self.forge_node, federated_id, remote_cluster).await,

            // Git Bridge operations
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeListRefs { repo_id } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_list_refs(&self.forge_node, repo_id).await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetch { repo_id, want, have } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_fetch(&self.forge_node, repo_id, want, have)
                    .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePush { repo_id, objects, refs } => {
                let resp = crate::handler::handlers::git_bridge::handle_git_bridge_push(
                    &self.forge_node,
                    repo_id.clone(),
                    objects,
                    refs,
                )
                .await?;
                // Emit hook events for successful pushes
                #[cfg(feature = "hooks")]
                if let ClientRpcResponse::GitBridgePush(ref push_resp) = resp
                    && push_resp.is_success
                {
                    self.emit_push_hook(&repo_id, &push_resp.ref_results);
                }
                Ok(resp)
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushStart {
                repo_id,
                total_objects,
                total_size_bytes,
                refs,
                metadata,
            } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_push_start(
                    &self.forge_node,
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
                crate::handler::handlers::git_bridge::handle_git_bridge_push_chunk(
                    &self.forge_node,
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
            } => {
                let resp = crate::handler::handlers::git_bridge::handle_git_bridge_push_complete(
                    &self.forge_node,
                    session_id,
                    content_hash,
                )
                .await?;
                // Emit hook events for successful chunked pushes
                #[cfg(feature = "hooks")]
                if let ClientRpcResponse::GitBridgePushComplete(ref push_resp) = resp
                    && push_resp.is_success
                {
                    self.emit_push_hook("", &push_resp.ref_results);
                }
                Ok(resp)
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeProbeObjects { repo_id, sha1s } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_probe_objects(&self.forge_node, repo_id, sha1s)
                    .await
            }

            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgePush { .. }
            | ClientRpcRequest::GitBridgePushStart { .. }
            | ClientRpcRequest::GitBridgePushChunk { .. }
            | ClientRpcRequest::GitBridgePushComplete { .. }
            | ClientRpcRequest::GitBridgeProbeObjects { .. } => Ok(ClientRpcResponse::error(
                "GIT_BRIDGE_UNAVAILABLE",
                "Git bridge feature not enabled. Rebuild with --features git-bridge",
            )),

            _ => unreachable!("ForgeServiceExecutor received unhandled request"),
        }
    }
}
