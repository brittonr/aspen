//! Forge service executor for typed RPC dispatch.
//!
//! Implements `ServiceExecutor` to handle native forge operations
//! (federation + git bridge) when invoked via the RPC protocol.

use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
#[cfg(not(feature = "git-bridge"))]
use aspen_client_api::ErrorResponse;
use aspen_client_api::forge::FORGE_GIT_BACKEND_TRANSPORT_ID;
use aspen_client_api::forge::ForgeRepoBackend;
use aspen_client_api::forge::ForgeRepoBackendManifest;
use aspen_client_api::forge::ForgeRepoBackendRoute;
use aspen_client_api::forge::JJ_NATIVE_FORGE_ALPN_STR;
use aspen_core::KeyValueStore as _;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;

use crate::handler::handlers::ForgeNodeRef;

fn unexpected_request_kind(request_family: &str) -> Result<ClientRpcResponse> {
    Ok(ClientRpcResponse::error("UNEXPECTED_REQUEST_KIND", format!("unexpected {request_family} request")))
}

fn usize_from_u32(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

const PLUGIN_ROUTE_SCAN_LIMIT: u32 = 1_000;

fn git_only_backends() -> Vec<ForgeRepoBackend> {
    ForgeRepoBackendManifest::git_only().backends
}

fn git_backend_route(node_id: u64) -> ForgeRepoBackendRoute {
    ForgeRepoBackendRoute {
        backend: ForgeRepoBackend::Git,
        node_id: Some(node_id),
        transport_id: Some(FORGE_GIT_BACKEND_TRANSPORT_ID.to_string()),
        transport_version: None,
    }
}

fn jj_backend_route_from_manifest(
    node_id: u64,
    manifest: &aspen_plugin_api::PluginManifest,
) -> Option<ForgeRepoBackendRoute> {
    manifest
        .protocols
        .iter()
        .find(|protocol| protocol.identifier == JJ_NATIVE_FORGE_ALPN_STR)
        .map(|protocol| ForgeRepoBackendRoute {
            backend: ForgeRepoBackend::Jj,
            node_id: Some(node_id),
            transport_id: Some(protocol.identifier.clone()),
            transport_version: Some(protocol.version),
        })
}

fn routes_for_backends(
    backends: &[ForgeRepoBackend],
    active_routes: &[ForgeRepoBackendRoute],
) -> Vec<ForgeRepoBackendRoute> {
    active_routes
        .iter()
        .filter(|route| backends.iter().any(|backend| *backend == route.backend))
        .cloned()
        .collect()
}

fn repo_info_from_identity_with_backends(
    identity: &aspen_forge::identity::RepoIdentity,
    description: Option<String>,
    backends: Vec<ForgeRepoBackend>,
    active_routes: &[ForgeRepoBackendRoute],
) -> aspen_client_api::ForgeRepoInfo {
    let backend_routes = routes_for_backends(&backends, active_routes);

    aspen_client_api::ForgeRepoInfo {
        id: identity.repo_id().to_hex(),
        name: identity.name.clone(),
        description: description.or_else(|| identity.description.clone()),
        default_branch: identity.default_branch.clone(),
        delegates: identity.delegates.iter().map(|delegate| delegate.to_string()).collect(),
        threshold_delegates: identity.threshold,
        created_at_ms: identity.created_at_ms,
        backends,
        backend_routes,
    }
}

fn repo_info_from_identity(
    identity: &aspen_forge::identity::RepoIdentity,
    description: Option<String>,
    active_routes: &[ForgeRepoBackendRoute],
) -> aspen_client_api::ForgeRepoInfo {
    repo_info_from_identity_with_backends(identity, description, git_only_backends(), active_routes)
}

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
        author_display_name: None, // Resolved separately if needed
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
    let revision_count = u32::try_from(patch.revisions.len()).unwrap_or(u32::MAX);
    let approval_count = u32::try_from(patch.approvals.len()).unwrap_or(u32::MAX);
    debug_assert!(
        usize_from_u32(revision_count) >= patch.revisions.len(),
        "revision count stays lossless until saturation"
    );
    debug_assert!(
        usize_from_u32(approval_count) >= patch.approvals.len(),
        "approval count stays lossless until saturation"
    );
    aspen_client_api::ForgePatchInfo {
        id: id.to_hex().to_string(),
        title: patch.title.clone(),
        description: patch.description.clone(),
        state: state_str.into(),
        base: hex::encode(patch.base),
        head: hex::encode(patch.head),
        labels: patch.labels.iter().cloned().collect(),
        revision_count,
        approval_count,
        assignees: patch.assignees.iter().map(hex::encode).collect(),
        created_at_ms: patch.created_at_ms,
        updated_at_ms: patch.updated_at_ms,
    }
}

