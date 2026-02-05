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

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::GitBridgeObject;
use aspen_client_api::GitBridgeRefUpdate;
use aspen_forge::constants::MAX_CONCURRENT_PUSH_SESSIONS;
use aspen_forge::constants::PUSH_SESSION_TIMEOUT;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

// =============================================================================
// Chunked Push Session State
// =============================================================================

/// State for an active chunked push session.
#[cfg(feature = "git-bridge")]
struct ChunkedPushSession {
    /// Repository ID for this push.
    repo_id: String,
    /// Ref updates to apply after all objects are received.
    refs: Vec<GitBridgeRefUpdate>,
    /// Expected total number of objects (stored for future progress tracking).
    #[allow(dead_code)]
    total_objects: u64,
    /// Expected total size in bytes (stored for future progress tracking).
    #[allow(dead_code)]
    total_size_bytes: u64,
    /// Set of chunk IDs that have been received.
    chunks_received: HashSet<u64>,
    /// Total number of chunks (set on first chunk).
    total_chunks: Option<u64>,
    /// Accumulated objects from all chunks.
    objects: Vec<GitBridgeObject>,
    /// Session creation time for timeout tracking.
    created_at: std::time::Instant,
}

/// Global session store for chunked push operations.
///
/// Tiger Style: Bounded to MAX_CONCURRENT_PUSH_SESSIONS with timeout-based cleanup.
#[cfg(feature = "git-bridge")]
static PUSH_SESSIONS: OnceLock<Arc<Mutex<HashMap<String, ChunkedPushSession>>>> = OnceLock::new();

