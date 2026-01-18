//! Pijul (patch-based VCS) request handler.
//!
//! Handles all Pijul* operations for patch-based version control.
//! This module is only available with the `pijul` feature.
//!
//! ## Operations (13 total)
//!
//! ### Repository Operations (3)
//! - PijulRepoInit: Create a new Pijul repository
//! - PijulRepoList: List all repositories
//! - PijulRepoInfo: Get repository details
//!
//! ### Channel Operations (5)
//! - PijulChannelList: List channels in a repository
//! - PijulChannelCreate: Create a new channel
//! - PijulChannelDelete: Delete a channel
//! - PijulChannelFork: Fork a channel
//! - PijulChannelInfo: Get channel details
//!
//! ### Change Operations (5)
//! - PijulRecord: Record changes (requires local filesystem - returns NOT_IMPLEMENTED)
//! - PijulApply: Apply a change to a channel
//! - PijulUnrecord: Remove a change from a channel
//! - PijulLog: Get change log for a channel
//! - PijulCheckout: Checkout to working directory (requires local filesystem - returns
//!   NOT_IMPLEMENTED)

use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;

use crate::context::ClientProtocolContext;

/// Get current Unix timestamp in milliseconds (Tiger Style: safe fallback to 0).
#[inline]
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
use crate::registry::RequestHandler;

/// Type alias for the PijulStore with concrete types.
#[cfg(feature = "pijul")]
type PijulStoreRef = std::sync::Arc<aspen_pijul::PijulStore<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;

/// Handler for Pijul operations.
pub struct PijulHandler;

#[async_trait::async_trait]
impl RequestHandler for PijulHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        #[cfg(feature = "pijul")]
        {
            matches!(
                request,
                ClientRpcRequest::PijulRepoInit { .. }
                    | ClientRpcRequest::PijulRepoList { .. }
                    | ClientRpcRequest::PijulRepoInfo { .. }
                    | ClientRpcRequest::PijulChannelList { .. }
                    | ClientRpcRequest::PijulChannelCreate { .. }
                    | ClientRpcRequest::PijulChannelDelete { .. }
                    | ClientRpcRequest::PijulChannelFork { .. }
                    | ClientRpcRequest::PijulChannelInfo { .. }
                    | ClientRpcRequest::PijulRecord { .. }
                    | ClientRpcRequest::PijulApply { .. }
                    | ClientRpcRequest::PijulUnrecord { .. }
                    | ClientRpcRequest::PijulLog { .. }
                    | ClientRpcRequest::PijulCheckout { .. }
                    | ClientRpcRequest::PijulShow { .. }
                    | ClientRpcRequest::PijulBlame { .. }
            )
        }
        #[cfg(not(feature = "pijul"))]
        {
            // Without pijul feature, these variants don't exist
            let _ = request;
            false
        }
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        #[cfg(feature = "pijul")]
        {
            // Check if Pijul feature is available
            let pijul_store = match &ctx.pijul_store {
                Some(store) => store,
                None => {
                    return Ok(ClientRpcResponse::error(
                        "PIJUL_UNAVAILABLE",
                        "Pijul feature not configured on this node",
                    ));
                }
            };

            match request {
                // ================================================================
                // Repository Operations
                // ================================================================
                ClientRpcRequest::PijulRepoInit {
                    name,
                    description,
                    default_channel,
                } => handle_repo_init(pijul_store, name, description, default_channel).await,

                ClientRpcRequest::PijulRepoList { limit } => handle_repo_list(pijul_store, limit).await,

                ClientRpcRequest::PijulRepoInfo { repo_id } => handle_repo_info(pijul_store, repo_id).await,

                // ================================================================
                // Channel Operations
                // ================================================================
                ClientRpcRequest::PijulChannelList { repo_id } => handle_channel_list(pijul_store, repo_id).await,

                ClientRpcRequest::PijulChannelCreate { repo_id, name } => {
                    handle_channel_create(pijul_store, repo_id, name).await
                }

                ClientRpcRequest::PijulChannelDelete { repo_id, name } => {
                    handle_channel_delete(pijul_store, repo_id, name).await
                }

                ClientRpcRequest::PijulChannelFork {
                    repo_id,
                    source,
                    target,
                } => handle_channel_fork(pijul_store, repo_id, source, target).await,

                ClientRpcRequest::PijulChannelInfo { repo_id, name } => {
                    handle_channel_info(pijul_store, repo_id, name).await
                }

                // ================================================================
                // Change Operations
                // ================================================================
                ClientRpcRequest::PijulRecord { .. } => {
                    // Recording requires local filesystem access - complex to implement remotely
                    Ok(ClientRpcResponse::error(
                        "NOT_IMPLEMENTED",
                        "Recording changes requires local filesystem access. Use local pijul tools.",
                    ))
                }

                ClientRpcRequest::PijulApply {
                    repo_id,
                    channel,
                    change_hash,
                } => handle_apply(pijul_store, repo_id, channel, change_hash).await,

                ClientRpcRequest::PijulUnrecord {
                    repo_id,
                    channel,
                    change_hash,
                } => handle_unrecord(pijul_store, repo_id, channel, change_hash).await,

                ClientRpcRequest::PijulLog {
                    repo_id,
                    channel,
                    limit,
                } => handle_log(pijul_store, repo_id, channel, limit).await,

                ClientRpcRequest::PijulCheckout { .. } => {
                    // Checkout requires local filesystem access - complex to implement remotely
                    Ok(ClientRpcResponse::error(
                        "NOT_IMPLEMENTED",
                        "Checkout requires local filesystem access. Use local pijul tools.",
                    ))
                }

                ClientRpcRequest::PijulShow { repo_id, change_hash } => {
                    handle_show(pijul_store, repo_id, change_hash).await
                }

                ClientRpcRequest::PijulBlame { repo_id, channel, path } => {
                    handle_blame(pijul_store, repo_id, channel, path).await
                }

                _ => Err(anyhow::anyhow!("request not handled by PijulHandler")),
            }
        }

        #[cfg(not(feature = "pijul"))]
        {
            let _ = (request, ctx);
            Ok(ClientRpcResponse::error(
                "PIJUL_UNAVAILABLE",
                "Pijul feature not compiled. Rebuild with --features pijul",
            ))
        }
    }

    fn name(&self) -> &'static str {
        "PijulHandler"
    }
}

