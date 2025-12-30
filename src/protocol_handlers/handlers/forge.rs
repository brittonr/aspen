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

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

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
                return Ok(ClientRpcResponse::error(
                    "FORGE_UNAVAILABLE",
                    "Forge feature not configured on this node",
                ));
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

            ClientRpcRequest::ForgeGetRepo { repo_id } => {
                handle_get_repo(forge_node, repo_id).await
            }

            ClientRpcRequest::ForgeListRepos { limit, offset } => {
                handle_list_repos(ctx, limit, offset).await
            }

            // ================================================================
            // Git Object Operations
            // ================================================================
            ClientRpcRequest::ForgeStoreBlob { repo_id, content } => {
                handle_store_blob(forge_node, repo_id, content).await
            }

            ClientRpcRequest::ForgeGetBlob { hash } => handle_get_blob(forge_node, hash).await,

            ClientRpcRequest::ForgeCreateTree {
                repo_id,
                entries_json,
            } => handle_create_tree(forge_node, repo_id, entries_json).await,

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
            ClientRpcRequest::ForgeGetRef { repo_id, ref_name } => {
                handle_get_ref(forge_node, repo_id, ref_name).await
            }

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
                signer: _,
                signature: _,
                timestamp_ms: _,
            } => handle_cas_ref(forge_node, repo_id, ref_name, expected, new_hash).await,

            ClientRpcRequest::ForgeListBranches { repo_id } => {
                handle_list_branches(forge_node, repo_id).await
            }

            ClientRpcRequest::ForgeListTags { repo_id } => {
                handle_list_tags(forge_node, repo_id).await
            }

            // ================================================================
            // Issue Operations
            // ================================================================
            ClientRpcRequest::ForgeCreateIssue {
                repo_id,
                title,
                body,
                labels,
            } => handle_create_issue(forge_node, repo_id, title, body, labels).await,

            ClientRpcRequest::ForgeListIssues {
                repo_id,
                state,
                limit,
            } => handle_list_issues(forge_node, repo_id, state, limit).await,

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

            ClientRpcRequest::ForgeListPatches {
                repo_id,
                state,
                limit,
            } => handle_list_patches(forge_node, repo_id, state, limit).await,

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
            ClientRpcRequest::ForgeGetDelegateKey { repo_id } => {
                handle_get_delegate_key(forge_node, repo_id).await
            }

            // ================================================================
            // Federation Operations
            // ================================================================
            ClientRpcRequest::GetFederationStatus => handle_get_federation_status(forge_node).await,

            ClientRpcRequest::ListDiscoveredClusters => {
                handle_list_discovered_clusters(ctx).await
            }

            ClientRpcRequest::GetDiscoveredCluster { cluster_key } => {
                handle_get_discovered_cluster(ctx, cluster_key).await
            }

            ClientRpcRequest::TrustCluster { cluster_key } => {
                handle_trust_cluster(ctx, cluster_key).await
            }

            ClientRpcRequest::UntrustCluster { cluster_key } => {
                handle_untrust_cluster(ctx, cluster_key).await
            }

            ClientRpcRequest::FederateRepository { repo_id, mode } => {
                handle_federate_repository(forge_node, repo_id, mode).await
            }

            ClientRpcRequest::ListFederatedRepositories => {
                handle_list_federated_repositories(forge_node).await
            }

            ClientRpcRequest::ForgeFetchFederated {
                federated_id,
                remote_cluster,
            } => handle_fetch_federated(forge_node, federated_id, remote_cluster).await,

            // ================================================================
            // Git Bridge Operations (requires git-bridge feature)
            // ================================================================
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeListRefs { repo_id } => {
                handle_git_bridge_list_refs(forge_node, repo_id).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetch {
                repo_id,
                want,
                have,
            } => handle_git_bridge_fetch(forge_node, repo_id, want, have).await,

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePush {
                repo_id,
                objects,
                refs,
            } => handle_git_bridge_push(forge_node, repo_id, objects, refs).await,

            // Return error when git-bridge feature is not enabled
            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgePush { .. } => {
                Ok(ClientRpcResponse::error(
                    "GIT_BRIDGE_UNAVAILABLE",
                    "Git bridge feature not enabled. Rebuild with --features git-bridge",
                ))
            }

            _ => Err(anyhow::anyhow!("request not handled by ForgeHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ForgeHandler"
    }
}

// Type alias for the concrete ForgeNode type used in the context
type ForgeNodeRef =
    std::sync::Arc<crate::forge::ForgeNode<crate::blob::IrohBlobStore, dyn crate::api::KeyValueStore>>;

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
    use crate::client_rpc::{ForgeRepoInfo, ForgeRepoResultResponse};

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

async fn handle_get_repo(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeRepoInfo, ForgeRepoResultResponse};
    use crate::forge::identity::RepoId;

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
    use crate::api::ScanRequest;
    use crate::client_rpc::{ForgeRepoInfo, ForgeRepoListResultResponse};
    use crate::forge::constants::KV_PREFIX_REPOS;

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
                        let repo_id = entry
                            .key
                            .strip_prefix(KV_PREFIX_REPOS)?
                            .strip_suffix(":identity")?;
                        Some(ForgeRepoInfo {
                            id: repo_id.to_string(),
                            name: String::new(), // Would need to decode identity
                            description: None,
                            default_branch: "main".to_string(),
                            delegates: vec![],
                            threshold: 1,
                            created_at_ms: 0,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            let count = repos.len() as u32;
            Ok(ClientRpcResponse::ForgeRepoListResult(
                ForgeRepoListResultResponse {
                    success: true,
                    repos,
                    count,
                    error: None,
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRepoListResult(
            ForgeRepoListResultResponse {
                success: false,
                repos: vec![],
                count: 0,
                error: Some(e.to_string()),
            },
        )),
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
    use crate::client_rpc::ForgeBlobResultResponse;

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

async fn handle_get_blob(
    forge_node: &ForgeNodeRef,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeBlobResultResponse;

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
    use crate::client_rpc::{ForgeTreeEntry as RpcTreeEntry, ForgeTreeResultResponse};
    use crate::forge::TreeEntry;

    // Parse entries from JSON
    let parsed: Vec<RpcTreeEntry> = match serde_json::from_str(&entries_json) {
        Ok(e) => e,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(
                ForgeTreeResultResponse {
                    success: false,
                    hash: None,
                    entries: None,
                    error: Some(format!("Invalid entries JSON: {}", e)),
                },
            ));
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
            return Ok(ClientRpcResponse::ForgeTreeResult(
                ForgeTreeResultResponse {
                    success: false,
                    hash: None,
                    entries: None,
                    error: Some(format!("Invalid entry hash: {}", e)),
                },
            ));
        }
    };

    match forge_node.git.create_tree(&entries).await {
        Ok(hash) => Ok(ClientRpcResponse::ForgeTreeResult(
            ForgeTreeResultResponse {
                success: true,
                hash: Some(hash.to_hex().to_string()),
                entries: Some(parsed),
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(
            ForgeTreeResultResponse {
                success: false,
                hash: None,
                entries: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_get_tree(
    forge_node: &ForgeNodeRef,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeTreeEntry as RpcTreeEntry, ForgeTreeResultResponse};

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeTreeResult(
                ForgeTreeResultResponse {
                    success: false,
                    hash: None,
                    entries: None,
                    error: Some(format!("Invalid hash: {}", e)),
                },
            ));
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

            Ok(ClientRpcResponse::ForgeTreeResult(
                ForgeTreeResultResponse {
                    success: true,
                    hash: Some(hash.to_hex().to_string()),
                    entries: Some(entries),
                    error: None,
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeTreeResult(
            ForgeTreeResultResponse {
                success: false,
                hash: None,
                entries: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_commit(
    forge_node: &ForgeNodeRef,
    _repo_id: String,
    tree: String,
    parents: Vec<String>,
    message: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeCommitInfo, ForgeCommitResultResponse};

    let tree_hash = match blake3::Hash::from_hex(&tree) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(
                ForgeCommitResultResponse {
                    success: false,
                    commit: None,
                    error: Some(format!("Invalid tree hash: {}", e)),
                },
            ));
        }
    };

    let parent_hashes: Result<Vec<blake3::Hash>, _> =
        parents.iter().map(|p| blake3::Hash::from_hex(p)).collect();
    let parent_hashes = match parent_hashes {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(
                ForgeCommitResultResponse {
                    success: false,
                    commit: None,
                    error: Some(format!("Invalid parent hash: {}", e)),
                },
            ));
        }
    };

    match forge_node.git.commit(tree_hash, parent_hashes.clone(), &message).await {
        Ok(hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgeCommitResult(
                ForgeCommitResultResponse {
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
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(
            ForgeCommitResultResponse {
                success: false,
                commit: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_get_commit(
    forge_node: &ForgeNodeRef,
    hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeCommitInfo, ForgeCommitResultResponse};

    let hash = match blake3::Hash::from_hex(&hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeCommitResult(
                ForgeCommitResultResponse {
                    success: false,
                    commit: None,
                    error: Some(format!("Invalid hash: {}", e)),
                },
            ));
        }
    };

    match forge_node.git.get_commit(&hash).await {
        Ok(commit) => Ok(ClientRpcResponse::ForgeCommitResult(
            ForgeCommitResultResponse {
                success: true,
                commit: Some(ForgeCommitInfo {
                    hash: hash.to_hex().to_string(),
                    tree: blake3::Hash::from_bytes(commit.tree).to_hex().to_string(),
                    parents: commit
                        .parents
                        .iter()
                        .map(|p| blake3::Hash::from_bytes(*p).to_hex().to_string())
                        .collect(),
                    author_name: commit.author.name.clone(),
                    author_email: Some(commit.author.email.clone()),
                    author_key: commit.author.public_key.map(|k| hex::encode(k.as_bytes())),
                    message: commit.message.clone(),
                    timestamp_ms: commit.author.timestamp_ms,
                }),
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeCommitResult(
            ForgeCommitResultResponse {
                success: false,
                commit: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_log(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    ref_name: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeCommitInfo, ForgeLogResultResponse};
    use crate::forge::identity::RepoId;

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
                    parents: commit
                        .parents
                        .iter()
                        .map(|p| blake3::Hash::from_bytes(*p).to_hex().to_string())
                        .collect(),
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
    use crate::client_rpc::{ForgeRefInfo, ForgeRefResultResponse};
    use crate::forge::identity::RepoId;

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
    use crate::client_rpc::{ForgeRefInfo, ForgeRefResultResponse};
    use crate::forge::identity::RepoId;

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
    use crate::client_rpc::ForgeRefResultResponse;
    use crate::forge::identity::RepoId;

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
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeRefInfo, ForgeRefResultResponse};
    use crate::forge::identity::RepoId;

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

    match forge_node
        .refs
        .compare_and_set(&repo_id, &ref_name, expected_hash, new_hash)
        .await
    {
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

async fn handle_list_branches(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeRefInfo, ForgeRefListResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefListResult(
                ForgeRefListResultResponse {
                    success: false,
                    refs: vec![],
                    count: 0,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
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
            Ok(ClientRpcResponse::ForgeRefListResult(
                ForgeRefListResultResponse {
                    success: true,
                    refs,
                    count,
                    error: None,
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRefListResult(
            ForgeRefListResultResponse {
                success: false,
                refs: vec![],
                count: 0,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_list_tags(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeRefInfo, ForgeRefListResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRefListResult(
                ForgeRefListResultResponse {
                    success: false,
                    refs: vec![],
                    count: 0,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
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
            Ok(ClientRpcResponse::ForgeRefListResult(
                ForgeRefListResultResponse {
                    success: true,
                    refs,
                    count,
                    error: None,
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeRefListResult(
            ForgeRefListResultResponse {
                success: false,
                refs: vec![],
                count: 0,
                error: Some(e.to_string()),
            },
        )),
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
    use crate::client_rpc::{ForgeIssueInfo, ForgeIssueResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(
                ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.create_issue(&repo_id, &title, &body, labels.clone()).await {
        Ok(issue_hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgeIssueResult(
                ForgeIssueResultResponse {
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
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueResult(
            ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_list_issues(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    state: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeIssueInfo, ForgeIssueListResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueListResult(
                ForgeIssueListResultResponse {
                    success: false,
                    issues: vec![],
                    count: 0,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let limit = limit.unwrap_or(50).min(1000);

    match forge_node.cobs.list_issues(&repo_id).await {
        Ok(issue_ids) => {
            let mut issues = Vec::new();
            for issue_id in issue_ids.iter().take(limit as usize) {
                if let Ok(issue) = forge_node.cobs.resolve_issue(&repo_id, issue_id).await {
                    let issue_state = if issue.state.is_open() {
                        "open"
                    } else {
                        "closed"
                    };

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
                        assignees: issue
                            .assignees
                            .iter()
                            .map(|a| hex::encode(a))
                            .collect(),
                        created_at_ms: issue.created_at_ms,
                        updated_at_ms: issue.updated_at_ms,
                    });
                }
            }

            let count = issues.len() as u32;
            Ok(ClientRpcResponse::ForgeIssueListResult(
                ForgeIssueListResultResponse {
                    success: true,
                    issues,
                    count,
                    error: None,
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueListResult(
            ForgeIssueListResultResponse {
                success: false,
                issues: vec![],
                count: 0,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_get_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgeCommentInfo, ForgeIssueInfo, ForgeIssueResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(
                ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeIssueResult(
                ForgeIssueResultResponse {
                    success: false,
                    issue: None,
                    comments: None,
                    error: Some(format!("Invalid issue ID: {}", e)),
                },
            ));
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

            Ok(ClientRpcResponse::ForgeIssueResult(
                ForgeIssueResultResponse {
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
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeIssueResult(
            ForgeIssueResultResponse {
                success: false,
                issue: None,
                comments: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_comment_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
    body: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid issue ID: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.add_comment(&repo_id, &issue_hash, &body).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_close_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid issue ID: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.close_issue(&repo_id, &issue_hash, reason).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_reopen_issue(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    issue_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let issue_hash = match blake3::Hash::from_hex(&issue_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid issue ID: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.reopen_issue(&repo_id, &issue_hash).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
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
    use crate::client_rpc::{ForgePatchInfo, ForgePatchResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let base_hash = match blake3::Hash::from_hex(&base) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some(format!("Invalid base hash: {}", e)),
                },
            ));
        }
    };

    let head_hash = match blake3::Hash::from_hex(&head) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some(format!("Invalid head hash: {}", e)),
                },
            ));
        }
    };

    match forge_node
        .cobs
        .create_patch(&repo_id, &title, &description, base_hash, head_hash)
        .await
    {
        Ok(patch_hash) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
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
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchResult(
            ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_list_patches(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    state: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{ForgePatchInfo, ForgePatchListResultResponse};
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchListResult(
                ForgePatchListResultResponse {
                    success: false,
                    patches: vec![],
                    count: 0,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let limit = limit.unwrap_or(50).min(1000);

    match forge_node.cobs.list_patches(&repo_id).await {
        Ok(patch_ids) => {
            let mut patches = Vec::new();
            for patch_id in patch_ids.iter().take(limit as usize) {
                if let Ok(patch) = forge_node.cobs.resolve_patch(&repo_id, patch_id).await {
                    let patch_state = match patch.state {
                        crate::forge::cob::PatchState::Open => "open",
                        crate::forge::cob::PatchState::Merged { .. } => "merged",
                        crate::forge::cob::PatchState::Closed { .. } => "closed",
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
                        assignees: patch
                            .assignees
                            .iter()
                            .map(|a| hex::encode(a))
                            .collect(),
                        created_at_ms: patch.created_at_ms,
                        updated_at_ms: patch.updated_at_ms,
                    });
                }
            }

            let count = patches.len() as u32;
            Ok(ClientRpcResponse::ForgePatchListResult(
                ForgePatchListResultResponse {
                    success: true,
                    patches,
                    count,
                    error: None,
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchListResult(
            ForgePatchListResultResponse {
                success: false,
                patches: vec![],
                count: 0,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_get_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{
        ForgeCommentInfo, ForgePatchApproval, ForgePatchInfo, ForgePatchResultResponse,
        ForgePatchRevision,
    };
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
                    success: false,
                    patch: None,
                    comments: None,
                    revisions: None,
                    approvals: None,
                    error: Some(format!("Invalid patch ID: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.resolve_patch(&repo_id, &patch_hash).await {
        Ok(patch) => {
            let patch_state = match patch.state {
                crate::forge::cob::PatchState::Open => "open",
                crate::forge::cob::PatchState::Merged { .. } => "merged",
                crate::forge::cob::PatchState::Closed { .. } => "closed",
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

            Ok(ClientRpcResponse::ForgePatchResult(
                ForgePatchResultResponse {
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
                },
            ))
        }
        Err(e) => Ok(ClientRpcResponse::ForgePatchResult(
            ForgePatchResultResponse {
                success: false,
                patch: None,
                comments: None,
                revisions: None,
                approvals: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_update_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    head: String,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid patch ID: {}", e)),
                },
            ));
        }
    };

    let head_hash = match blake3::Hash::from_hex(&head) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid head hash: {}", e)),
                },
            ));
        }
    };

    match forge_node
        .cobs
        .update_patch(&repo_id, &patch_hash, head_hash, message)
        .await
    {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_approve_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    commit: String,
    message: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid patch ID: {}", e)),
                },
            ));
        }
    };

    let commit_hash = match blake3::Hash::from_hex(&commit) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid commit hash: {}", e)),
                },
            ));
        }
    };

    match forge_node
        .cobs
        .approve_patch(&repo_id, &patch_hash, commit_hash, message)
        .await
    {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_merge_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    merge_commit: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid patch ID: {}", e)),
                },
            ));
        }
    };

    let merge_hash = match blake3::Hash::from_hex(&merge_commit) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid merge commit hash: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.merge_patch(&repo_id, &patch_hash, merge_hash).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_close_patch(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    patch_id: String,
    reason: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeOperationResultResponse;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let patch_hash = match blake3::Hash::from_hex(&patch_id) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeOperationResult(
                ForgeOperationResultResponse {
                    success: false,
                    error: Some(format!("Invalid patch ID: {}", e)),
                },
            ));
        }
    };

    match forge_node.cobs.close_patch(&repo_id, &patch_hash, reason).await {
        Ok(_) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: true,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeOperationResult(
            ForgeOperationResultResponse {
                success: false,
                error: Some(e.to_string()),
            },
        )),
    }
}

// ============================================================================
// Delegate Key
// ============================================================================

async fn handle_get_delegate_key(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeKeyResultResponse;
    use crate::forge::identity::RepoId;

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

async fn handle_get_federation_status(
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::FederationStatusResponse;

    let enabled = forge_node.has_federation();
    let cluster_name = forge_node
        .cluster_identity()
        .map(|i| i.name().to_string())
        .unwrap_or_default();
    let cluster_key = forge_node
        .cluster_identity()
        .map(|i| i.public_key().to_string())
        .unwrap_or_default();

    Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
        enabled,
        cluster_name,
        cluster_key,
        dht_enabled: false,
        gossip_enabled: forge_node.has_gossip(),
        discovered_clusters: 0,
        federated_repos: 0,
        error: None,
    }))
}

async fn handle_list_discovered_clusters(
    _ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::DiscoveredClustersResponse;

    // Federation discovery is not currently exposed through ClientProtocolContext
    // This feature requires direct integration with the federation discovery service
    Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
        clusters: vec![],
        count: 0,
        error: Some("Federation discovery not available through RPC".to_string()),
    }))
}

async fn handle_get_discovered_cluster(
    _ctx: &ClientProtocolContext,
    _cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::DiscoveredClusterResponse;

    // Federation discovery is not currently exposed through ClientProtocolContext
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

async fn handle_trust_cluster(
    _ctx: &ClientProtocolContext,
    _cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::TrustClusterResultResponse;

    // Trust manager is not currently exposed through ClientProtocolContext
    Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
        success: false,
        error: Some("Trust management not available through RPC".to_string()),
    }))
}

async fn handle_untrust_cluster(
    _ctx: &ClientProtocolContext,
    _cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::UntrustClusterResultResponse;

    // Trust manager is not currently exposed through ClientProtocolContext
    Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
        success: false,
        error: Some("Trust management not available through RPC".to_string()),
    }))
}

async fn handle_federate_repository(
    forge_node: &ForgeNodeRef,
    repo_id: String,
    mode: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::FederateRepositoryResultResponse;
    use crate::cluster::federation::FederationMode;
    use crate::forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::FederateRepositoryResult(
                FederateRepositoryResultResponse {
                    success: false,
                    fed_id: None,
                    error: Some(format!("Invalid repo ID: {}", e)),
                },
            ));
        }
    };

    let federation_mode = match mode.to_lowercase().as_str() {
        "public" => FederationMode::Public,
        "allowlist" => FederationMode::AllowList,
        _ => {
            return Ok(ClientRpcResponse::FederateRepositoryResult(
                FederateRepositoryResultResponse {
                    success: false,
                    fed_id: None,
                    error: Some(format!("Invalid federation mode: {}", mode)),
                },
            ));
        }
    };

    match forge_node.federate_repo(&repo_id, federation_mode, vec![]).await {
        Ok(fed_id) => Ok(ClientRpcResponse::FederateRepositoryResult(
            FederateRepositoryResultResponse {
                success: true,
                fed_id: Some(fed_id.to_string()),
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::FederateRepositoryResult(
            FederateRepositoryResultResponse {
                success: false,
                fed_id: None,
                error: Some(e.to_string()),
            },
        )),
    }
}

async fn handle_list_federated_repositories(
    _forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::FederatedRepositoriesResponse;

    // TODO: Implement listing federated repositories
    Ok(ClientRpcResponse::FederatedRepositories(
        FederatedRepositoriesResponse {
            repositories: vec![],
            count: 0,
            error: None,
        },
    ))
}

async fn handle_fetch_federated(
    forge_node: &ForgeNodeRef,
    federated_id: String,
    remote_cluster: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::ForgeFetchFederatedResultResponse;
    use crate::cluster::federation::FederatedId;

    let fed_id: FederatedId = match federated_id.parse() {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeFetchResult(
                ForgeFetchFederatedResultResponse {
                    success: false,
                    remote_cluster: None,
                    fetched: 0,
                    already_present: 0,
                    errors: vec![format!("Invalid federated ID: {}", e)],
                    error: Some(format!("Invalid federated ID: {}", e)),
                },
            ));
        }
    };

    let remote_key: iroh::PublicKey = match remote_cluster.parse() {
        Ok(k) => k,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeFetchResult(
                ForgeFetchFederatedResultResponse {
                    success: false,
                    remote_cluster: None,
                    fetched: 0,
                    already_present: 0,
                    errors: vec![format!("Invalid cluster key: {}", e)],
                    error: Some(format!("Invalid cluster key: {}", e)),
                },
            ));
        }
    };

    match forge_node.fetch_federated(&fed_id, remote_key).await {
        Ok(result) => Ok(ClientRpcResponse::ForgeFetchResult(
            ForgeFetchFederatedResultResponse {
                success: true,
                remote_cluster: Some(remote_cluster),
                fetched: result.fetched,
                already_present: result.already_present,
                errors: result.errors,
                error: None,
            },
        )),
        Err(e) => Ok(ClientRpcResponse::ForgeFetchResult(
            ForgeFetchFederatedResultResponse {
                success: false,
                remote_cluster: None,
                fetched: 0,
                already_present: 0,
                errors: vec![],
                error: Some(e.to_string()),
            },
        )),
    }
}

// ============================================================================
// Git Bridge Operations (requires git-bridge feature)
// ============================================================================

#[cfg(feature = "git-bridge")]
async fn handle_git_bridge_list_refs(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{GitBridgeListRefsResponse, GitBridgeRefInfo};
    use crate::forge::git::bridge::exporter::GitBridgeExporter;
    use crate::forge::git::bridge::mapping::HashMappingStore;
    use crate::forge::identity::RepoId;

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
    let mapping = HashMappingStore::new(forge_node.kv().clone());
    let exporter = GitBridgeExporter::new(forge_node.git.clone(), mapping);

    match exporter.list_refs(&repo_id, &forge_node.refs).await {
        Ok(ref_list) => {
            let refs: Vec<GitBridgeRefInfo> = ref_list
                .refs
                .iter()
                .map(|(name, sha1)| GitBridgeRefInfo {
                    ref_name: format!("refs/{}", name),
                    sha1: sha1.to_hex_string(),
                })
                .collect();

            Ok(ClientRpcResponse::GitBridgeListRefs(GitBridgeListRefsResponse {
                success: true,
                refs,
                head: ref_list.head.map(|h| format!("refs/{}", h)),
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
    use crate::client_rpc::{GitBridgeFetchResponse, GitBridgeObject};
    use crate::forge::git::bridge::exporter::GitBridgeExporter;
    use crate::forge::git::bridge::mapping::HashMappingStore;
    use crate::forge::git::bridge::sha1::Sha1Hash;
    use crate::forge::identity::RepoId;

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

    let have_hashes: Result<Vec<Sha1Hash>, _> = have.iter().map(|s| Sha1Hash::from_hex(s)).collect();
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

    // Create exporter
    let mapping = HashMappingStore::new(forge_node.kv().clone());
    let exporter = GitBridgeExporter::new(forge_node.git.clone(), mapping);

    // Export commits for all wanted refs
    let mut objects = Vec::new();
    let mut skipped = 0;

    for want_sha1 in &want_hashes {
        // Get the blake3 hash for this SHA-1
        if let Ok(Some(blake3_hash)) = exporter.get_sha1(&repo_id, want_sha1).await {
            match exporter.export_commit_dag(&repo_id, blake3_hash, &have_hashes).await {
                Ok(exported) => {
                    for obj in exported.objects {
                        objects.push(GitBridgeObject {
                            sha1: obj.sha1.to_hex_string(),
                            object_type: obj.object_type,
                            data: obj.data,
                        });
                    }
                    skipped += exported.skipped;
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
    objects: Vec<crate::client_rpc::GitBridgeObject>,
    refs: Vec<crate::client_rpc::GitBridgeRefUpdate>,
) -> anyhow::Result<ClientRpcResponse> {
    use crate::client_rpc::{GitBridgePushResponse, GitBridgeRefResult};
    use crate::forge::git::bridge::importer::GitBridgeImporter;
    use crate::forge::git::bridge::mapping::HashMappingStore;
    use crate::forge::git::bridge::sha1::Sha1Hash;
    use crate::forge::identity::RepoId;

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
    let mapping = HashMappingStore::new(forge_node.kv().clone());
    let importer = GitBridgeImporter::new(forge_node.git.clone(), mapping);

    // Import objects
    let mut objects_imported = 0;
    let mut objects_skipped = 0;

    for obj in &objects {
        let sha1 = match Sha1Hash::from_hex(&obj.sha1) {
            Ok(h) => h,
            Err(_) => continue,
        };

        match importer
            .import_object_raw(&repo_id, sha1, &obj.object_type, obj.data.clone())
            .await
        {
            Ok(imported) => {
                if imported {
                    objects_imported += 1;
                } else {
                    objects_skipped += 1;
                }
            }
            Err(_) => objects_skipped += 1,
        }
    }

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
        let ref_name = ref_update
            .ref_name
            .strip_prefix("refs/")
            .unwrap_or(&ref_update.ref_name);

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