/// Get the global session store, initializing if needed.
#[cfg(feature = "git-bridge")]
fn get_session_store() -> &'static Arc<Mutex<HashMap<String, ChunkedPushSession>>> {
    PUSH_SESSIONS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

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
            ClientRpcRequest::GitBridgeListRefs { repo_id } => handle_git_bridge_list_refs(forge_node, repo_id).await,

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetch { repo_id, want, have } => {
                handle_git_bridge_fetch(forge_node, repo_id, want, have).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePush { repo_id, objects, refs } => {
                handle_git_bridge_push(forge_node, repo_id, objects, refs).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushStart {
                repo_id,
                total_objects,
                total_size_bytes,
                refs,
                metadata,
            } => {
                handle_git_bridge_push_start(forge_node, repo_id, total_objects, total_size_bytes, refs, metadata).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushChunk {
                session_id,
                chunk_id,
                total_chunks,
                objects,
                chunk_hash,
            } => {
                handle_git_bridge_push_chunk(forge_node, session_id, chunk_id, total_chunks, objects, chunk_hash).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushComplete {
                session_id,
                content_hash,
            } => handle_git_bridge_push_complete(forge_node, session_id, content_hash).await,

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

// Type alias for the concrete ForgeNode type used in the context
type ForgeNodeRef = std::sync::Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;

// ============================================================================
// Repository Operations
// ============================================================================

async fn handle_create_repo(
    forge_node: &ForgeNodeRef,
    _ctx: &ClientProtocolContext,
    name: String,
    description: Option<String>,
    default_branch: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRepoInfo;
    use aspen_client_api::ForgeRepoResultResponse;

    // Use node's public key as the delegate
    let delegates = vec![forge_node.public_key()];
    let threshold = 1;

    match forge_node.create_repo(&name, delegates.clone(), threshold).await {
        Ok(identity) => {
            // Initialize with empty commit if requested
            let _default_branch = default_branch.as_deref().unwrap_or("main");

            Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                success: true,
                repo: Some(ForgeRepoInfo {
                    id: identity.repo_id().to_hex(),
                    name: identity.name.clone(),
                    description,
                    default_branch: identity.default_branch.clone(),
                    delegates: delegates.iter().map(|k| k.to_string()).collect(),
                    threshold,
                    created_at_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                }),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            success: false,
            repo: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_repo(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRepoInfo;
    use aspen_client_api::ForgeRepoResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                success: false,
                repo: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.get_repo(&repo_id).await {
        Ok(identity) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            success: true,
            repo: Some(ForgeRepoInfo {
                id: identity.repo_id().to_hex(),
                name: identity.name.clone(),
                description: None,
                default_branch: identity.default_branch.clone(),
                delegates: identity.delegates.iter().map(|k| k.to_string()).collect(),
                threshold: identity.threshold,
                created_at_ms: 0, // Not stored in identity
            }),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            success: false,
            repo: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_list_repos(
    ctx: &ClientProtocolContext,
    limit: Option<u32>,
    offset: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRepoInfo;
    use aspen_client_api::ForgeRepoListResultResponse;
    use aspen_core::ScanRequest;
    use aspen_forge::constants::KV_PREFIX_REPOS;
    use aspen_forge::identity::RepoIdentity;
    use aspen_forge::types::SignedObject;

    let limit = limit.unwrap_or(100).min(1000);
    let offset = offset.unwrap_or(0);

    // Scan for all repos
    let scan_result = ctx
        .kv_store
        .scan(ScanRequest {
            prefix: KV_PREFIX_REPOS.to_string(),
            limit: Some(limit + offset),
            continuation_token: None,
        })
        .await;

    match scan_result {
        Ok(result) => {
            let repos: Vec<ForgeRepoInfo> = result
                .entries
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .filter_map(|entry| {
                    // Parse repo identity from the stored value
                    if entry.key.ends_with(":identity") {
                        let repo_id = entry.key.strip_prefix(KV_PREFIX_REPOS)?.strip_suffix(":identity")?;
                        // Decode the identity: Base64 -> bytes -> SignedObject -> RepoIdentity
                        let bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &entry.value).ok()?;
                        let signed: SignedObject<RepoIdentity> = SignedObject::from_bytes(&bytes).ok()?;
                        let identity = signed.payload;
                        Some(ForgeRepoInfo {
                            id: repo_id.to_string(),
                            name: identity.name,
                            description: identity.description,
                            default_branch: identity.default_branch,
                            delegates: identity.delegates.iter().map(|k| k.to_string()).collect(),
                            threshold: identity.threshold,
                            created_at_ms: identity.created_at_ms,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let count = repos.len() as u32;
            Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
                success: true,
                repos,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRepoListResult(ForgeRepoListResultResponse {
            success: false,
            repos: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Git Object Operations
// ============================================================================

async fn handle_store_blob(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    content: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeBlobResultResponse;

    let size = content.len() as u64;
    match forge_node.git.store_blob(content).await {
        Ok(hash) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            success: true,
            hash: Some(hash.to_hex().to_string()),
            content: None,
            size: Some(size),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            success: false,
            hash: None,
            content: None,
            size: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_blob(forge_node: &ForgeNodeRef, hash: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeBlobResultResponse;

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                success: false,
                hash: None,
                content: None,
                size: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    match forge_node.git.get_blob(&hash).await {
        Ok(content) => {
            let size = content.len() as u64;
            Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
                success: true,
                hash: Some(hash.to_hex().to_string()),
                content: Some(content),
                size: Some(size),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeBlobResult(ForgeBlobResultResponse {
            success: false,
            hash: None,
            content: None,
            size: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_create_tree(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    entries_json: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeTreeEntry as RpcTreeEntry;
    use aspen_client_api::ForgeTreeResultResponse;
    use aspen_forge::TreeEntry;

    // Parse entries from JSON
    let parsed: Vec<RpcTreeEntry> = match serde_json::from_str(&entries_json) {
        Ok(e) => e,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                success: false,
                hash: None,
                entries: None,
                error: Some(format!("Invalid entries JSON: {}", e)),
            }));
        }
    };

    // Convert to internal TreeEntry format
    let entries: Result<Vec<TreeEntry>, blake3::HexError> = parsed
        .iter()
        .map(|e| {
            let hash = blake3::Hash::from_hex(&e.hash)?;
            Ok(TreeEntry {
                mode: e.mode,
                name: e.name.clone(),
                hash: *hash.as_bytes(),
            })
        })
        .collect();

    let entries = match entries {
        Ok(e) => e,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                success: false,
                hash: None,
                entries: None,
                error: Some(format!("Invalid entry hash: {}", e)),
            }));
        }
    };

    match forge_node.git.create_tree(&entries).await {
        Ok(hash) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            success: true,
            hash: Some(hash.to_hex().to_string()),
            entries: Some(parsed),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            success: false,
            hash: None,
            entries: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_tree(forge_node: &ForgeNodeRef, hash: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeTreeEntry as RpcTreeEntry;
    use aspen_client_api::ForgeTreeResultResponse;

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                success: false,
                hash: None,
                entries: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    match forge_node.git.get_tree(&hash).await {
        Ok(tree) => {
            let entries: Vec<RpcTreeEntry> = tree
                .entries
                .iter()
                .map(|e| RpcTreeEntry {
                    mode: e.mode,
                    name: e.name.clone(),
                    hash: blake3::Hash::from_bytes(e.hash).to_hex().to_string(),
                })
                .collect();

            Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
                success: true,
                hash: Some(hash.to_hex().to_string()),
                entries: Some(entries),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(ForgeTreeResultResponse {
            success: false,
            hash: None,
            entries: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_commit(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    tree: String,
    parents: Vec<String>,
    message: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommitInfo;
    use aspen_client_api::ForgeCommitResultResponse;

    let tree_hash = match blake3::Hash::from_hex(&tree) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                success: false,
                commit: None,
                error: Some(format!("Invalid tree hash: {}", e)),
            }));
        }
    };

    let parent_hashes: Result<Vec<blake3::Hash>, _> = parents.iter().map(|p| blake3::Hash::from_hex(p)).collect();
    let parent_hashes = match parent_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                success: false,
                commit: None,
                error: Some(format!("Invalid parent hash: {}", e)),
            }));
        }
    };

    match forge_node.git.commit(tree_hash, parent_hashes.clone(), &message).await {
        Ok(hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                success: true,
                commit: Some(ForgeCommitInfo {
                    hash: hash.to_hex().to_string(),
                    tree,
                    parents,
                    author_name: "anonymous".to_string(),
                    author_email: None,
                    author_key: Some(forge_node.public_key().to_string()),
                    message,
                    timestamp_ms: now,
                }),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            success: false,
            commit: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_commit(forge_node: &ForgeNodeRef, hash: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommitInfo;
    use aspen_client_api::ForgeCommitResultResponse;

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
                success: false,
                commit: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    match forge_node.git.get_commit(&hash).await {
        Ok(commit) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            success: true,
            commit: Some(ForgeCommitInfo {
                hash: hash.to_hex().to_string(),
                tree: blake3::Hash::from_bytes(commit.tree).to_hex().to_string(),
                parents: commit.parents.iter().map(|p| blake3::Hash::from_bytes(*p).to_hex().to_string()).collect(),
                author_name: commit.author.name.clone(),
                author_email: Some(commit.author.email.clone()),
                author_key: commit.author.public_key.map(|k| hex::encode(k.as_bytes())),
                message: commit.message.clone(),
                timestamp_ms: commit.author.timestamp_ms,
            }),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(ForgeCommitResultResponse {
            success: false,
            commit: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_log(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommitInfo;
    use aspen_client_api::ForgeLogResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                success: false,
                commits: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let ref_name = ref_name.unwrap_or_else(|| "heads/main".to_string());
    let limit = limit.unwrap_or(50).min(1000);

    // Get the ref
    let start_hash = match forge_node.refs.get(&repo_id, &ref_name).await {
        Ok(Some(h)) => h,
        Ok(None) => {
            return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                success: true,
                commits: vec![],
                count: 0,
                error: None,
            }));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
                success: false,
                commits: vec![],
                count: 0,
                error: Some(e.to_string()),
            }));
        }
    };

    // Walk commits
    let mut commits = Vec::new();
    let mut queue = vec![start_hash];
    let mut seen = std::collections::HashSet::new();

    while let Some(hash) = queue.pop() {
        if commits.len() >= limit as usize {
            break;
        }
        if seen.contains(&hash) {
            continue;
        }
        seen.insert(hash);

        match forge_node.git.get_commit(&hash).await {
            Ok(commit) => {
                commits.push(ForgeCommitInfo {
                    hash: hash.to_hex().to_string(),
                    tree: blake3::Hash::from_bytes(commit.tree).to_hex().to_string(),
                    parents: commit.parents.iter().map(|p| blake3::Hash::from_bytes(*p).to_hex().to_string()).collect(),
                    author_name: commit.author.name.clone(),
                    author_email: Some(commit.author.email.clone()),
                    author_key: commit.author.public_key.map(|k| hex::encode(k.as_bytes())),
                    message: commit.message.clone(),
                    timestamp_ms: commit.author.timestamp_ms,
                });

                // Add parents to queue
                for parent in &commit.parents {
                    queue.push(blake3::Hash::from_bytes(*parent));
                }
            }
            Err(_) => break,
        }
    }

    let count = commits.len() as u32;
    Ok(ClientRpcResponse::ForgeLogResult(ForgeLogResultResponse {
        success: true,
        commits,
        count,
        error: None,
    }))
}

// ============================================================================
// Ref Operations
// ============================================================================

async fn handle_get_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.refs.get(&repo_id, &ref_name).await {
        Ok(Some(hash)) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: hash.to_hex().to_string(),
            }),
            previous_hash: None,
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: false,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_set_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid hash: {}", e)),
            }));
        }
    };

    // Get previous value for response
    let previous = forge_node.refs.get(&repo_id, &ref_name).await.ok().flatten();

    match forge_node.refs.set(&repo_id, &ref_name, hash).await {
        Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: hash.to_hex().to_string(),
            }),
            previous_hash: previous.map(|h| h.to_hex().to_string()),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: false,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_delete_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Get previous value
    let previous = forge_node.refs.get(&repo_id, &ref_name).await.ok().flatten();

    match forge_node.refs.delete(&repo_id, &ref_name).await {
        Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: previous.is_some(),
            ref_info: None,
            previous_hash: previous.map(|h| h.to_hex().to_string()),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: false,
            found: false,
            ref_info: None,
            previous_hash: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_cas_ref(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: String,
    expected: Option<String>,
    new_hash: String,
    signer: Option<String>,
    signature: Option<String>,
    timestamp_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefResultResponse;
    use aspen_forge::identity::RepoId;
    use aspen_forge::refs::DelegateVerifier;
    use aspen_forge::refs::SignedRefUpdate;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let expected_hash = match &expected {
        Some(h) => match blake3::Hash::from_hex(h) {
            Ok(hash) => Some(hash),
            Err(e) => {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some(format!("Invalid expected hash: {}", e)),
                }));
            }
        },
        None => None,
    };

    let new_hash = match blake3::Hash::from_hex(&new_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(format!("Invalid new hash: {}", e)),
            }));
        }
    };

    // For canonical refs (tags, default branch), verify signature if provided
    // If signature is provided, verify it; if not provided for canonical refs,
    // currently we allow it for backwards compatibility during migration
    if let (Some(signer_hex), Some(sig_hex), Some(ts)) = (&signer, &signature, timestamp_ms) {
        // Get repository identity for delegate verification
        let identity = match forge_node.get_repo(&repo_id).await {
            Ok(id) => id,
            Err(e) => {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some(format!("Failed to get repository: {}", e)),
                }));
            }
        };

        // Check if this is a canonical ref that requires delegate authorization
        if DelegateVerifier::is_canonical_ref(&ref_name, &identity.default_branch) {
            // Parse signer public key
            let signer_bytes = match hex::decode(signer_hex) {
                Ok(b) if b.len() == 32 => b,
                _ => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some("Invalid signer key: must be 32 bytes hex".to_string()),
                    }));
                }
            };
            let mut signer_arr = [0u8; 32];
            signer_arr.copy_from_slice(&signer_bytes);
            let signer_key = match iroh::PublicKey::from_bytes(&signer_arr) {
                Ok(k) => k,
                Err(e) => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some(format!("Invalid signer key: {}", e)),
                    }));
                }
            };

            // Parse signature
            let sig_bytes = match hex::decode(sig_hex) {
                Ok(b) if b.len() == 64 => b,
                _ => {
                    return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                        success: false,
                        found: false,
                        ref_info: None,
                        previous_hash: None,
                        error: Some("Invalid signature: must be 64 bytes hex".to_string()),
                    }));
                }
            };
            let mut sig_arr = [0u8; 64];
            sig_arr.copy_from_slice(&sig_bytes);
            let signature = iroh::Signature::from_bytes(&sig_arr);

            // Build SignedRefUpdate for verification
            let signed_update = SignedRefUpdate {
                repo_id,
                ref_name: ref_name.clone(),
                new_hash: *new_hash.as_bytes(),
                old_hash: expected_hash.map(|h| *h.as_bytes()),
                signer: signer_key,
                signature,
                timestamp_ms: ts,
            };

            // Verify the signature and delegate authorization
            if let Err(e) = DelegateVerifier::verify_update(&signed_update, &identity) {
                return Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                    success: false,
                    found: false,
                    ref_info: None,
                    previous_hash: None,
                    error: Some(format!("Authorization failed: {}", e)),
                }));
            }
        }
    }

    match forge_node.refs.compare_and_set(&repo_id, &ref_name, expected_hash, new_hash).await {
        Ok(()) => Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
            success: true,
            found: true,
            ref_info: Some(ForgeRefInfo {
                name: ref_name,
                hash: new_hash.to_hex().to_string(),
            }),
            previous_hash: expected,
            error: None,
        })),
        Err(e) => {
            // CAS failure or other error
            Ok(ClientRpcResponse::ForgeRefResult(ForgeRefResultResponse {
                success: false,
                found: false,
                ref_info: None,
                previous_hash: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn handle_list_branches(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: false,
                refs: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.refs.list_branches(&repo_id).await {
        Ok(branches) => {
            let refs: Vec<ForgeRefInfo> = branches
                .iter()
                .map(|(name, hash)| ForgeRefInfo {
                    name: name.clone(),
                    hash: hash.to_hex().to_string(),
                })
                .collect();
            let count = refs.len() as u32;
            Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: true,
                refs,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
            success: false,
            refs: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_list_tags(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRefInfo;
    use aspen_client_api::ForgeRefListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: false,
                refs: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.refs.list_tags(&repo_id).await {
        Ok(tags) => {
            let refs: Vec<ForgeRefInfo> = tags
                .iter()
                .map(|(name, hash)| ForgeRefInfo {
                    name: name.clone(),
                    hash: hash.to_hex().to_string(),
                })
                .collect();
            let count = refs.len() as u32;
            Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
                success: true,
                refs,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRefListResult(ForgeRefListResultResponse {
            success: false,
            refs: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Issue Operations
// ============================================================================

async fn handle_create_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    title: String,
    body: String,
    labels: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeIssueInfo;
    use aspen_client_api::ForgeIssueResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.create_issue(&repo_id, &title, &body, labels.clone()).await {
        Ok(issue_hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: true,
                issue: Some(ForgeIssueInfo {
                    id: issue_hash.to_hex().to_string(),
                    title,
                    body,
                    state: "open".to_string(),
                    labels,
                    comment_count: 0,
                    assignees: vec![],
                    created_at_ms: now,
                    updated_at_ms: now,
                }),
                comments: None,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            success: false,
            issue: None,
            comments: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_list_issues(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    state: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeIssueInfo;
    use aspen_client_api::ForgeIssueListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                success: false,
                issues: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let limit = limit.unwrap_or(50).min(1000) as usize;

    match forge_node.cobs.list_issues(&repo_id).await {
        Ok(issue_ids) => {
            let mut issues = Vec::new();
            // Filter first, then limit - fixes bug where closed issues wouldn't appear
            // if they weren't in the first N issues before filtering
            for issue_id in issue_ids.iter() {
                if let Ok(issue) = forge_node.cobs.resolve_issue(&repo_id, issue_id).await {
                    let issue_state = if issue.state.is_open() { "open" } else { "closed" };

                    // Filter by state if specified
                    if let Some(ref filter_state) = state {
                        if filter_state != issue_state {
                            continue;
                        }
                    }

                    issues.push(ForgeIssueInfo {
                        id: issue_id.to_hex().to_string(),
                        title: issue.title.clone(),
                        body: issue.body.clone(),
                        state: issue_state.to_string(),
                        labels: issue.labels.iter().cloned().collect(),
                        comment_count: issue.comments.len() as u32,
                        assignees: issue.assignees.iter().map(|a| hex::encode(a)).collect(),
                        created_at_ms: issue.created_at_ms,
                        updated_at_ms: issue.updated_at_ms,
                    });

                    // Apply limit after filtering
                    if issues.len() >= limit {
                        break;
                    }
                }
            }

            let count = issues.len() as u32;
            Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
                success: true,
                issues,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueListResult(ForgeIssueListResultResponse {
            success: false,
            issues: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommentInfo;
    use aspen_client_api::ForgeIssueInfo;
    use aspen_client_api::ForgeIssueResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.resolve_issue(&repo_id, &issue_hash).await {
        Ok(issue) => {
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
                success: true,
                issue: Some(ForgeIssueInfo {
                    id: issue_id,
                    title: issue.title,
                    body: issue.body,
                    state: if issue.state.is_open() {
                        "open".to_string()
                    } else {
                        "closed".to_string()
                    },
                    labels: issue.labels.into_iter().collect(),
                    comment_count: comments.len() as u32,
                    assignees: issue.assignees.iter().map(|a| hex::encode(a)).collect(),
                    created_at_ms: issue.created_at_ms,
                    updated_at_ms: issue.updated_at_ms,
                }),
                comments: Some(comments),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueResult(ForgeIssueResultResponse {
            success: false,
            issue: None,
            comments: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_comment_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
    body: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.add_comment(&repo_id, &issue_hash, &body).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_close_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.close_issue(&repo_id, &issue_hash, reason).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_reopen_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid issue ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.reopen_issue(&repo_id, &issue_hash).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Patch Operations
// ============================================================================

async fn handle_create_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    title: String,
    description: String,
    base: String,
    head: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgePatchInfo;
    use aspen_client_api::ForgePatchResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let base_hash = match blake3::Hash::from_hex(&base) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid base hash: {}", e)),
            }));
        }
    };

    let head_hash = match blake3::Hash::from_hex(&head) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid head hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.create_patch(&repo_id, &title, &description, base_hash, head_hash).await {
        Ok(patch_hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: true,
                patch: Some(ForgePatchInfo {
                    id: patch_hash.to_hex().to_string(),
                    title,
                    description,
                    state: "open".to_string(),
                    base,
                    head,
                    labels: vec![],
                    revision_count: 1,
                    approval_count: 0,
                    assignees: vec![],
                    created_at_ms: now,
                    updated_at_ms: now,
                }),
                comments: None,
                revisions: None,
                approvals: None,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_list_patches(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    state: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgePatchInfo;
    use aspen_client_api::ForgePatchListResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                success: false,
                patches: vec![],
                count: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let limit = limit.unwrap_or(50).min(1000) as usize;

    match forge_node.cobs.list_patches(&repo_id).await {
        Ok(patch_ids) => {
            let mut patches = Vec::new();
            // Filter first, then limit - fixes bug where merged patches wouldn't appear
            // if they weren't in the first N patches before filtering
            for patch_id in patch_ids.iter() {
                if let Ok(patch) = forge_node.cobs.resolve_patch(&repo_id, patch_id).await {
                    let patch_state = match patch.state {
                        aspen_forge::cob::PatchState::Open => "open",
                        aspen_forge::cob::PatchState::Merged { .. } => "merged",
                        aspen_forge::cob::PatchState::Closed { .. } => "closed",
                    };

                    // Filter by state if specified
                    if let Some(ref filter_state) = state {
                        if filter_state != patch_state {
                            continue;
                        }
                    }

                    patches.push(ForgePatchInfo {
                        id: patch_id.to_hex().to_string(),
                        title: patch.title.clone(),
                        description: patch.description.clone(),
                        state: patch_state.to_string(),
                        base: blake3::Hash::from_bytes(patch.base).to_hex().to_string(),
                        head: blake3::Hash::from_bytes(patch.head).to_hex().to_string(),
                        labels: patch.labels.iter().cloned().collect(),
                        revision_count: patch.revisions.len() as u32,
                        approval_count: patch.approvals.len() as u32,
                        assignees: patch.assignees.iter().map(|a| hex::encode(a)).collect(),
                        created_at_ms: patch.created_at_ms,
                        updated_at_ms: patch.updated_at_ms,
                    });

                    // Apply limit after filtering
                    if patches.len() >= limit {
                        break;
                    }
                }
            }

            let count = patches.len() as u32;
            Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
                success: true,
                patches,
                count,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchListResult(ForgePatchListResultResponse {
            success: false,
            patches: vec![],
            count: 0,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeCommentInfo;
    use aspen_client_api::ForgePatchApproval;
    use aspen_client_api::ForgePatchInfo;
    use aspen_client_api::ForgePatchResultResponse;
    use aspen_client_api::ForgePatchRevision;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.resolve_patch(&repo_id, &patch_hash).await {
        Ok(patch) => {
            let patch_state = match patch.state {
                aspen_forge::cob::PatchState::Open => "open",
                aspen_forge::cob::PatchState::Merged { .. } => "merged",
                aspen_forge::cob::PatchState::Closed { .. } => "closed",
            };

            let comments: Vec<ForgeCommentInfo> = patch
                .comments
                .iter()
                .map(|c| ForgeCommentInfo {
                    hash: hex::encode(c.change_hash),
                    author: hex::encode(c.author),
                    body: c.body.clone(),
                    timestamp_ms: c.timestamp_ms,
                })
                .collect();

            let revisions: Vec<ForgePatchRevision> = patch
                .revisions
                .iter()
                .map(|r| ForgePatchRevision {
                    hash: hex::encode(r.change_hash),
                    head: blake3::Hash::from_bytes(r.head).to_hex().to_string(),
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
                    commit: blake3::Hash::from_bytes(a.commit).to_hex().to_string(),
                    message: a.message.clone(),
                    timestamp_ms: a.timestamp_ms,
                })
                .collect();

            Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
                success: true,
                patch: Some(ForgePatchInfo {
                    id: patch_id,
                    title: patch.title,
                    description: patch.description,
                    state: patch_state.to_string(),
                    base: blake3::Hash::from_bytes(patch.base).to_hex().to_string(),
                    head: blake3::Hash::from_bytes(patch.head).to_hex().to_string(),
                    labels: patch.labels.into_iter().collect(),
                    revision_count: revisions.len() as u32,
                    approval_count: approvals.len() as u32,
                    assignees: patch.assignees.iter().map(|a| hex::encode(a)).collect(),
                    created_at_ms: patch.created_at_ms,
                    updated_at_ms: patch.updated_at_ms,
                }),
                comments: Some(comments),
                revisions: Some(revisions),
                approvals: Some(approvals),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchResult(ForgePatchResultResponse {
            success: false,
            patch: None,
            comments: None,
            revisions: None,
            approvals: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_update_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    head: String,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    let head_hash = match blake3::Hash::from_hex(&head) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid head hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.update_patch(&repo_id, &patch_hash, head_hash, message).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_approve_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    commit: String,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    let commit_hash = match blake3::Hash::from_hex(&commit) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid commit hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.approve_patch(&repo_id, &patch_hash, commit_hash, message).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_merge_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    merge_commit: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    let merge_hash = match blake3::Hash::from_hex(&merge_commit) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid merge commit hash: {}", e)),
            }));
        }
    };

    match forge_node.cobs.merge_patch(&repo_id, &patch_hash, merge_hash).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_close_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeOperationResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
                success: false,
                error: Some(format!("Invalid patch ID: {}", e)),
            }));
        }
    };

    match forge_node.cobs.close_patch(&repo_id, &patch_hash, reason).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(ForgeOperationResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Delegate Key
// ============================================================================

async fn handle_get_delegate_key(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeKeyResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
                success: false,
                public_key: None,
                secret_key: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Verify repo exists
    match forge_node.get_repo(&repo_id).await {
        Ok(_identity) => {
            // Return the node's key as the delegate key
            Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
                success: true,
                public_key: Some(forge_node.public_key().to_string()),
                secret_key: Some(hex::encode(forge_node.secret_key().to_bytes())),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
            success: false,
            public_key: None,
            secret_key: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Federation Operations
// ============================================================================

/// Count federated repositories by scanning for federation settings.
///
/// A repository is considered "federated" if it has federation settings
/// stored in KV with `mode != Disabled`. Uses `ForgeNode::count_federated_resources`
/// to scan persisted settings.
///
/// Returns 0 if no forge_node is available or scan fails.
async fn count_federated_repos(forge_node: &ForgeNodeRef) -> u32 {
    match forge_node.count_federated_resources().await {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!(error = %e, "failed to count federated repos");
            0
        }
    }
}

async fn handle_get_federation_status(
    ctx: &ClientProtocolContext,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationStatusResponse;

    // Check DHT availability via content_discovery service
    #[cfg(feature = "global-discovery")]
    let dht_enabled = ctx.content_discovery.is_some();
    #[cfg(not(feature = "global-discovery"))]
    let dht_enabled = false;

    // Get discovered cluster count from federation discovery service
    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    let discovered_clusters =
        ctx.federation_discovery.as_ref().map(|d| d.get_discovered_clusters().len() as u32).unwrap_or(0);
    #[cfg(not(all(feature = "forge", feature = "global-discovery")))]
    let discovered_clusters = 0u32;

    // Count federated repos by scanning for persisted federation settings
    // Repos with FederationSettings where mode != Disabled are considered federated
    let federated_repos = count_federated_repos(forge_node).await;

    // Check if federation identity is configured
    match &ctx.federation_identity {
        Some(identity) => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            enabled: true,
            cluster_name: identity.name().to_string(),
            cluster_key: identity.public_key().to_string(),
            dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: None,
        })),
        None => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: Some("Federation not configured for this node".to_string()),
        })),
    }
}

async fn handle_list_discovered_clusters(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DiscoveredClustersResponse;

    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    {
        use aspen_client_api::DiscoveredClusterInfo;
        let discovery = match &ctx.federation_discovery {
            Some(d) => d,
            None => {
                return Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
                    clusters: vec![],
                    count: 0,
                    error: Some("Federation discovery service not initialized".to_string()),
                }));
            }
        };

        let discovered = discovery.get_discovered_clusters();
        let clusters: Vec<DiscoveredClusterInfo> = discovered
            .iter()
            .map(|c| DiscoveredClusterInfo {
                cluster_key: c.cluster_key.to_string(),
                name: c.name.clone(),
                node_count: c.node_keys.len() as u32,
                capabilities: c.capabilities.clone(),
                discovered_at: format!("{:?}", c.discovered_at.elapsed()),
            })
            .collect();

        let count = clusters.len() as u32;
        Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
            clusters,
            count,
            error: None,
        }))
    }

    #[cfg(not(all(feature = "forge", feature = "global-discovery")))]
    {
        let _ = ctx; // Suppress unused warning
        Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
            clusters: vec![],
            count: 0,
            error: Some("Federation discovery requires 'forge' and 'global-discovery' features".to_string()),
        }))
    }
}

async fn handle_get_discovered_cluster(
    ctx: &ClientProtocolContext,
    cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DiscoveredClusterResponse;

    #[cfg(all(feature = "forge", feature = "global-discovery"))]
    {
        let discovery = match &ctx.federation_discovery {
            Some(d) => d,
            None => {
                return Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                    found: false,
                    cluster_key: None,
                    name: None,
                    node_count: None,
                    capabilities: None,
                    relay_urls: None,
                    discovered_at: None,
                }));
            }
        };

        // Parse the cluster key
        let key_bytes = match hex::decode(&cluster_key) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                arr
            }
            _ => {
                return Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                    found: false,
                    cluster_key: None,
                    name: None,
                    node_count: None,
                    capabilities: None,
                    relay_urls: None,
                    discovered_at: None,
                }));
            }
        };

        let public_key = match iroh::PublicKey::from_bytes(&key_bytes) {
            Ok(pk) => pk,
            Err(_) => {
                return Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                    found: false,
                    cluster_key: None,
                    name: None,
                    node_count: None,
                    capabilities: None,
                    relay_urls: None,
                    discovered_at: None,
                }));
            }
        };

        // Try to discover the cluster (will check cache first, then DHT if not found)
        match discovery.discover_cluster(&public_key).await {
            Some(cluster) => Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                found: true,
                cluster_key: Some(cluster.cluster_key.to_string()),
                name: Some(cluster.name),
                node_count: Some(cluster.node_keys.len() as u32),
                capabilities: Some(cluster.capabilities),
                relay_urls: Some(cluster.relay_urls),
                discovered_at: Some(format!("{:?}", cluster.discovered_at.elapsed())),
            })),
            None => Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                found: false,
                cluster_key: Some(cluster_key),
                name: None,
                node_count: None,
                capabilities: None,
                relay_urls: None,
                discovered_at: None,
            })),
        }
    }

    #[cfg(not(all(feature = "forge", feature = "global-discovery")))]
    {
        let _ = (ctx, cluster_key); // Suppress unused warnings
        Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
            found: false,
            cluster_key: None,
            name: None,
            node_count: None,
            capabilities: None,
            relay_urls: None,
            discovered_at: None,
        }))
    }
}