// ============================================================================
// Repository Operations
// ============================================================================

#[cfg(feature = "pijul")]
async fn handle_repo_init(
    pijul_store: &PijulStoreRef,
    name: String,
    description: Option<String>,
    default_channel: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulRepoResponse;
    use aspen_pijul::types::PijulRepoIdentity;

    // Create identity with no delegates (self-signed)
    let mut identity = PijulRepoIdentity::new(&name, vec![]).with_default_channel(&default_channel);

    if let Some(desc) = description {
        identity = identity.with_description(desc);
    }

    match pijul_store.create_repo(identity.clone()).await {
        Ok(repo_id) => {
            // Count channels (should be 1 for the default channel)
            let channel_count = pijul_store.list_channels(&repo_id).await.map(|c| c.len() as u32).unwrap_or(1);

            Ok(ClientRpcResponse::PijulRepoResult(PijulRepoResponse {
                id: repo_id.to_hex(),
                name: identity.name,
                description: identity.description,
                default_channel: identity.default_channel,
                channel_count,
                created_at_ms: identity.created_at_ms,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_CREATE_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_repo_list(pijul_store: &PijulStoreRef, limit: u32) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulRepoListResponse;
    use aspen_client_rpc::PijulRepoResponse;

    // Tiger Style: cap limit to 1000
    let limit = limit.min(1000);

    match pijul_store.list_repos(limit).await {
        Ok(repos) => {
            let count = repos.len() as u32;
            let mut response_repos = Vec::with_capacity(repos.len());

            for (repo_id, identity) in repos {
                let channel_count = pijul_store.list_channels(&repo_id).await.map(|c| c.len() as u32).unwrap_or(0);

                response_repos.push(PijulRepoResponse {
                    id: repo_id.to_hex(),
                    name: identity.name,
                    description: identity.description,
                    default_channel: identity.default_channel,
                    channel_count,
                    created_at_ms: identity.created_at_ms,
                });
            }

            Ok(ClientRpcResponse::PijulRepoListResult(PijulRepoListResponse {
                repos: response_repos,
                count,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_LIST_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_repo_info(pijul_store: &PijulStoreRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulRepoResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.get_repo(&repo_id).await {
        Ok(Some(identity)) => {
            let channel_count = pijul_store.list_channels(&repo_id).await.map(|c| c.len() as u32).unwrap_or(0);

            Ok(ClientRpcResponse::PijulRepoResult(PijulRepoResponse {
                id: repo_id.to_hex(),
                name: identity.name,
                description: identity.description,
                default_channel: identity.default_channel,
                channel_count,
                created_at_ms: identity.created_at_ms,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_NOT_FOUND".to_string(),
            message: format!("Repository not found: {}", repo_id),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "REPO_INFO_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

// ============================================================================
// Channel Operations
// ============================================================================

#[cfg(feature = "pijul")]
async fn handle_channel_list(pijul_store: &PijulStoreRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulChannelListResponse;
    use aspen_client_rpc::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.list_channels(&repo_id).await {
        Ok(channels) => {
            let count = channels.len() as u32;
            let response_channels: Vec<_> = channels
                .into_iter()
                .map(|ch| PijulChannelResponse {
                    name: ch.name,
                    head: ch.head.map(|h| h.to_hex()),
                    updated_at_ms: ch.updated_at_ms,
                })
                .collect();

            Ok(ClientRpcResponse::PijulChannelListResult(PijulChannelListResponse {
                channels: response_channels,
                count,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_LIST_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_channel_create(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.create_channel(&repo_id, &name).await {
        Ok(()) => Ok(ClientRpcResponse::PijulChannelResult(PijulChannelResponse {
            name,
            head: None,
            updated_at_ms: current_time_ms(),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_CREATE_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_channel_delete(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.delete_channel(&repo_id, &name).await {
        Ok(()) => Ok(ClientRpcResponse::PijulSuccess),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_DELETE_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_channel_fork(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    source: String,
    target: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.fork_channel(&repo_id, &source, &target).await {
        Ok(channel) => Ok(ClientRpcResponse::PijulChannelResult(PijulChannelResponse {
            name: channel.name,
            head: channel.head.map(|h| h.to_hex()),
            updated_at_ms: channel.updated_at_ms,
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_FORK_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_channel_info(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulChannelResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    match pijul_store.get_channel(&repo_id, &name).await {
        Ok(Some(channel)) => Ok(ClientRpcResponse::PijulChannelResult(PijulChannelResponse {
            name: channel.name,
            head: channel.head.map(|h| h.to_hex()),
            updated_at_ms: channel.updated_at_ms,
        })),
        Ok(None) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_NOT_FOUND".to_string(),
            message: format!("Channel '{}' not found", name),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANNEL_INFO_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

// ============================================================================
// Change Operations
// ============================================================================

#[cfg(feature = "pijul")]
async fn handle_apply(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    change_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulApplyResponse;
    use aspen_forge::identity::RepoId;
    use aspen_pijul::types::ChangeHash;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    let hash = match ChangeHash::from_hex(&change_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_CHANGE_HASH".to_string(),
                message: format!("Invalid change hash: {}", e),
            }));
        }
    };

    match pijul_store.apply_change(&repo_id, &channel, &hash).await {
        Ok(result) => Ok(ClientRpcResponse::PijulApplyResult(PijulApplyResponse {
            operations: result.changes_applied,
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "APPLY_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_unrecord(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    change_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulUnrecordResponse;
    use aspen_forge::identity::RepoId;
    use aspen_pijul::types::ChangeHash;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    let hash = match ChangeHash::from_hex(&change_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_CHANGE_HASH".to_string(),
                message: format!("Invalid change hash: {}", e),
            }));
        }
    };

    match pijul_store.unrecord_change(&repo_id, &channel, &hash).await {
        Ok(unrecorded) => Ok(ClientRpcResponse::PijulUnrecordResult(PijulUnrecordResponse { unrecorded })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "UNRECORD_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_log(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    limit: u32,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulLogEntry;
    use aspen_client_rpc::PijulLogResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    // Tiger Style: cap limit to 1000
    let limit = limit.min(1000);

    match pijul_store.get_change_log(&repo_id, &channel, limit).await {
        Ok(metadata_list) => {
            let entries: Vec<PijulLogEntry> = metadata_list
                .into_iter()
                .map(|m| PijulLogEntry {
                    change_hash: m.hash.to_hex(),
                    message: m.message,
                    author: m.authors.first().map(|a| {
                        if let Some(email) = &a.email {
                            format!("{} <{}>", a.name, email)
                        } else {
                            a.name.clone()
                        }
                    }),
                    timestamp_ms: m.recorded_at_ms,
                })
                .collect();

            let count = entries.len() as u32;
            Ok(ClientRpcResponse::PijulLogResult(PijulLogResponse { entries, count }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "LOG_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_show(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    change_hash: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulAuthorInfo;
    use aspen_client_rpc::PijulShowResponse;
    use aspen_forge::identity::RepoId;
    use aspen_pijul::types::ChangeHash;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    let hash = match ChangeHash::from_hex(&change_hash) {
        Ok(h) => h,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_CHANGE_HASH".to_string(),
                message: format!("Invalid change hash: {}", e),
            }));
        }
    };

    match pijul_store.get_change_metadata(&repo_id, &hash).await {
        Ok(Some(metadata)) => Ok(ClientRpcResponse::PijulShowResult(PijulShowResponse {
            change_hash: metadata.hash.to_hex(),
            repo_id: metadata.repo_id.to_hex(),
            channel: metadata.channel,
            message: metadata.message,
            authors: metadata
                .authors
                .into_iter()
                .map(|a| PijulAuthorInfo {
                    name: a.name,
                    email: a.email,
                })
                .collect(),
            dependencies: metadata.dependencies.into_iter().map(|d| d.to_hex()).collect(),
            size_bytes: metadata.size_bytes,
            recorded_at_ms: metadata.recorded_at_ms,
        })),
        Ok(None) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "CHANGE_NOT_FOUND".to_string(),
            message: format!("Change '{}' not found", change_hash),
        })),
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "SHOW_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}

#[cfg(feature = "pijul")]
async fn handle_blame(
    pijul_store: &PijulStoreRef,
    repo_id: String,
    channel: String,
    path: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_rpc::ErrorResponse;
    use aspen_client_rpc::PijulBlameEntry;
    use aspen_client_rpc::PijulBlameResponse;
    use aspen_forge::identity::RepoId;

    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::Error(ErrorResponse {
                code: "INVALID_REPO_ID".to_string(),
                message: format!("Invalid repo ID: {}", e),
            }));
        }
    };

    // Get blame information for the file
    // Currently returns change-level attribution. Per-line blame requires
    // additional libpijul graph traversal.
    match pijul_store.blame_file(&repo_id_parsed, &channel, &path).await {
        Ok(result) => {
            let attributions = result
                .attributions
                .into_iter()
                .map(|attr| PijulBlameEntry {
                    change_hash: attr.change_hash.to_hex(),
                    author: attr.author,
                    author_email: attr.author_email,
                    message: attr.message,
                    recorded_at_ms: attr.recorded_at_ms,
                    change_type: attr.change_type,
                })
                .collect();

            Ok(ClientRpcResponse::PijulBlameResult(PijulBlameResponse {
                path: result.path,
                channel: result.channel,
                repo_id,
                attributions,
                file_exists: result.file_exists,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::Error(ErrorResponse {
            code: "BLAME_FAILED".to_string(),
            message: format!("{}", e),
        })),
    }
}