fn discussion_to_info(id: &blake3::Hash, d: &aspen_forge::cob::Discussion) -> aspen_client_api::ForgeDiscussionInfo {
    let state_str = match &d.state {
        aspen_forge::cob::DiscussionState::Open => "open",
        aspen_forge::cob::DiscussionState::Closed { .. } => "closed",
        aspen_forge::cob::DiscussionState::Locked => "locked",
    };
    aspen_client_api::ForgeDiscussionInfo {
        id: id.to_hex().to_string(),
        title: d.title.clone(),
        body: d.body.clone(),
        state: state_str.into(),
        labels: d.labels.iter().cloned().collect(),
        reply_count: d.replies.len() as u32,
        resolved_thread_count: d.resolved_threads.len() as u32,
        created_at_ms: d.created_at_ms,
        updated_at_ms: d.updated_at_ms,
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
    /// Cluster identity for federation handshakes (contains signing key).
    federation_cluster_identity: Option<Arc<aspen_cluster::federation::ClusterIdentity>>,
    /// Iroh endpoint for outbound federation connections.
    iroh_endpoint: Option<Arc<iroh::Endpoint>>,
    /// Optional hook service for emitting forge events.
    #[cfg(all(feature = "hooks", feature = "git-bridge"))]
    hook_service: Option<Arc<aspen_hooks::HookService>>,
    /// Node ID for hook event metadata.
    #[cfg_attr(not(all(feature = "hooks", feature = "git-bridge")), allow(dead_code))]
    node_id: u64,
    /// Nostr authentication service (created lazily from ForgeNode's key).
    nostr_auth: Arc<aspen_forge::identity::nostr_auth::NostrAuthService<dyn aspen_core::KeyValueStore>>,
}

pub(crate) struct ForgeServiceExecutorDeps {
    #[cfg(feature = "global-discovery")]
    pub(crate) content_discovery: Option<Arc<dyn aspen_core::ContentDiscovery>>,
    #[cfg(feature = "global-discovery")]
    pub(crate) federation_discovery: Option<Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    pub(crate) federation_identity: Option<Arc<aspen_cluster::federation::SignedClusterIdentity>>,
    pub(crate) federation_trust_manager: Option<Arc<aspen_cluster::federation::TrustManager>>,
    pub(crate) federation_cluster_identity: Option<Arc<aspen_cluster::federation::ClusterIdentity>>,
    pub(crate) iroh_endpoint: Option<Arc<iroh::Endpoint>>,
    #[cfg(all(feature = "hooks", feature = "git-bridge"))]
    pub(crate) hook_service: Option<Arc<aspen_hooks::HookService>>,
    pub(crate) node_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForgeRequestGroup {
    Repo,
    Blob,
    Tree,
    Commit,
    Ref,
    Issue,
    Patch,
    Discussion,
    Fork,
    Mirror,
    Federation,
    GitBridge,
    Nostr,
    Diff,
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
        "ForgeCheckMerge",
        "ForgeClosePatch",
        "ForgeCreateDiscussion",
        "ForgeListDiscussions",
        "ForgeGetDiscussion",
        "ForgeReplyDiscussion",
        "ForgeLockDiscussion",
        "ForgeUnlockDiscussion",
        "ForgeCloseDiscussion",
        "ForgeReopenDiscussion",
        "ForgeForkRepo",
        "ForgeSetMirror",
        "ForgeDisableMirror",
        "ForgeGetMirrorStatus",
        "ForgeGetDelegateKey",
        "GetFederationStatus",
        "ListDiscoveredClusters",
        "GetDiscoveredCluster",
        "TrustCluster",
        "UntrustCluster",
        "FederateRepository",
        "ListFederatedRepositories",
        "FederationSyncPeer",
        "FederationFetchRefs",
        "FederationGitListRefs",
        "FederationGitFetch",
        "FederationPull",
        "FederationPush",
        "FederationBidiSync",
        "ForgeFetchFederated",
        "GitBridgeListRefs",
        "GitBridgeFetch",
        "GitBridgeFetchStart",
        "GitBridgeFetchChunk",
        "GitBridgeFetchComplete",
        "GitBridgePush",
        "GitBridgePushStart",
        "GitBridgePushChunk",
        "GitBridgePushComplete",
        "GitBridgeProbeObjects",
        "NostrAuthChallenge",
        "NostrAuthVerify",
        "NostrGetProfile",
        "ForgeDiffCommits",
        "ForgeDiffRefs",
    ];

    pub const SERVICE_NAME: &'static str = "forge";
    pub const PRIORITY: u32 = 540;
    pub const APP_ID: Option<&'static str> = Some("forge");

    /// Create a new forge service executor with captured dependencies.
    pub(crate) fn new(forge_node: ForgeNodeRef, deps: ForgeServiceExecutorDeps) -> Self {
        // Build auth service from the forge node's key and KV store
        let identity_store = Arc::new(aspen_forge::identity::nostr_mapping::NostrIdentityStore::new(
            forge_node.kv().clone(),
            forge_node.secret_key(),
        ));
        let nostr_auth = Arc::new(aspen_forge::identity::nostr_auth::NostrAuthService::new(
            identity_store,
            forge_node.secret_key().clone(),
        ));

        Self {
            forge_node,
            #[cfg(feature = "global-discovery")]
            content_discovery: deps.content_discovery,
            #[cfg(feature = "global-discovery")]
            federation_discovery: deps.federation_discovery,
            federation_identity: deps.federation_identity,
            federation_trust_manager: deps.federation_trust_manager,
            federation_cluster_identity: deps.federation_cluster_identity,
            iroh_endpoint: deps.iroh_endpoint,
            #[cfg(all(feature = "hooks", feature = "git-bridge"))]
            hook_service: deps.hook_service,
            node_id: deps.node_id,
            nostr_auth,
        }
    }

    async fn active_backend_routes(&self) -> Vec<ForgeRepoBackendRoute> {
        let mut routes = vec![git_backend_route(self.node_id)];
        let request = aspen_core::ScanRequest {
            prefix: aspen_plugin_api::PLUGIN_KV_PREFIX.to_string(),
            limit_results: Some(PLUGIN_ROUTE_SCAN_LIMIT),
            continuation_token: None,
        };

        match self.forge_node.kv().scan_local(request).await {
            Ok(result) => {
                for entry in result.entries {
                    let Ok(manifest) = serde_json::from_str::<aspen_plugin_api::PluginManifest>(&entry.value) else {
                        continue;
                    };
                    if !manifest.enabled {
                        continue;
                    }
                    if let Some(route) = jj_backend_route_from_manifest(self.node_id, &manifest) {
                        routes.push(route);
                    }
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "failed to scan active plugin manifests for Forge backend routes");
            }
        }

        routes
    }
}

impl ForgeServiceExecutor {
    fn request_group(request: &ClientRpcRequest) -> Result<ForgeRequestGroup> {
        match request {
            ClientRpcRequest::ForgeCreateRepo { .. }
            | ClientRpcRequest::ForgeListRepos { .. }
            | ClientRpcRequest::ForgeGetRepo { .. } => Ok(ForgeRequestGroup::Repo),
            ClientRpcRequest::ForgeStoreBlob { .. } | ClientRpcRequest::ForgeGetBlob { .. } => {
                Ok(ForgeRequestGroup::Blob)
            }
            ClientRpcRequest::ForgeCreateTree { .. } | ClientRpcRequest::ForgeGetTree { .. } => {
                Ok(ForgeRequestGroup::Tree)
            }
            ClientRpcRequest::ForgeCommit { .. }
            | ClientRpcRequest::ForgeGetCommit { .. }
            | ClientRpcRequest::ForgeLog { .. } => Ok(ForgeRequestGroup::Commit),
            ClientRpcRequest::ForgeGetRef { .. }
            | ClientRpcRequest::ForgeSetRef { .. }
            | ClientRpcRequest::ForgeDeleteRef { .. }
            | ClientRpcRequest::ForgeCasRef { .. }
            | ClientRpcRequest::ForgeListBranches { .. }
            | ClientRpcRequest::ForgeListTags { .. } => Ok(ForgeRequestGroup::Ref),
            ClientRpcRequest::ForgeCreateIssue { .. }
            | ClientRpcRequest::ForgeListIssues { .. }
            | ClientRpcRequest::ForgeGetIssue { .. }
            | ClientRpcRequest::ForgeCommentIssue { .. }
            | ClientRpcRequest::ForgeCloseIssue { .. }
            | ClientRpcRequest::ForgeReopenIssue { .. } => Ok(ForgeRequestGroup::Issue),
            ClientRpcRequest::ForgeCreatePatch { .. }
            | ClientRpcRequest::ForgeListPatches { .. }
            | ClientRpcRequest::ForgeGetPatch { .. }
            | ClientRpcRequest::ForgeUpdatePatch { .. }
            | ClientRpcRequest::ForgeApprovePatch { .. }
            | ClientRpcRequest::ForgeMergePatch { .. }
            | ClientRpcRequest::ForgeCheckMerge { .. }
            | ClientRpcRequest::ForgeClosePatch { .. } => Ok(ForgeRequestGroup::Patch),
            ClientRpcRequest::ForgeCreateDiscussion { .. }
            | ClientRpcRequest::ForgeListDiscussions { .. }
            | ClientRpcRequest::ForgeGetDiscussion { .. }
            | ClientRpcRequest::ForgeReplyDiscussion { .. }
            | ClientRpcRequest::ForgeLockDiscussion { .. }
            | ClientRpcRequest::ForgeUnlockDiscussion { .. }
            | ClientRpcRequest::ForgeCloseDiscussion { .. }
            | ClientRpcRequest::ForgeReopenDiscussion { .. } => Ok(ForgeRequestGroup::Discussion),
            ClientRpcRequest::ForgeForkRepo { .. } => Ok(ForgeRequestGroup::Fork),
            ClientRpcRequest::ForgeSetMirror { .. }
            | ClientRpcRequest::ForgeDisableMirror { .. }
            | ClientRpcRequest::ForgeGetMirrorStatus { .. } => Ok(ForgeRequestGroup::Mirror),
            ClientRpcRequest::ForgeGetDelegateKey { .. }
            | ClientRpcRequest::GetFederationStatus
            | ClientRpcRequest::ListDiscoveredClusters
            | ClientRpcRequest::GetDiscoveredCluster { .. }
            | ClientRpcRequest::TrustCluster { .. }
            | ClientRpcRequest::UntrustCluster { .. }
            | ClientRpcRequest::FederateRepository { .. }
            | ClientRpcRequest::ListFederatedRepositories
            | ClientRpcRequest::FederationSyncPeer { .. }
            | ClientRpcRequest::FederationFetchRefs { .. }
            | ClientRpcRequest::FederationGitListRefs { .. }
            | ClientRpcRequest::FederationGitFetch { .. }
            | ClientRpcRequest::FederationPull { .. }
            | ClientRpcRequest::FederationPush { .. }
            | ClientRpcRequest::FederationBidiSync { .. }
            | ClientRpcRequest::ForgeFetchFederated { .. } => Ok(ForgeRequestGroup::Federation),
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgeFetchStart { .. }
            | ClientRpcRequest::GitBridgeFetchChunk { .. }
            | ClientRpcRequest::GitBridgeFetchComplete { .. }
            | ClientRpcRequest::GitBridgePush { .. }
            | ClientRpcRequest::GitBridgePushStart { .. }
            | ClientRpcRequest::GitBridgePushChunk { .. }
            | ClientRpcRequest::GitBridgePushComplete { .. }
            | ClientRpcRequest::GitBridgeProbeObjects { .. } => Ok(ForgeRequestGroup::GitBridge),
            ClientRpcRequest::NostrAuthChallenge { .. }
            | ClientRpcRequest::NostrAuthVerify { .. }
            | ClientRpcRequest::NostrGetProfile { .. } => Ok(ForgeRequestGroup::Nostr),
            ClientRpcRequest::ForgeDiffCommits { .. } | ClientRpcRequest::ForgeDiffRefs { .. } => {
                Ok(ForgeRequestGroup::Diff)
            }
            _ => Err(anyhow!("ForgeServiceExecutor received unhandled request")),
        }
    }

    // ========================================================================
    // Helper Methods for Git Operations
    // ========================================================================

    async fn handle_get_repo(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRepoResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {}", e))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        match self.forge_node.get_repo(&repo_id).await {
            Ok(identity) => {
                let active_routes = self.active_backend_routes().await;
                let repo_info = repo_info_from_identity(&identity, None, &active_routes);
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
        max_entries_hint: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeLogResultResponse;

        let max_entries = usize_from_u32(max_entries_hint.unwrap_or(20).min(100));
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

        let commits = self.walk_commit_history(start, max_entries).await?;
        let count = u32::try_from(commits.len()).unwrap_or(u32::MAX);
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
    // Diff Operations
    // ========================================================================

    async fn handle_diff_commits(
        &self,
        repo_id: String,
        old_commit: String,
        new_commit: String,
        should_include_content: bool,
        context_lines: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::DiffEntryResponse;
        use aspen_client_api::ForgeDiffResultResponse;
        use aspen_forge::git::DiffOptions;
        use aspen_forge::git::diff_commits;
        use aspen_forge::git::render_unified_diff;

        let _ = repo_id; // diff_commits operates on the shared git store
        let old_hash =
            blake3::Hash::from_hex(&old_commit).map_err(|e| anyhow::anyhow!("invalid old commit hash: {e}"))?;
        let new_hash =
            blake3::Hash::from_hex(&new_commit).map_err(|e| anyhow::anyhow!("invalid new commit hash: {e}"))?;

        let opts = DiffOptions {
            include_content: should_include_content,
        };
        match diff_commits(&self.forge_node.git, &old_hash, &new_hash, &opts).await {
            Ok(result) => {
                let unified_diff = if should_include_content {
                    let ctx = context_lines.unwrap_or(3);
                    Some(render_unified_diff(&result.entries, ctx))
                } else {
                    None
                };

                let entries: Vec<DiffEntryResponse> = result
                    .entries
                    .iter()
                    .map(|e| DiffEntryResponse {
                        path: e.path.clone(),
                        kind: match e.kind {
                            aspen_forge::git::DiffKind::Added => "added".to_string(),
                            aspen_forge::git::DiffKind::Removed => "removed".to_string(),
                            aspen_forge::git::DiffKind::Modified => "modified".to_string(),
                            aspen_forge::git::DiffKind::Renamed => "renamed".to_string(),
                        },
                        old_path: e.old_path.clone(),
                        old_mode: e.old_mode,
                        new_mode: e.new_mode,
                        old_hash: e.old_hash.map(hex::encode),
                        new_hash: e.new_hash.map(hex::encode),
                    })
                    .collect();

                Ok(ClientRpcResponse::ForgeDiffResult(ForgeDiffResultResponse {
                    entries,
                    truncated: result.truncated,
                    unified_diff,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::error("DIFF_FAILED", format!("{e}"))),
        }
    }

    async fn handle_diff_refs(
        &self,
        repo_id: String,
        old_ref: String,
        new_ref: String,
        should_include_content: bool,
        context_lines: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let rid = aspen_forge::identity::RepoId::from_hash(hash);

        let resolve_ref = |ref_name: &str| -> Vec<String> {
            vec![
                ref_name.to_string(),
                format!("heads/{ref_name}"),
                format!("refs/heads/{ref_name}"),
            ]
        };

        let mut old_commit = None;
        for candidate in resolve_ref(&old_ref) {
            if let Some(h) = self.forge_node.refs.get(&rid, &candidate).await? {
                old_commit = Some(h);
                break;
            }
        }
        let old_commit = match old_commit {
            Some(h) => h.to_hex().to_string(),
            None => {
                return Ok(ClientRpcResponse::error("REF_NOT_FOUND", format!("ref not found: {old_ref}")));
            }
        };

        let mut new_commit = None;
        for candidate in resolve_ref(&new_ref) {
            if let Some(h) = self.forge_node.refs.get(&rid, &candidate).await? {
                new_commit = Some(h);
                break;
            }
        }
        let new_commit = match new_commit {
            Some(h) => h.to_hex().to_string(),
            None => {
                return Ok(ClientRpcResponse::error("REF_NOT_FOUND", format!("ref not found: {new_ref}")));
            }
        };

        self.handle_diff_commits(repo_id, old_commit, new_commit, should_include_content, context_lines)
            .await
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
        strategy: Option<String>,
        message: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeMergeCheckResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;

        let merge_strategy = match strategy.as_deref() {
            Some(s) => s.parse::<aspen_forge::GitMergeStrategy>().map_err(|e| anyhow::anyhow!("{e}"))?,
            None => aspen_forge::GitMergeStrategy::MergeCommit,
        };

        match self.forge_node.merge_patch(&repo_id, &pid, merge_strategy, message).await {
            Ok(merge_commit) => Ok(ClientRpcResponse::ForgeMergeCheckResult(ForgeMergeCheckResultResponse {
                is_success: true,
                mergeable: true,
                available_strategies: vec![],
                conflicts: vec![],
                protection_satisfied: true,
                protection_reason: None,
                merge_commit: Some(merge_commit.to_hex().to_string()),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeMergeCheckResult(ForgeMergeCheckResultResponse {
                is_success: false,
                mergeable: false,
                available_strategies: vec![],
                conflicts: vec![],
                protection_satisfied: false,
                protection_reason: None,
                merge_commit: None,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_check_merge(&self, repo_id: String, patch_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeMergeCheckResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let pid = blake3::Hash::from_hex(&patch_id).map_err(|e| anyhow::anyhow!("invalid patch ID: {e}"))?;

        match self.forge_node.check_merge(&repo_id, &pid).await {
            Ok(result) => Ok(ClientRpcResponse::ForgeMergeCheckResult(ForgeMergeCheckResultResponse {
                is_success: true,
                mergeable: result.mergeable,
                available_strategies: result.available_strategies,
                conflicts: result.conflicts,
                protection_satisfied: result.protection_satisfied,
                protection_reason: result.protection_reason,
                merge_commit: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::ForgeMergeCheckResult(ForgeMergeCheckResultResponse {
                is_success: false,
                mergeable: false,
                available_strategies: vec![],
                conflicts: vec![],
                protection_satisfied: false,
                protection_reason: None,
                merge_commit: None,
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

    // ========================================================================
    // Discussion Handler Methods
    // ========================================================================

    async fn handle_create_discussion(
        &self,
        repo_id: String,
        title: String,
        body: String,
        labels: Vec<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        match self.forge_node.cobs.create_discussion(&repo_id, title, body, labels).await {
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

    async fn handle_list_discussions(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeDiscussionListResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);

        let ids = self.forge_node.cobs.list_discussions(&repo_id).await?;
        let mut discussions = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Ok(discussion) = self.forge_node.cobs.resolve_discussion(&repo_id, id).await {
                discussions.push(discussion_to_info(id, &discussion));
            }
        }
        Ok(ClientRpcResponse::ForgeDiscussionListResult(ForgeDiscussionListResultResponse { discussions }))
    }

    async fn handle_get_discussion(&self, repo_id: String, discussion_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeDiscussionResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let did = blake3::Hash::from_hex(&discussion_id).map_err(|e| anyhow::anyhow!("invalid discussion ID: {e}"))?;

        let discussion = self.forge_node.cobs.resolve_discussion(&repo_id, &did).await?;
        Ok(ClientRpcResponse::ForgeDiscussionResult(ForgeDiscussionResultResponse {
            discussion: discussion_to_info(&did, &discussion),
        }))
    }

    async fn handle_reply_discussion(
        &self,
        repo_id: String,
        discussion_id: String,
        body: String,
        parent_reply: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let did = blake3::Hash::from_hex(&discussion_id).map_err(|e| anyhow::anyhow!("invalid discussion ID: {e}"))?;

        let parent_reply_hash = if let Some(parent) = parent_reply {
            let ph = blake3::Hash::from_hex(&parent).map_err(|e| anyhow::anyhow!("invalid parent reply hash: {e}"))?;
            Some(*ph.as_bytes())
        } else {
            None
        };

        match self.forge_node.cobs.add_reply(&repo_id, &did, body, parent_reply_hash).await {
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

    async fn handle_lock_discussion(&self, repo_id: String, discussion_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let did = blake3::Hash::from_hex(&discussion_id).map_err(|e| anyhow::anyhow!("invalid discussion ID: {e}"))?;

        match self.forge_node.cobs.lock_discussion(&repo_id, &did).await {
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

    async fn handle_unlock_discussion(&self, repo_id: String, discussion_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let did = blake3::Hash::from_hex(&discussion_id).map_err(|e| anyhow::anyhow!("invalid discussion ID: {e}"))?;

        match self.forge_node.cobs.unlock_discussion(&repo_id, &did).await {
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

    async fn handle_close_discussion(
        &self,
        repo_id: String,
        discussion_id: String,
        reason: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let did = blake3::Hash::from_hex(&discussion_id).map_err(|e| anyhow::anyhow!("invalid discussion ID: {e}"))?;

        match self.forge_node.cobs.close_discussion(&repo_id, &did, reason).await {
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

    async fn handle_reopen_discussion(&self, repo_id: String, discussion_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(hash);
        let did = blake3::Hash::from_hex(&discussion_id).map_err(|e| anyhow::anyhow!("invalid discussion ID: {e}"))?;

        match self.forge_node.cobs.reopen_discussion(&repo_id, &did).await {
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
    // Fork Handler Methods
    // ========================================================================

    async fn handle_fork_repo(
        &self,
        upstream_repo_id: String,
        name: String,
        description: Option<String>,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeRepoResultResponse;

        let upstream_hash =
            blake3::Hash::from_hex(&upstream_repo_id).map_err(|e| anyhow::anyhow!("invalid upstream repo ID: {e}"))?;
        let upstream_repo_id = aspen_forge::identity::RepoId::from_hash(upstream_hash);

        // Use the current node's key as the single delegate with threshold 1
        let delegates = vec![self.forge_node.public_key()];
        let threshold = 1;

        match self.forge_node.fork_repo(&upstream_repo_id, &name, delegates, threshold).await {
            Ok(identity) => {
                let active_routes = self.active_backend_routes().await;
                let repo_info = repo_info_from_identity(&identity, description, &active_routes);
                Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                    is_success: true,
                    repo: Some(repo_info),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                is_success: false,
                repo: None,
                error: Some(format!("{e}")),
            })),
        }
    }

    async fn handle_set_mirror(
        &self,
        repo_id: String,
        upstream_repo_id: String,
        interval_secs: u32,
    ) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let repo_hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(repo_hash);
        let upstream_hash =
            blake3::Hash::from_hex(&upstream_repo_id).map_err(|e| anyhow::anyhow!("invalid upstream repo ID: {e}"))?;
        let upstream = aspen_forge::identity::RepoId::from_hash(upstream_hash);

        let config = aspen_forge::mirror::MirrorConfig::new(upstream, interval_secs);

        match self.forge_node.set_mirror_config(&repo_id, &config).await {
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

    async fn handle_disable_mirror(&self, repo_id: String) -> Result<ClientRpcResponse> {
        use aspen_client_api::ForgeOperationResultResponse;

        let repo_hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(repo_hash);

        match self.forge_node.delete_mirror_config(&repo_id).await {
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

    async fn handle_get_mirror_status(&self, repo_id: String) -> Result<ClientRpcResponse> {
        let repo_hash = blake3::Hash::from_hex(&repo_id).map_err(|e| anyhow::anyhow!("invalid repo ID: {e}"))?;
        let repo_id = aspen_forge::identity::RepoId::from_hash(repo_hash);

        match self.forge_node.get_mirror_status(&repo_id).await {
            Ok(Some(status)) => {
                let resp = aspen_client_api::ForgeMirrorStatusResponse {
                    upstream_repo_id: status.config.upstream_repo_id.to_hex(),
                    upstream_cluster: status.config.upstream_cluster.map(|k| k.to_string()),
                    interval_secs: status.config.interval_secs,
                    enabled: status.config.enabled,
                    last_sync_ms: status.config.last_sync_ms,
                    synced_refs_count: status.config.last_synced_refs_count,
                    is_due: status.is_due,
                };
                Ok(ClientRpcResponse::ForgeMirrorStatus(Some(resp)))
            }
            Ok(None) => Ok(ClientRpcResponse::ForgeMirrorStatus(None)),
            Err(_e) => Ok(ClientRpcResponse::ForgeMirrorStatus(None)),
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
        match Self::request_group(&request)? {
            ForgeRequestGroup::Repo => self.execute_repo_request(request).await,
            ForgeRequestGroup::Blob => self.execute_blob_request(request).await,
            ForgeRequestGroup::Tree => self.execute_tree_request(request).await,
            ForgeRequestGroup::Commit => self.execute_commit_request(request).await,
            ForgeRequestGroup::Ref => self.execute_ref_request(request).await,
            ForgeRequestGroup::Issue => self.execute_issue_request(request).await,
            ForgeRequestGroup::Patch => self.execute_patch_request(request).await,
            ForgeRequestGroup::Discussion => self.execute_discussion_request(request).await,
            ForgeRequestGroup::Fork => self.execute_fork_request(request).await,
            ForgeRequestGroup::Mirror => self.execute_mirror_request(request).await,
            ForgeRequestGroup::Federation => self.execute_federation_request(request).await,
            ForgeRequestGroup::GitBridge => self.execute_git_bridge_request(request).await,
            ForgeRequestGroup::Nostr => self.execute_nostr_request(request).await,
            ForgeRequestGroup::Diff => self.execute_diff_request(request).await,
        }
    }
}

impl ForgeServiceExecutor {
    async fn execute_repo_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeCreateRepo {
                name,
                description,
                default_branch: _,
            } => {
                use aspen_client_api::ForgeRepoResultResponse;

                let delegates = vec![self.forge_node.public_key()];
                match self.forge_node.create_repo(&name, delegates, 1).await {
                    Ok(identity) => {
                        let active_routes = self.active_backend_routes().await;
                        let repo_info = repo_info_from_identity(&identity, description, &active_routes);
                        Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                            is_success: true,
                            repo: Some(repo_info),
                            error: None,
                        }))
                    }
                    Err(error) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                        is_success: false,
                        repo: None,
                        error: Some(error.to_string()),
                    })),
                }
            }
            ClientRpcRequest::ForgeListRepos {
                limit: max_results_hint,
                offset: start_offset_hint,
            } => {
                use aspen_client_api::ForgeRepoListResultResponse;

                let max_results = usize_from_u32(max_results_hint.unwrap_or(100).min(1000));
                let start_index = usize_from_u32(start_offset_hint.unwrap_or(0));
                match self.forge_node.list_repos().await {
                    Ok(repos) => {
                        let active_routes = self.active_backend_routes().await;
                        let count = u32::try_from(repos.len()).unwrap_or(u32::MAX);
                        let repos = repos
                            .into_iter()
                            .skip(start_index)
                            .take(max_results)
                            .map(|identity| repo_info_from_identity(&identity, None, &active_routes))
                            .collect();
                        Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                            is_success: true,
                            repos,
                            count,
                            error: None,
                        }))
                    }
                    Err(error) => Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                        is_success: false,
                        repos: vec![],
                        count: 0,
                        error: Some(error.to_string()),
                    })),
                }
            }
            ClientRpcRequest::ForgeGetRepo { repo_id } => self.handle_get_repo(repo_id).await,
            _ => unexpected_request_kind("repo"),
        }
    }

    async fn execute_blob_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeStoreBlob { repo_id, content } => self.handle_store_blob(repo_id, content).await,
            ClientRpcRequest::ForgeGetBlob { hash } => self.handle_get_blob(hash).await,
            _ => unexpected_request_kind("blob"),
        }
    }

    async fn execute_tree_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeCreateTree {
                repo_id: _,
                entries_json,
            } => self.handle_create_tree(entries_json).await,
            ClientRpcRequest::ForgeGetTree { hash } => self.handle_get_tree(hash).await,
            _ => unexpected_request_kind("tree"),
        }
    }

    async fn execute_commit_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
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
                limit: max_entries_hint,
            } => self.handle_log(repo_id, ref_name, max_entries_hint).await,
            _ => unexpected_request_kind("commit"),
        }
    }

    async fn execute_ref_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
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
            _ => unexpected_request_kind("ref"),
        }
    }

    async fn execute_issue_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
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
            _ => unexpected_request_kind("issue"),
        }
    }

    async fn execute_patch_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
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
                strategy,
                message,
            } => self.handle_merge_patch(repo_id, patch_id, strategy, message).await,
            ClientRpcRequest::ForgeCheckMerge { repo_id, patch_id } => self.handle_check_merge(repo_id, patch_id).await,
            ClientRpcRequest::ForgeClosePatch {
                repo_id,
                patch_id,
                reason,
            } => self.handle_close_patch(repo_id, patch_id, reason).await,
            _ => unexpected_request_kind("patch"),
        }
    }

    async fn execute_discussion_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeCreateDiscussion {
                repo_id,
                title,
                body,
                labels,
            } => self.handle_create_discussion(repo_id, title, body, labels).await,
            ClientRpcRequest::ForgeListDiscussions {
                repo_id,
                state: _,
                limit: _,
            } => self.handle_list_discussions(repo_id).await,
            ClientRpcRequest::ForgeGetDiscussion { repo_id, discussion_id } => {
                self.handle_get_discussion(repo_id, discussion_id).await
            }
            ClientRpcRequest::ForgeReplyDiscussion {
                repo_id,
                discussion_id,
                body,
                parent_reply,
            } => self.handle_reply_discussion(repo_id, discussion_id, body, parent_reply).await,
            ClientRpcRequest::ForgeLockDiscussion { repo_id, discussion_id } => {
                self.handle_lock_discussion(repo_id, discussion_id).await
            }
            ClientRpcRequest::ForgeUnlockDiscussion { repo_id, discussion_id } => {
                self.handle_unlock_discussion(repo_id, discussion_id).await
            }
            ClientRpcRequest::ForgeCloseDiscussion {
                repo_id,
                discussion_id,
                reason,
            } => self.handle_close_discussion(repo_id, discussion_id, reason).await,
            ClientRpcRequest::ForgeReopenDiscussion { repo_id, discussion_id } => {
                self.handle_reopen_discussion(repo_id, discussion_id).await
            }
            _ => unexpected_request_kind("discussion"),
        }
    }

    async fn execute_fork_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeForkRepo {
                upstream_repo_id,
                name,
                description,
            } => self.handle_fork_repo(upstream_repo_id, name, description).await,
            _ => unexpected_request_kind("fork"),
        }
    }

    async fn execute_mirror_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeSetMirror {
                repo_id,
                upstream_repo_id,
                interval_secs,
            } => self.handle_set_mirror(repo_id, upstream_repo_id, interval_secs).await,
            ClientRpcRequest::ForgeDisableMirror { repo_id } => self.handle_disable_mirror(repo_id).await,
            ClientRpcRequest::ForgeGetMirrorStatus { repo_id } => self.handle_get_mirror_status(repo_id).await,
            _ => unexpected_request_kind("mirror"),
        }
    }

    async fn execute_federation_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        use crate::handler::handlers::federation::*;

        match request {
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
                handle_federate_repository(&self.forge_node, repo_id, mode, self.federation_cluster_identity.as_ref())
                    .await
            }
            ClientRpcRequest::ListFederatedRepositories => handle_list_federated_repositories(&self.forge_node).await,
            ClientRpcRequest::FederationSyncPeer {
                peer_node_id,
                peer_addr,
            } => {
                handle_federation_sync_peer(
                    &peer_node_id,
                    peer_addr.as_deref(),
                    self.federation_cluster_identity.as_ref(),
                    self.iroh_endpoint.as_ref(),
                    &self.forge_node,
                )
                .await
            }
            ClientRpcRequest::FederationFetchRefs {
                peer_node_id,
                peer_addr,
                fed_id,
            } => {
                handle_federation_fetch_refs(
                    FetchRefsTarget {
                        fed_id_str: &fed_id,
                        peer_node_id: &peer_node_id,
                    },
                    peer_addr.as_deref(),
                    self.federation_cluster_identity.as_ref(),
                    self.iroh_endpoint.as_ref(),
                    &self.forge_node,
                )
                .await
            }
            ClientRpcRequest::FederationPull {
                mirror_repo_id,
                peer_node_id,
                peer_addr,
                repo_id,
            } => {
                handle_federation_pull(
                    FederationPullRequest {
                        mirror_repo_id: mirror_repo_id.as_deref(),
                        peer_node_id: peer_node_id.as_deref(),
                        peer_addr: peer_addr.as_deref(),
                        repo_id: repo_id.as_deref(),
                    },
                    FederationRuntime {
                        cluster_identity: self.federation_cluster_identity.as_ref(),
                        iroh_endpoint: self.iroh_endpoint.as_ref(),
                        forge_node: &self.forge_node,
                    },
                )
                .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::FederationPush {
                peer_node_id,
                peer_addr,
                repo_id,
            } => {
                handle_federation_push(
                    &peer_node_id,
                    peer_addr.as_deref(),
                    &repo_id,
                    self.federation_cluster_identity.as_ref(),
                    self.iroh_endpoint.as_ref(),
                    &self.forge_node,
                )
                .await
            }
            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::FederationPush { .. } => Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "UNSUPPORTED".to_string(),
                message: "federation push requires git-bridge feature".to_string(),
            })),
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::FederationBidiSync {
                peer_node_id,
                peer_addr,
                repo_id,
                push_wins,
            } => {
                handle_federation_bidi_sync(
                    &peer_node_id,
                    peer_addr.as_deref(),
                    &repo_id,
                    push_wins,
                    self.federation_cluster_identity.as_ref(),
                    self.iroh_endpoint.as_ref(),
                    &self.forge_node,
                )
                .await
            }
            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::FederationBidiSync { .. } => Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "UNSUPPORTED".to_string(),
                message: "federation bidi sync requires git-bridge feature".to_string(),
            })),
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::FederationGitListRefs {
                origin_key,
                repo_id,
                origin_addr_hint,
            } => {
                crate::handler::handlers::federation_git::handle_federation_git_list_refs(
                    &origin_key,
                    &repo_id,
                    origin_addr_hint.as_deref(),
                    self.federation_cluster_identity.as_ref(),
                    self.iroh_endpoint.as_ref(),
                    &self.forge_node,
                )
                .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::FederationGitFetch {
                origin_key,
                repo_id,
                want,
                have,
                origin_addr_hint,
            } => {
                crate::handler::handlers::federation_git::handle_federation_git_fetch(
                    &origin_key,
                    &repo_id,
                    want,
                    have,
                    origin_addr_hint.as_deref(),
                    self.federation_cluster_identity.as_ref(),
                    self.iroh_endpoint.as_ref(),
                    &self.forge_node,
                )
                .await
            }
            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::FederationGitListRefs { .. } | ClientRpcRequest::FederationGitFetch { .. } => {
                Ok(ClientRpcResponse::Error(ErrorResponse {
                    code: "FEATURE_DISABLED".to_string(),
                    message: "git-bridge feature is required for federated git operations".to_string(),
                }))
            }
            ClientRpcRequest::ForgeFetchFederated {
                federated_id,
                remote_cluster,
            } => handle_fetch_federated(&self.forge_node, federated_id, remote_cluster).await,
            _ => unexpected_request_kind("federation"),
        }
    }

    async fn execute_git_bridge_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
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
                let response = crate::handler::handlers::git_bridge::handle_git_bridge_push(
                    &self.forge_node,
                    repo_id.clone(),
                    objects,
                    refs,
                )
                .await?;
                #[cfg(feature = "hooks")]
                if let ClientRpcResponse::GitBridgePush(ref push_response) = response
                    && push_response.is_success
                {
                    self.emit_push_hook(&repo_id, &push_response.ref_results);
                }
                Ok(response)
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
                let response = crate::handler::handlers::git_bridge::handle_git_bridge_push_complete(
                    &self.forge_node,
                    session_id,
                    content_hash,
                )
                .await?;
                #[cfg(feature = "hooks")]
                if let ClientRpcResponse::GitBridgePushComplete(ref push_response) = response
                    && push_response.is_success
                {
                    self.emit_push_hook("", &push_response.ref_results);
                }
                Ok(response)
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeProbeObjects { repo_id, sha1s } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_probe_objects(&self.forge_node, repo_id, sha1s)
                    .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetchStart { repo_id, want, have } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_fetch_start(
                    &self.forge_node,
                    repo_id,
                    want,
                    have,
                )
                .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetchChunk { session_id, chunk_id } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_fetch_chunk(
                    &self.forge_node,
                    session_id,
                    chunk_id,
                )
                .await
            }
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetchComplete { session_id } => {
                crate::handler::handlers::git_bridge::handle_git_bridge_fetch_complete(session_id).await
            }
            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgeFetchStart { .. }
            | ClientRpcRequest::GitBridgeFetchChunk { .. }
            | ClientRpcRequest::GitBridgeFetchComplete { .. }
            | ClientRpcRequest::GitBridgePush { .. }
            | ClientRpcRequest::GitBridgePushStart { .. }
            | ClientRpcRequest::GitBridgePushChunk { .. }
            | ClientRpcRequest::GitBridgePushComplete { .. }
            | ClientRpcRequest::GitBridgeProbeObjects { .. } => Ok(ClientRpcResponse::error(
                "GIT_BRIDGE_UNAVAILABLE",
                "Git bridge feature not enabled. Rebuild with --features git-bridge",
            )),
            _ => unexpected_request_kind("git bridge"),
        }
    }

    async fn execute_nostr_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::NostrAuthChallenge { npub_hex } => {
                match self.nostr_auth.create_challenge(&npub_hex).await {
                    Ok((challenge_id, challenge_bytes)) => Ok(ClientRpcResponse::NostrAuthChallengeResult {
                        challenge_id,
                        challenge_hex: hex::encode(challenge_bytes),
                    }),
                    Err(error) => Ok(ClientRpcResponse::error("AUTH_CHALLENGE_FAILED", format!("{error}"))),
                }
            }
            ClientRpcRequest::NostrAuthVerify {
                npub_hex,
                challenge_id,
                signature_hex,
            } => match self.nostr_auth.verify_challenge(&npub_hex, &challenge_id, &signature_hex).await {
                Ok(user_ctx) => match self.nostr_auth.issue_token(&user_ctx) {
                    Ok(token) => match postcard::to_stdvec(&token) {
                        Ok(token_bytes) => {
                            let token_b64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &token_bytes);
                            Ok(ClientRpcResponse::NostrAuthVerifyResult {
                                is_success: true,
                                token: Some(token_b64),
                                ed25519_public_key: Some(user_ctx.public_key.to_string()),
                                error: None,
                            })
                        }
                        Err(error) => Ok(ClientRpcResponse::NostrAuthVerifyResult {
                            is_success: false,
                            token: None,
                            ed25519_public_key: None,
                            error: Some(format!("token serialization failed: {error}")),
                        }),
                    },
                    Err(error) => Ok(ClientRpcResponse::NostrAuthVerifyResult {
                        is_success: false,
                        token: None,
                        ed25519_public_key: None,
                        error: Some(error.to_string()),
                    }),
                },
                Err(error) => Ok(ClientRpcResponse::NostrAuthVerifyResult {
                    is_success: false,
                    token: None,
                    ed25519_public_key: None,
                    error: Some(error.to_string()),
                }),
            },
            #[allow(clippy::collapsible_if)]
            ClientRpcRequest::NostrGetProfile { npub_hex } => {
                let prefix = format!("nostr:au:{}:", npub_hex);
                let scan = self
                    .forge_node
                    .kv()
                    .scan(aspen_core::ScanRequest {
                        prefix,
                        limit_results: Some(20),
                        continuation_token: None,
                    })
                    .await;

                let mut display_name = None;
                let mut nip05 = None;
                if let Ok(results) = scan {
                    for kv in &results.entries {
                        let event_id = kv.key.rsplit(':').next().unwrap_or("");
                        let event_key = format!("nostr:ev:{event_id}");
                        if let Ok(ev_result) = self
                            .forge_node
                            .kv()
                            .read(aspen_core::ReadRequest {
                                key: event_key,
                                consistency: aspen_core::ReadConsistency::Linearizable,
                            })
                            .await
                        {
                            let Some(ev_kv) = ev_result.kv else {
                                continue;
                            };
                            let Ok(event) = serde_json::from_str::<serde_json::Value>(&ev_kv.value) else {
                                continue;
                            };
                            if event.get("kind").and_then(|k| k.as_u64()).unwrap_or(0) != 0 {
                                continue;
                            }
                            if let Some(content_str) = event.get("content").and_then(|c| c.as_str())
                                && let Ok(profile) = serde_json::from_str::<serde_json::Value>(content_str)
                            {
                                display_name = profile
                                    .get("display_name")
                                    .or_else(|| profile.get("name"))
                                    .and_then(|value| value.as_str())
                                    .map(ToString::to_string);
                                nip05 = profile.get("nip05").and_then(|value| value.as_str()).map(ToString::to_string);
                            }
                            break;
                        }
                    }
                }
                Ok(ClientRpcResponse::NostrGetProfileResult { display_name, nip05 })
            }
            _ => unexpected_request_kind("nostr"),
        }
    }

    async fn execute_diff_request(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeDiffCommits {
                repo_id,
                old_commit,
                new_commit,
                include_content,
                context_lines,
            } => self.handle_diff_commits(repo_id, old_commit, new_commit, include_content, context_lines).await,
            ClientRpcRequest::ForgeDiffRefs {
                repo_id,
                old_ref,
                new_ref,
                include_content,
                context_lines,
            } => self.handle_diff_refs(repo_id, old_ref, new_ref, include_content, context_lines).await,
            _ => unexpected_request_kind("diff"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::net::SocketAddrV4;
    use std::sync::Arc;

    use aspen_blob::IrohBlobStore;
    use aspen_rpc_core::ServiceExecutor;
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;

    const TEST_NODE_ID: u64 = 7;
    const TEST_THRESHOLD: u32 = 1;
    const TEST_PROTOCOL_VERSION: u16 = 1;
    const TEST_MAX_SESSIONS: u32 = 2;
    const TEST_MAX_CHUNK_BYTES: u64 = 65_536;
    const TEST_MAX_IN_FLIGHT_BYTES: u64 = 131_072;
    const TEST_SESSION_TIMEOUT_MS: u64 = 30_000;

    fn test_plugin_manifest(identifier: &str, is_enabled: bool) -> aspen_plugin_api::PluginManifest {
        aspen_plugin_api::PluginManifest {
            name: "jj-plugin".to_string(),
            version: "1.0.0".to_string(),
            wasm_hash: "hash".to_string(),
            handles: vec![],
            protocols: vec![aspen_plugin_api::PluginProtocol {
                identifier: identifier.to_string(),
                version: TEST_PROTOCOL_VERSION,
                max_concurrent_sessions: TEST_MAX_SESSIONS,
                max_chunk_size_bytes: TEST_MAX_CHUNK_BYTES,
                max_in_flight_bytes: TEST_MAX_IN_FLIGHT_BYTES,
                session_timeout_ms: TEST_SESSION_TIMEOUT_MS,
            }],
            priority: ForgeServiceExecutor::PRIORITY,
            fuel_limit: None,
            memory_limit: None,
            enabled: is_enabled,
            app_id: None,
            execution_timeout_secs: None,
            kv_prefixes: vec![],
            permissions: aspen_plugin_api::PluginPermissions::default(),
            signature: None,
            description: None,
            author: None,
            tags: vec![],
            min_api_version: None,
            dependencies: vec![],
        }
    }

    async fn make_test_executor() -> ForgeServiceExecutor {
        let endpoint_secret = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(endpoint_secret)
            .bind_addr("127.0.0.1:0".parse::<SocketAddrV4>().expect("loopback bind address should parse"))
            .expect("loopback bind address should be valid")
            .bind()
            .await
            .expect("test endpoint should bind");
        let blob_dir = std::env::temp_dir().join(format!("aspen-forge-handler-executor-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&blob_dir).expect("blob directory should be created");
        let blob_store =
            Arc::new(IrohBlobStore::new(&blob_dir, endpoint.clone()).await.expect("blob store should initialize"));
        let kv: Arc<dyn aspen_core::KeyValueStore> = Arc::new(DeterministicKeyValueStore::new());
        let forge_secret = iroh::SecretKey::generate(&mut rand::rng());
        let forge_node = Arc::new(aspen_forge::ForgeNode::new(blob_store, kv, forge_secret));
        ForgeServiceExecutor::new(forge_node, ForgeServiceExecutorDeps {
            #[cfg(feature = "global-discovery")]
            content_discovery: None,
            #[cfg(feature = "global-discovery")]
            federation_discovery: None,
            federation_identity: None,
            federation_trust_manager: None,
            federation_cluster_identity: None,
            iroh_endpoint: Some(Arc::new(endpoint)),
            #[cfg(all(feature = "hooks", feature = "git-bridge"))]
            hook_service: None,
            node_id: TEST_NODE_ID,
        })
    }

    #[test]
    fn test_backend_route_helpers_advertise_git_and_active_jj() {
        let git_route = git_backend_route(TEST_NODE_ID);
        assert_eq!(git_route.backend, ForgeRepoBackend::Git);
        assert_eq!(git_route.node_id, Some(TEST_NODE_ID));
        assert_eq!(git_route.transport_id.as_deref(), Some(FORGE_GIT_BACKEND_TRANSPORT_ID));

        let manifest = test_plugin_manifest(JJ_NATIVE_FORGE_ALPN_STR, true);
        let jj_route =
            jj_backend_route_from_manifest(TEST_NODE_ID, &manifest).expect("JJ plugin should advertise route");
        assert_eq!(jj_route.backend, ForgeRepoBackend::Jj);
        assert_eq!(jj_route.node_id, Some(TEST_NODE_ID));
        assert_eq!(jj_route.transport_id.as_deref(), Some(JJ_NATIVE_FORGE_ALPN_STR));
        assert_eq!(jj_route.transport_version, Some(TEST_PROTOCOL_VERSION));
    }

    #[test]
    fn test_repo_info_dual_backend_uses_only_active_routes() {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        let identity = aspen_forge::identity::RepoIdentity::new("repo", vec![secret.public()], TEST_THRESHOLD)
            .expect("repo identity builds");
        let active_routes = vec![
            git_backend_route(TEST_NODE_ID),
            jj_backend_route_from_manifest(TEST_NODE_ID, &test_plugin_manifest(JJ_NATIVE_FORGE_ALPN_STR, true))
                .expect("JJ route"),
        ];

        let info = repo_info_from_identity_with_backends(
            &identity,
            None,
            vec![ForgeRepoBackend::Git, ForgeRepoBackend::Jj],
            &active_routes,
        );

        assert_eq!(info.backends, vec![ForgeRepoBackend::Git, ForgeRepoBackend::Jj]);
        assert_eq!(info.backend_routes.len(), 2);
    }

    #[test]
    fn test_repo_info_omits_inactive_backend_routes() {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        let identity = aspen_forge::identity::RepoIdentity::new("repo", vec![secret.public()], TEST_THRESHOLD)
            .expect("repo identity builds");
        let active_routes = vec![git_backend_route(TEST_NODE_ID)];

        let info = repo_info_from_identity_with_backends(
            &identity,
            None,
            vec![ForgeRepoBackend::Git, ForgeRepoBackend::Jj],
            &active_routes,
        );

        assert_eq!(info.backend_routes.len(), 1);
        assert_eq!(info.backend_routes[0].backend, ForgeRepoBackend::Git);
    }

    #[test]
    fn test_backend_routes_filter_by_repo_backend() {
        let routes = vec![
            git_backend_route(TEST_NODE_ID),
            jj_backend_route_from_manifest(TEST_NODE_ID, &test_plugin_manifest(JJ_NATIVE_FORGE_ALPN_STR, true))
                .expect("JJ route"),
        ];
        let git_routes = routes_for_backends(&[ForgeRepoBackend::Git], &routes);

        assert_eq!(git_routes.len(), 1);
        assert_eq!(git_routes[0].backend, ForgeRepoBackend::Git);
    }

    #[tokio::test]
    async fn test_active_backend_routes_reads_enabled_jj_plugin() {
        let executor = make_test_executor().await;
        let manifest = test_plugin_manifest(JJ_NATIVE_FORGE_ALPN_STR, true);
        let manifest_json = serde_json::to_string(&manifest).expect("manifest serializes");
        executor
            .forge_node
            .kv()
            .write(aspen_core::WriteRequest::set(
                format!("{}{}", aspen_plugin_api::PLUGIN_KV_PREFIX, manifest.name),
                manifest_json,
            ))
            .await
            .expect("plugin manifest writes to KV");

        let routes = executor.active_backend_routes().await;

        assert!(routes.iter().any(|route| route.backend == ForgeRepoBackend::Git));
        assert!(routes.iter().any(|route| route.backend == ForgeRepoBackend::Jj));
    }

    #[tokio::test]
    async fn test_active_backend_routes_omits_disabled_jj_plugin() {
        let executor = make_test_executor().await;
        let manifest = test_plugin_manifest(JJ_NATIVE_FORGE_ALPN_STR, false);
        let manifest_json = serde_json::to_string(&manifest).expect("manifest serializes");
        executor
            .forge_node
            .kv()
            .write(aspen_core::WriteRequest::set(
                format!("{}{}", aspen_plugin_api::PLUGIN_KV_PREFIX, manifest.name),
                manifest_json,
            ))
            .await
            .expect("plugin manifest writes to KV");

        let routes = executor.active_backend_routes().await;

        assert!(routes.iter().any(|route| route.backend == ForgeRepoBackend::Git));
        assert!(!routes.iter().any(|route| route.backend == ForgeRepoBackend::Jj));
    }

    #[tokio::test]
    async fn test_active_backend_routes_keep_stable_jj_identifier_after_manifest_rewrite() {
        let executor = make_test_executor().await;
        let mut manifest = test_plugin_manifest(JJ_NATIVE_FORGE_ALPN_STR, true);
        let key = format!("{}{}", aspen_plugin_api::PLUGIN_KV_PREFIX, manifest.name);
        let first_json = serde_json::to_string(&manifest).expect("manifest serializes");
        executor
            .forge_node
            .kv()
            .write(aspen_core::WriteRequest::set(key.clone(), first_json))
            .await
            .expect("plugin manifest writes to KV");
        let first_route = executor
            .active_backend_routes()
            .await
            .into_iter()
            .find(|route| route.backend == ForgeRepoBackend::Jj)
            .expect("JJ route is active");

        manifest.wasm_hash = "replacement-hash".to_string();
        let second_json = serde_json::to_string(&manifest).expect("manifest serializes");
        executor
            .forge_node
            .kv()
            .write(aspen_core::WriteRequest::set(key, second_json))
            .await
            .expect("plugin manifest rewrites to KV");
        let second_route = executor
            .active_backend_routes()
            .await
            .into_iter()
            .find(|route| route.backend == ForgeRepoBackend::Jj)
            .expect("JJ route remains active");

        assert_eq!(first_route.transport_id, second_route.transport_id);
        assert_eq!(first_route.transport_version, second_route.transport_version);
    }

    #[test]
    fn test_request_group_routes_repo_and_git_bridge_requests() {
        assert_eq!(
            ForgeServiceExecutor::request_group(&ClientRpcRequest::ForgeCreateRepo {
                name: "repo".into(),
                description: None,
                default_branch: None,
            })
            .expect("repo request should route"),
            ForgeRequestGroup::Repo
        );
        assert_eq!(
            ForgeServiceExecutor::request_group(&ClientRpcRequest::ForgeGetRef {
                repo_id: "abcd".into(),
                ref_name: "main".into(),
            })
            .expect("ref request should route"),
            ForgeRequestGroup::Ref
        );
        assert_eq!(
            ForgeServiceExecutor::request_group(&ClientRpcRequest::GitBridgeFetch {
                repo_id: "abcd".into(),
                want: vec![],
                have: vec![],
            })
            .expect("git bridge request should route"),
            ForgeRequestGroup::GitBridge
        );
    }

    #[test]
    fn test_request_group_routes_federation_nostr_and_diff_requests() {
        assert_eq!(
            ForgeServiceExecutor::request_group(&ClientRpcRequest::FederationPull {
                mirror_repo_id: None,
                peer_node_id: None,
                peer_addr: None,
                repo_id: Some("repo".into()),
            })
            .expect("federation request should route"),
            ForgeRequestGroup::Federation
        );
        assert_eq!(
            ForgeServiceExecutor::request_group(&ClientRpcRequest::NostrAuthChallenge {
                npub_hex: "npub".into(),
            })
            .expect("nostr request should route"),
            ForgeRequestGroup::Nostr
        );
        assert_eq!(
            ForgeServiceExecutor::request_group(&ClientRpcRequest::ForgeDiffRefs {
                repo_id: "repo".into(),
                old_ref: "main".into(),
                new_ref: "feature".into(),
                include_content: false,
                context_lines: None,
            })
            .expect("diff request should route"),
            ForgeRequestGroup::Diff
        );
    }

    #[tokio::test]
    async fn test_execute_create_repo_then_list_repos() {
        let executor = make_test_executor().await;
        let create_result = executor
            .execute(ClientRpcRequest::ForgeCreateRepo {
                name: "repo".into(),
                description: Some("demo".into()),
                default_branch: None,
            })
            .await
            .expect("create repo should succeed");
        match create_result {
            ClientRpcResponse::ForgeRepoResult(response) => {
                assert!(response.is_success);
                let repo = response.repo.expect("repo result should include created repo");
                assert_eq!(repo.name, "repo");
                assert_eq!(repo.default_branch, "main");
                assert_eq!(repo.backends, vec![ForgeRepoBackend::Git]);
                assert_eq!(repo.backend_routes.len(), 1);
                assert_eq!(repo.backend_routes[0].backend, ForgeRepoBackend::Git);
            }
            other => panic!("expected ForgeRepoResult, got {other:?}"),
        }

        let list_result = executor
            .execute(ClientRpcRequest::ForgeListRepos {
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .expect("list repos should succeed");
        match list_result {
            ClientRpcResponse::ForgeRepoListResult(response) => {
                assert!(response.is_success);
                assert_eq!(response.count, 1);
                assert_eq!(response.repos.len(), 1);
                assert_eq!(response.repos[0].name, "repo");
                assert_eq!(response.repos[0].backend_routes.len(), 1);
            }
            other => panic!("expected ForgeRepoListResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_execute_routes_nostr_requests() {
        let executor = make_test_executor().await;
        let response = executor
            .execute(ClientRpcRequest::NostrAuthChallenge {
                npub_hex: "npub-test".into(),
            })
            .await
            .expect("nostr auth challenge should succeed");
        match response {
            ClientRpcResponse::NostrAuthChallengeResult {
                challenge_id,
                challenge_hex,
            } => {
                assert_eq!(challenge_id.len(), 32);
                assert_eq!(challenge_hex.len(), 64);
            }
            other => panic!("expected NostrAuthChallengeResult, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_execute_rejects_unhandled_requests_with_error() {
        let executor = make_test_executor().await;
        let error = executor
            .execute(ClientRpcRequest::Ping)
            .await
            .expect_err("unhandled request should return an error");
        assert!(error.to_string().contains("ForgeServiceExecutor received unhandled request"));
    }

    #[test]
    fn test_execute_dispatch_metadata_is_stable() {
        assert_eq!(ForgeServiceExecutor::SERVICE_NAME, "forge");
        assert_eq!(ForgeServiceExecutor::PRIORITY, 540);
        assert_eq!(ForgeServiceExecutor::APP_ID, Some("forge"));
        assert!(ForgeServiceExecutor::HANDLES.contains(&"ForgeCreateRepo"));
        assert!(ForgeServiceExecutor::HANDLES.contains(&"GitBridgePush"));
        assert!(ForgeServiceExecutor::HANDLES.contains(&"NostrAuthVerify"));
    }
}