async fn handle_trust_cluster(ctx: &ClientProtocolContext, cluster_key: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::TrustClusterResultResponse;

    let trust_manager = match &ctx.federation_trust_manager {
        Some(tm) => tm,
        None => {
            return Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                success: false,
                error: Some("Trust management not available - federation not configured".to_string()),
            }));
        }
    };

    // Parse the cluster key from hex
    let key_bytes = match hex::decode(&cluster_key) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        _ => {
            return Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                success: false,
                error: Some("Invalid cluster key format (expected 64-character hex string)".to_string()),
            }));
        }
    };

    let public_key = match iroh::PublicKey::from_bytes(&key_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                success: false,
                error: Some("Invalid cluster public key".to_string()),
            }));
        }
    };

    // Add as trusted cluster
    trust_manager.add_trusted(public_key, format!("trusted-{}", &cluster_key[..8]), None);

    Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
        success: true,
        error: None,
    }))
}

async fn handle_untrust_cluster(ctx: &ClientProtocolContext, cluster_key: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::UntrustClusterResultResponse;

    let trust_manager = match &ctx.federation_trust_manager {
        Some(tm) => tm,
        None => {
            return Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                success: false,
                error: Some("Trust management not available - federation not configured".to_string()),
            }));
        }
    };

    // Parse the cluster key from hex
    let key_bytes = match hex::decode(&cluster_key) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        _ => {
            return Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                success: false,
                error: Some("Invalid cluster key format (expected 64-character hex string)".to_string()),
            }));
        }
    };

    let public_key = match iroh::PublicKey::from_bytes(&key_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                success: false,
                error: Some("Invalid cluster public key".to_string()),
            }));
        }
    };

    // Remove from trusted clusters
    if trust_manager.remove_trusted(&public_key) {
        Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
            success: true,
            error: None,
        }))
    } else {
        Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
            success: false,
            error: Some("Cluster was not in trusted list".to_string()),
        }))
    }
}

