//! Repository CRUD operations.

use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::ForgeNodeRef;

pub(crate) async fn handle_create_repo(
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
                is_success: true,
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
            is_success: false,
            repo: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_get_repo(forge_node: &ForgeNodeRef, repo_id: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeRepoInfo;
    use aspen_client_api::ForgeRepoResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
                is_success: false,
                repo: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    match forge_node.get_repo(&repo_id).await {
        Ok(identity) => Ok(ClientRpcResponse::ForgeRepoResult(ForgeRepoResultResponse {
            is_success: true,
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
            is_success: false,
            repo: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_list_repos(
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
            error: Some(e.to_string()),
        })),
    }
}