async fn handle_federate_repository(
    _forge_node: &ForgeNodeRef,
    _repo_id: String,
    _mode: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederateRepositoryResultResponse;

    // Federation integration is handled separately from ForgeNode
    Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
        success: false,
        fed_id: None,
        error: Some("Federation not available through RPC".to_string()),
    }))
}

async fn handle_list_federated_repositories(forge_node: &ForgeNodeRef) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederatedRepoInfo;
    use aspen_client_api::FederatedRepositoriesResponse;

    // List federated repositories from KV storage
    match forge_node.list_federated_resources(None, 1000).await {
        Ok(federated) => {
            let repositories: Vec<FederatedRepoInfo> = federated
                .into_iter()
                .map(|(fed_id, settings)| {
                    let mode = match settings.mode {
                        aspen_cluster::federation::FederationMode::Disabled => "disabled",
                        aspen_cluster::federation::FederationMode::Public => "public",
                        aspen_cluster::federation::FederationMode::AllowList => "allowlist",
                    };
                    FederatedRepoInfo {
                        repo_id: hex::encode(fed_id.local_id()),
                        mode: mode.to_string(),
                        fed_id: fed_id.to_string(),
                    }
                })
                .collect();
            let count = repositories.len() as u32;
            Ok(ClientRpcResponse::FederatedRepositories(FederatedRepositoriesResponse {
                repositories,
                count,
                error: None,
            }))
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to list federated repositories");
            Ok(ClientRpcResponse::FederatedRepositories(FederatedRepositoriesResponse {
                repositories: vec![],
                count: 0,
                error: Some(format!("Failed to list federated repositories: {}", e)),
            }))
        }
    }
}

async fn handle_fetch_federated(
    _forge_node: &ForgeNodeRef,
    _federated_id: String,
    _remote_cluster: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeFetchFederatedResultResponse;

    // Federation integration is handled separately from ForgeNode
    Ok(ClientRpcResponse::ForgeFetchResult(ForgeFetchFederatedResultResponse {
        success: false,
        remote_cluster: None,
        fetched: 0,
        already_present: 0,
        errors: vec![],
        error: Some("Federation not available through RPC".to_string()),
    }))
}

// ============================================================================
// Git Bridge Operations (requires git-bridge feature)
// ============================================================================

#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_list_refs(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgeListRefsResponse;
    use aspen_client_api::GitBridgeRefInfo;
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitExporter;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                success: false,
                refs: vec![],
                head: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Create exporter with hash mapping
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let exporter = GitExporter::new(
        mapping,
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    match exporter.list_refs(&repo_id).await {
        Ok(ref_list) => {
            let refs: Vec<GitBridgeRefInfo> = ref_list
                .iter()
                .filter_map(|(name, sha1_opt)| {
                    sha1_opt.map(|sha1| GitBridgeRefInfo {
                        ref_name: format!("refs/{}", name),
                        sha1: sha1.to_hex(),
                    })
                })
                .collect();

            // Determine HEAD - look for main or master branch
            let head = ref_list
                .iter()
                .find(|(name, sha1_opt)| sha1_opt.is_some() && (name == "heads/main" || name == "heads/master"))
                .map(|(name, _)| format!("refs/{}", name));

            Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                success: true,
                refs,
                head,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
            success: false,
            refs: vec![],
            head: None,
            error: Some(e.to_string()),
        })),
    }
}

#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_fetch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    want: Vec<String>,
    have: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use std::collections::HashSet;

    use aspen_client_api::GitBridgeFetchResponse;
    use aspen_client_api::GitBridgeObject;
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitExporter;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Parse want/have SHA-1 hashes
    let want_hashes: Result<Vec<Sha1Hash>, _> = want.iter().map(|s| Sha1Hash::from_hex(s)).collect();
    let want_hashes = match want_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("Invalid want hash: {}", e)),
            }));
        }
    };

    let have_hashes: Result<HashSet<Sha1Hash>, _> = have.iter().map(|s| Sha1Hash::from_hex(s)).collect();
    let have_hashes = match have_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                success: false,
                objects: vec![],
                skipped: 0,
                error: Some(format!("Invalid have hash: {}", e)),
            }));
        }
    };

    // Create exporter with hash mapping
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let exporter = GitExporter::new(
        mapping.clone(),
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    // Export commits for all wanted refs
    let mut objects = Vec::new();
    let mut skipped: usize = 0;

    for want_sha1 in &want_hashes {
        // Get the blake3 hash for this SHA-1 using the mapping
        if let Ok(Some((blake3_hash, _))) = mapping.get_blake3(&repo_id, want_sha1).await {
            match exporter.export_commit_dag(&repo_id, blake3_hash, &have_hashes).await {
                Ok(exported) => {
                    for obj in exported.objects {
                        objects.push(GitBridgeObject {
                            sha1: obj.sha1.to_hex(),
                            object_type: obj.object_type.as_str().to_string(),
                            data: obj.content,
                        });
                    }
                    skipped += exported.objects_skipped;
                }
                Err(e) => {
                    return Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
                        success: false,
                        objects: vec![],
                        skipped: 0,
                        error: Some(e.to_string()),
                    }));
                }
            }
        }
    }

    Ok(ClientRpcResponse::GitBridgeFetch(GitBridgeFetchResponse {
        success: true,
        objects,
        skipped,
        error: None,
    }))
}

#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_push(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    objects: Vec<aspen_client_api::GitBridgeObject>,
    refs: Vec<aspen_client_api::GitBridgeRefUpdate>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgePushResponse;
    use aspen_client_api::GitBridgeRefResult;
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitImporter;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                success: false,
                objects_imported: 0,
                objects_skipped: 0,
                ref_results: vec![],
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Create importer
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let importer = GitImporter::new(
        mapping,
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    // Convert objects to import format (with git headers)
    use aspen_forge::git::bridge::GitObjectType;
    let import_objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)> = objects
        .iter()
        .filter_map(|obj| {
            let sha1 = Sha1Hash::from_hex(&obj.sha1).ok()?;
            let obj_type = match obj.object_type.as_str() {
                "blob" => GitObjectType::Blob,
                "tree" => GitObjectType::Tree,
                "commit" => GitObjectType::Commit,
                "tag" => GitObjectType::Tag,
                _ => return None,
            };
            // Build full git object bytes with header: "type size\0content"
            let header = format!("{} {}\0", obj.object_type, obj.data.len());
            let mut git_bytes = Vec::with_capacity(header.len() + obj.data.len());
            git_bytes.extend_from_slice(header.as_bytes());
            git_bytes.extend_from_slice(&obj.data);
            Some((sha1, obj_type, git_bytes))
        })
        .collect();

    // Import objects using batch import which handles topological ordering
    let (objects_imported, objects_skipped) = match importer.import_objects(&repo_id, import_objects).await {
        Ok(result) => (result.objects_imported, result.objects_skipped),
        Err(e) => {
            return Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
                success: false,
                objects_imported: 0,
                objects_skipped: 0,
                ref_results: vec![],
                error: Some(format!("Import failed: {}", e)),
            }));
        }
    };

    // Update refs
    let mut ref_results = Vec::new();
    for ref_update in &refs {
        // Get the blake3 hash for the new SHA-1
        let new_sha1 = match Sha1Hash::from_hex(&ref_update.new_sha1) {
            Ok(h) => h,
            Err(e) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    success: false,
                    error: Some(format!("Invalid SHA-1: {}", e)),
                });
                continue;
            }
        };

        // Get blake3 hash from mapping
        let blake3_hash = match importer.get_blake3(&repo_id, &new_sha1).await {
            Ok(Some(h)) => h,
            Ok(None) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    success: false,
                    error: Some("Object not found in mapping".to_string()),
                });
                continue;
            }
            Err(e) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
                continue;
            }
        };

        // Convert ref name (strip refs/ prefix)
        let ref_name = ref_update.ref_name.strip_prefix("refs/").unwrap_or(&ref_update.ref_name);

        match forge_node.refs.set(&repo_id, ref_name, blake3_hash).await {
            Ok(()) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                ref_results.push(GitBridgeRefResult {
                    ref_name: ref_update.ref_name.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let all_success = ref_results.iter().all(|r| r.success);
    Ok(ClientRpcResponse::GitBridgePush(GitBridgePushResponse {
        success: all_success,
        objects_imported,
        objects_skipped,
        ref_results,
        error: None,
    }))
}

/// Handle GitBridgePushStart - initiate chunked push session.
#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_push_start(
    _forge_node: &ForgeNodeRef,
    repo_id: String,
    total_objects: u64,
    total_size_bytes: u64,
    refs: Vec<aspen_client_api::GitBridgeRefUpdate>,
    _metadata: Option<aspen_client_api::GitBridgePushMetadata>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DEFAULT_GIT_CHUNK_SIZE;
    use aspen_client_api::GitBridgePushStartResponse;
    use uuid::Uuid;

    // Generate unique session ID
    let session_id = Uuid::new_v4().to_string();

    // Validate limits
    if total_objects > 100_000 {
        return Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
            session_id: session_id.clone(),
            max_chunk_size: DEFAULT_GIT_CHUNK_SIZE,
            success: false,
            error: Some("Too many objects - maximum 100,000 allowed".to_string()),
        }));
    }

    if total_size_bytes > 1024 * 1024 * 1024 {
        // 1GB limit
        return Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
            session_id: session_id.clone(),
            max_chunk_size: DEFAULT_GIT_CHUNK_SIZE,
            success: false,
            error: Some("Push too large - maximum 1GB allowed".to_string()),
        }));
    }

    tracing::info!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        repo_id = repo_id,
        total_objects = total_objects,
        total_size_bytes = total_size_bytes,
        "Starting chunked git push session"
    );

    // Store session state for tracking chunks
    let session = ChunkedPushSession {
        repo_id: repo_id.clone(),
        refs,
        total_objects,
        total_size_bytes,
        chunks_received: HashSet::new(),
        total_chunks: None,
        objects: Vec::new(),
        created_at: std::time::Instant::now(),
    };

    let store = get_session_store();
    let mut sessions = store.lock().unwrap();

    // Cleanup expired sessions first (Tiger Style: opportunistic cleanup)
    sessions.retain(|_, s| s.created_at.elapsed() < PUSH_SESSION_TIMEOUT);

    // Check capacity (Tiger Style bound)
    if sessions.len() >= MAX_CONCURRENT_PUSH_SESSIONS {
        return Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
            session_id,
            max_chunk_size: DEFAULT_GIT_CHUNK_SIZE,
            success: false,
            error: Some("Too many concurrent push sessions - try again later".to_string()),
        }));
    }

    sessions.insert(session_id.clone(), session);

    Ok(ClientRpcResponse::GitBridgePushStart(GitBridgePushStartResponse {
        session_id,
        max_chunk_size: DEFAULT_GIT_CHUNK_SIZE,
        success: true,
        error: None,
    }))
}

/// Handle GitBridgePushChunk - receive and validate chunk.
#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_push_chunk(
    _forge_node: &ForgeNodeRef,
    session_id: String,
    chunk_id: u64,
    total_chunks: u64,
    objects: Vec<aspen_client_api::GitBridgeObject>,
    chunk_hash: [u8; 32],
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgePushChunkResponse;
    use blake3::Hasher;

    // Validate chunk hash for integrity
    let mut hasher = Hasher::new();
    for obj in &objects {
        hasher.update(obj.sha1.as_bytes());
        hasher.update(obj.object_type.as_bytes());
        hasher.update(&obj.data);
    }
    let computed_hash = *hasher.finalize().as_bytes();

    if computed_hash != chunk_hash {
        tracing::warn!(
            target: "aspen_forge::git_bridge",
            session_id = session_id,
            chunk_id = chunk_id,
            "Chunk hash mismatch - integrity check failed"
        );
        return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
            session_id,
            chunk_id,
            success: false,
            error: Some("Chunk integrity check failed - hash mismatch".to_string()),
        }));
    }

    tracing::debug!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        chunk_id = chunk_id,
        total_chunks = total_chunks,
        objects_count = objects.len(),
        "Received chunk with valid hash"
    );

    // Store chunk data and validate sequence
    let store = get_session_store();
    let mut sessions = store.lock().unwrap();

    let session = match sessions.get_mut(&session_id) {
        Some(s) => s,
        None => {
            return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
                session_id,
                chunk_id,
                success: false,
                error: Some("Session not found or expired".to_string()),
            }));
        }
    };

    // Check session timeout
    if session.created_at.elapsed() > PUSH_SESSION_TIMEOUT {
        let sid = session_id.clone();
        sessions.remove(&session_id);
        return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
            session_id: sid,
            chunk_id,
            success: false,
            error: Some("Session expired".to_string()),
        }));
    }

    // Set or validate total_chunks consistency
    match session.total_chunks {
        Some(tc) if tc != total_chunks => {
            return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
                session_id,
                chunk_id,
                success: false,
                error: Some("Inconsistent total_chunks value".to_string()),
            }));
        }
        None => session.total_chunks = Some(total_chunks),
        _ => {}
    }

    // Check for duplicate chunk
    if session.chunks_received.contains(&chunk_id) {
        return Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
            session_id,
            chunk_id,
            success: false,
            error: Some("Duplicate chunk received".to_string()),
        }));
    }

    // Store chunk data
    session.chunks_received.insert(chunk_id);
    session.objects.extend(objects);

    Ok(ClientRpcResponse::GitBridgePushChunk(GitBridgePushChunkResponse {
        session_id,
        chunk_id,
        success: true,
        error: None,
    }))
}

/// Handle GitBridgePushComplete - finalize chunked push.
#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_push_complete(
    forge_node: &ForgeNodeRef,
    session_id: String,
    content_hash: [u8; 32],
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::GitBridgePushCompleteResponse;
    use blake3::Hasher;

    tracing::info!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        "Completing chunked git push session"
    );

    // Retrieve and remove session from store
    let session = {
        let store = get_session_store();
        let mut sessions = store.lock().unwrap();
        match sessions.remove(&session_id) {
            Some(s) => s,
            None => {
                return Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
                    session_id,
                    success: false,
                    objects_imported: 0,
                    objects_skipped: 0,
                    ref_results: vec![],
                    error: Some("Session not found or expired".to_string()),
                }));
            }
        }
    };

    // Validate all chunks were received
    let total_chunks = session.total_chunks.unwrap_or(0);
    if session.chunks_received.len() as u64 != total_chunks {
        return Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
            session_id,
            success: false,
            objects_imported: 0,
            objects_skipped: 0,
            ref_results: vec![],
            error: Some(format!("Missing chunks: received {}/{}", session.chunks_received.len(), total_chunks)),
        }));
    }

    // Validate content hash matches reassembled chunks
    let mut hasher = Hasher::new();
    for obj in &session.objects {
        hasher.update(obj.sha1.as_bytes());
        hasher.update(obj.object_type.as_bytes());
        hasher.update(&obj.data);
    }
    let computed_hash = *hasher.finalize().as_bytes();

    if computed_hash != content_hash {
        tracing::warn!(
            target: "aspen_forge::git_bridge",
            session_id = session_id,
            "Content hash mismatch on push complete"
        );
        return Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
            session_id,
            success: false,
            objects_imported: 0,
            objects_skipped: 0,
            ref_results: vec![],
            error: Some("Content hash mismatch - push data corrupted".to_string()),
        }));
    }

    tracing::info!(
        target: "aspen_forge::git_bridge",
        session_id = session_id,
        objects_count = session.objects.len(),
        refs_count = session.refs.len(),
        "All chunks received, processing push"
    );

    // Reuse existing push logic
    let push_result = handle_git_bridge_push(forge_node, session.repo_id, session.objects, session.refs).await?;

    // Convert GitBridgePushResponse to GitBridgePushCompleteResponse
    match push_result {
        ClientRpcResponse::GitBridgePush(resp) => {
            Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
                session_id,
                success: resp.success,
                objects_imported: resp.objects_imported,
                objects_skipped: resp.objects_skipped,
                ref_results: resp.ref_results,
                error: resp.error,
            }))
        }
        _ => Ok(ClientRpcResponse::GitBridgePushComplete(GitBridgePushCompleteResponse {
            session_id,
            success: false,
            objects_imported: 0,
            objects_skipped: 0,
            ref_results: vec![],
            error: Some("Unexpected response from push handler".to_string()),
        })),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_client_api::ClientRpcRequest;
    use aspen_client_api::ClientRpcResponse;

    use super::*;
    use crate::context::test_support::TestContextBuilder;
    use crate::test_mocks::MockEndpointProvider;
    #[cfg(feature = "sql")]
    use crate::test_mocks::mock_sql_executor;

    /// Create a test context without forge node (to test unavailability).
    async fn setup_test_context_without_forge() -> ClientProtocolContext {
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        let mut builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster");

        #[cfg(feature = "sql")]
        {
            builder = builder.with_sql_executor(mock_sql_executor());
        }

        builder.build()
    }

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
    // Forge Availability Tests
    // =========================================================================

    #[tokio::test]
    async fn test_forge_unavailable_error_create_repo() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::ForgeCreateRepo {
            name: "test-repo".to_string(),
            description: None,
            default_branch: None,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "FORGE_UNAVAILABLE");
                assert!(err.message.contains("not configured"));
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_forge_unavailable_error_get_repo() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::ForgeGetRepo {
            repo_id: "abcd1234".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "FORGE_UNAVAILABLE");
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_forge_unavailable_error_store_blob() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::ForgeStoreBlob {
            repo_id: "abcd1234".to_string(),
            content: b"test content".to_vec(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "FORGE_UNAVAILABLE");
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_forge_unavailable_error_create_issue() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::ForgeCreateIssue {
            repo_id: "abcd1234".to_string(),
            title: "Test Issue".to_string(),
            body: "Issue body".to_string(),
            labels: vec![],
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "FORGE_UNAVAILABLE");
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_forge_unavailable_error_create_patch() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::ForgeCreatePatch {
            repo_id: "abcd1234".to_string(),
            title: "Test Patch".to_string(),
            description: "Patch description".to_string(),
            base: "base_hash".to_string(),
            head: "head_hash".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "FORGE_UNAVAILABLE");
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_forge_unavailable_error_federation() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::GetFederationStatus;

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Error(err) => {
                assert_eq!(err.code, "FORGE_UNAVAILABLE");
            }
            other => panic!("expected Error response, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_forge_unavailable_error_git_bridge() {
        let ctx = setup_test_context_without_forge().await;
        let handler = ForgeHandler;

        let request = ClientRpcRequest::GitBridgeListRefs {
            repo_id: "abcd1234".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            // When forge feature is enabled but node isn't configured
            ClientRpcResponse::Error(err) => {
                // Either FORGE_UNAVAILABLE or GIT_BRIDGE_UNAVAILABLE is acceptable
                assert!(
                    err.code == "FORGE_UNAVAILABLE" || err.code == "GIT_BRIDGE_UNAVAILABLE",
                    "expected FORGE_UNAVAILABLE or GIT_BRIDGE_UNAVAILABLE, got: {}",
                    err.code
                );
            }
            other => panic!("expected Error response, got {:?}", other),
        }
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
